#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub commands: Vec<Command>,
    pub statements: Vec<Statement>,
    pub parameters: Vec<String>,
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
    Variable(String),
    List(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Command(Command),
    If {
        condition: Condition,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
    },
    MonMsg {
        msgid: String,
        exec: Option<Command>,
    },
    While {
        condition: Condition,
        body: Vec<Statement>,
        until: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub left: Value,
    pub operator: String,
    pub right: Value,
}
