#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Positional(Value),
    Named(String, Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    StringLiteral(String),
    Keyword(String),
    Identifier(String),
}
