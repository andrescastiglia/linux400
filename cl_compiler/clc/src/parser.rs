use crate::ast::*;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CLParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
    cpf: &'static str,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            cpf: "CPF0006",
        }
    }

    fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn cpf(&self) -> &'static str {
        self.cpf
    }

    pub fn line(&self) -> Option<usize> {
        self.line
    }

    pub fn column(&self) -> Option<usize> {
        self.column
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                write!(
                    f,
                    "{} line={} column={}: {}",
                    self.cpf, line, column, self.message
                )
            }
            _ => write!(f, "{}: {}", self.cpf, self.message),
        }
    }
}

impl std::error::Error for ParseError {}

enum Frame {
    Root(Vec<Statement>),
    If {
        condition: Condition,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
        in_else: bool,
    },
    Do(Vec<Statement>),
    While {
        condition: Condition,
        body: Vec<Statement>,
        until: bool,
    },
}

impl Frame {
    fn push(&mut self, statement: Statement) {
        match self {
            Frame::Root(statements) | Frame::Do(statements) => statements.push(statement),
            Frame::While { body, .. } => body.push(statement),
            Frame::If {
                then_branch,
                else_branch,
                in_else,
                ..
            } => {
                if *in_else {
                    else_branch.push(statement);
                } else {
                    then_branch.push(statement);
                }
            }
        }
    }
}

pub fn parse_file(source: &str) -> Result<Program, ParseError> {
    let source = strip_block_comments(source);
    let mut frames = vec![Frame::Root(Vec::new())];
    let mut commands = Vec::new();
    let mut parameters = Vec::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let upper = line.to_uppercase();
        if upper == "ELSE" || upper == "ELSE DO" || upper == "ELSEDO" {
            match frames.last_mut() {
                Some(Frame::If { in_else, .. }) => *in_else = true,
                _ => {
                    return Err(
                        ParseError::new("ELSE without matching IF").with_location(line_number, 1)
                    );
                }
            }
            continue;
        }
        if upper == "ENDDO" {
            let closed = frames.pop().ok_or_else(|| {
                ParseError::new("ENDDO without open block").with_location(line_number, 1)
            })?;
            let statement = match closed {
                Frame::If {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                },
                Frame::While {
                    condition,
                    body,
                    until,
                } => Statement::While {
                    condition,
                    body,
                    until,
                },
                Frame::Do(statements) => {
                    for statement in statements {
                        frames
                            .last_mut()
                            .ok_or_else(|| {
                                ParseError::new("internal parser stack underflow")
                                    .with_location(line_number, 1)
                            })?
                            .push(statement);
                    }
                    continue;
                }
                Frame::Root(_) => {
                    return Err(
                        ParseError::new("ENDDO closes root block").with_location(line_number, 1)
                    );
                }
            };
            frames
                .last_mut()
                .ok_or_else(|| {
                    ParseError::new("internal parser stack underflow").with_location(line_number, 1)
                })?
                .push(statement);
            continue;
        }

        let command =
            parse_command_line(line).map_err(|error| error.with_location(line_number, 1))?;
        if command.name == "IF" {
            let condition =
                parse_condition(&command).map_err(|error| error.with_location(line_number, 1))?;
            if command_uses_do(&command, "THEN") {
                frames.push(Frame::If {
                    condition,
                    then_branch: Vec::new(),
                    else_branch: Vec::new(),
                    in_else: false,
                });
            } else if let Some(inline) = named_param(&command, "THEN")
                .map(command_from_value)
                .transpose()
                .map_err(|error| error.with_location(line_number, 1))?
                .flatten()
            {
                frames.last_mut().unwrap().push(Statement::If {
                    condition,
                    then_branch: vec![Statement::Command(inline.clone())],
                    else_branch: Vec::new(),
                });
                commands.push(inline);
            } else {
                return Err(ParseError::new("IF requires THEN(DO) or THEN(command)")
                    .with_location(line_number, 1));
            }
            commands.push(command);
            continue;
        }
        if command.name == "DO" {
            frames.push(Frame::Do(Vec::new()));
            commands.push(command);
            continue;
        }
        if command.name == "DOWHILE" || command.name == "DOUNTIL" {
            let condition =
                parse_condition(&command).map_err(|error| error.with_location(line_number, 1))?;
            frames.push(Frame::While {
                condition,
                body: Vec::new(),
                until: command.name == "DOUNTIL",
            });
            commands.push(command);
            continue;
        }

        if command.name == "MONMSG" {
            let msgid = named_param(&command, "MSGID")
                .map(value_to_plain)
                .or_else(|| first_positional(&command))
                .unwrap_or_else(|| "CPF0000".to_string());
            let exec = named_param(&command, "EXEC")
                .map(command_from_value)
                .transpose()
                .map_err(|error| error.with_location(line_number, 1))?
                .flatten();
            frames.last_mut().unwrap().push(Statement::MonMsg {
                msgid,
                exec: exec.clone(),
            });
            if let Some(exec) = exec {
                commands.push(exec);
            }
            commands.push(command);
            continue;
        }

        if command.name == "PGM" {
            parameters = named_param(&command, "PARM")
                .map(value_to_list)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| match value {
                    Value::Variable(name) => Some(name),
                    _ => None,
                })
                .collect();
        }
        frames
            .last_mut()
            .unwrap()
            .push(Statement::Command(command.clone()));
        commands.push(command);
    }

    if frames.len() != 1 {
        return Err(ParseError::new("unclosed DO/IF block"));
    }
    let statements = match frames.pop().unwrap() {
        Frame::Root(statements) => statements,
        _ => unreachable!(),
    };
    Ok(Program {
        commands,
        statements,
        parameters,
    })
}

