use crate::ast::*;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CLParser;

pub fn parse_file(source: &str) -> Result<Program, pest::error::Error<Rule>> {
    let mut commands = Vec::new();

    let mut parsed = CLParser::parse(Rule::file, source)?;
    let file_node = parsed.next().unwrap();

    for command_node in file_node.into_inner() {
        if command_node.as_rule() != Rule::command {
            continue;
        }

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
