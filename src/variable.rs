use crate::function;

#[derive(Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
    pub function: Option<function::Function>,
}

impl Variable {
    pub fn set_value(&mut self, value: Value) {
        self.value = value;
    }
}

// TODO remove duplicate with config::Value
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    String(String),
    Int(i32),
}
