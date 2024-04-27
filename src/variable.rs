use crate::function;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(i32),
}