fn parse_command_line(line: &str) -> Result<Command, ParseError> {
    let (name, rest) =
        split_first_token(line).ok_or_else(|| ParseError::new("expected CL command name"))?;
    let mut parameters = Vec::new();
    let mut rest = rest.trim();
    while !rest.is_empty() {
        let (token, remaining) = take_parameter(rest)?;
        parameters.push(parse_parameter(&token)?);
        rest = remaining.trim_start();
    }
    Ok(Command {
        name: name.to_uppercase(),
        parameters,
    })
}

fn split_first_token(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Some((&input[..end], &input[end..]))
}

fn take_parameter(input: &str) -> Result<(String, &str), ParseError> {
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => depth += 1,
            ')' if !in_single && !in_double => depth = depth.saturating_sub(1),
            ch if ch.is_whitespace() && !in_single && !in_double && depth == 0 => {
                return Ok((input[..index].to_string(), &input[index..]));
            }
            _ => {}
        }
    }
    if depth != 0 || in_single || in_double {
        return Err(ParseError::new("unterminated CL parameter"));
    }
    Ok((input.to_string(), ""))
}

fn parse_parameter(token: &str) -> Result<Parameter, ParseError> {
    if let Some(open) = token.find('(')
        && token.ends_with(')')
    {
        let key = token[..open].trim().to_uppercase();
        let raw_values = &token[open + 1..token.len() - 1];
        let values = split_values(raw_values)
            .into_iter()
            .map(|value| parse_value_token(&value))
            .collect::<Vec<_>>();
        let value = if values.len() == 1 {
            values.into_iter().next().unwrap()
        } else {
            Value::List(values)
        };
        return Ok(Parameter::Named(key, value));
    }
    Ok(Parameter::Positional(parse_value_token(token)))
}

fn split_values(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0usize;
    for ch in input.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch.is_whitespace() && !in_single && !in_double && depth == 0 => {
                if !current.trim().is_empty() {
                    values.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_string());
    }
    values
}

fn parse_value_token(token: &str) -> Value {
    let token = token.trim();
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 2 {
        Value::StringLiteral(token[1..token.len() - 1].to_string())
    } else if token.starts_with('&') {
        Value::Variable(token.trim_start_matches('&').to_uppercase())
    } else if token.starts_with('*') {
        Value::Keyword(token.to_uppercase())
    } else {
        Value::Identifier(token.to_uppercase())
    }
}

