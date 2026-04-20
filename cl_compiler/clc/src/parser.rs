use crate::ast::*;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CLParser;

pub fn parse_file(source: &str) -> Result<Program, pest::error::Error<Rule>> {
    let mut commands = Vec::new();
    let source = strip_block_comments(source);

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parsed = CLParser::parse(Rule::command, line)?;
        let command_node = parsed.next().unwrap();

        let mut inner = command_node.into_inner();
        let name = inner.next().unwrap().as_str().to_uppercase();
        let mut parameters = Vec::new();

        if let Some(params_node) = inner.next() {
            for param_node in params_node.into_inner() {
                let mut p_inner = param_node.clone().into_inner();
                let first = p_inner.next().unwrap();

                let param = if first.as_rule() == Rule::identifier && p_inner.peek().is_some() {
                    // Named parameter: KWD(VAL) or KWD(VAL1 VAL2)
                    let key = first.as_str().to_uppercase();
                    // value_list node — take first value for simplicity (primary param)
                    let vlist = p_inner.next().unwrap();
                    let val_node = vlist.into_inner().next().unwrap();
                    Parameter::Named(key, parse_value(val_node))
                } else if first.as_rule() == Rule::value {
                    Parameter::Positional(parse_value(first.into_inner().next().unwrap()))
                } else {
                    Parameter::Positional(parse_value(first))
                };
                parameters.push(param);
            }
        }
        commands.push(Command { name, parameters });
    }
    Ok(Program { commands })
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

fn parse_value(node: pest::iterators::Pair<Rule>) -> Value {
    match node.as_rule() {
        Rule::string_literal => {
            let s = node.as_str();
            Value::StringLiteral(s[1..s.len() - 1].to_string()) // Quitar comillas
        }
        Rule::keyword => Value::Keyword(node.as_str().to_uppercase()),
        Rule::identifier => Value::Identifier(node.as_str().to_uppercase()),
        Rule::value => parse_value(node.into_inner().next().unwrap()),
        _ => unreachable!(),
    }
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
}