fn named_param<'a>(command: &'a Command, key: &str) -> Option<&'a Value> {
    command
        .parameters
        .iter()
        .find_map(|parameter| match parameter {
            Parameter::Named(candidate, value) if candidate.eq_ignore_ascii_case(key) => {
                Some(value)
            }
            _ => None,
        })
}

fn first_positional(command: &Command) -> Option<String> {
    command
        .parameters
        .iter()
        .find_map(|parameter| match parameter {
            Parameter::Positional(value) => Some(value_to_plain(value)),
            _ => None,
        })
}

fn value_to_plain(value: &Value) -> String {
    match value {
        Value::StringLiteral(value)
        | Value::Keyword(value)
        | Value::Identifier(value)
        | Value::Variable(value) => value.clone(),
        Value::List(values) => values
            .iter()
            .map(value_to_plain)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn value_to_list(value: &Value) -> Vec<Value> {
    match value {
        Value::List(values) => values.clone(),
        value => vec![value.clone()],
    }
}

fn parse_condition(command: &Command) -> Result<Condition, ParseError> {
    let values = named_param(command, "COND")
        .map(value_to_list)
        .ok_or_else(|| ParseError::new("IF requires COND(...)"))?;
    if values.len() != 3 {
        return Err(ParseError::new("COND requires: left operator right"));
    }
    Ok(Condition {
        left: values[0].clone(),
        operator: value_to_plain(&values[1]).to_uppercase(),
        right: values[2].clone(),
    })
}

fn command_uses_do(command: &Command, key: &str) -> bool {
    named_param(command, key)
        .map(value_to_plain)
        .is_some_and(|value| value.eq_ignore_ascii_case("DO"))
}

fn command_from_value(value: &Value) -> Result<Option<Command>, ParseError> {
    let value = value_to_plain(value);
    if value.eq_ignore_ascii_case("DO") || value.trim().is_empty() {
        Ok(None)
    } else {
        parse_command_line(&value).map(Some)
    }
}

fn strip_block_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    let mut in_comment = false;

    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        if !in_comment && current == '/' && next == Some('*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if in_comment && current == '*' && next == Some('/') {
            in_comment = false;
            index += 2;
            continue;
        }

        if !in_comment {
            result.push(current);
        }
        index += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_ignores_block_comments() {
        let program = parse_file("/* comentario inicial */\nPGM\n/* comentario medio */\nENDPGM\n")
            .expect("parse_file falló");

        assert_eq!(program.commands.len(), 2);
        assert_eq!(program.commands[0].name, "PGM");
        assert_eq!(program.commands[1].name, "ENDPGM");
    }

    #[test]
    fn parse_control_language_blocks() {
        let program = parse_file(
            "PGM PARM(&LIB)\nDCL VAR(&LIB) TYPE(*CHAR) VALUE('QGPL')\nIF COND(&LIB *EQ 'QGPL') THEN(DO)\nSNDPGMMSG MSG('ok')\nELSE\nSNDPGMMSG MSG('bad')\nENDDO\nDOWHILE COND(&LIB *EQ 'QGPL')\nSNDPGMMSG MSG('loop')\nENDDO\nMONMSG MSGID(CPF0000) EXEC(SNDPGMMSG MSG('ignored'))\nENDPGM\n",
        )
        .expect("parse_file falló");

        assert_eq!(program.parameters, vec!["LIB".to_string()]);
        assert!(
            program
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::If { .. }))
        );
        assert!(
            program
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::MonMsg { .. }))
        );
        assert!(
            program
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::While { .. }))
        );
    }

    #[test]
    fn parse_errors_include_cpf_and_location() {
        let error = parse_file("PGM\nIF COND(&A *EQ) THEN(DO)\nENDPGM\n").unwrap_err();

        assert_eq!(error.cpf(), "CPF0006");
        assert_eq!(error.line(), Some(2));
        assert_eq!(error.column(), Some(1));
        assert!(error.to_string().contains("line=2 column=1"));
    }
}
