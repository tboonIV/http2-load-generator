use crate::function;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
    pub function: Option<function::Function>,
}

impl Variable {
    // pub fn set_value(&mut self, value: Value) {
    //     self.value = value;
    // }

    #[allow(dead_code)]
    pub fn apply(&mut self) {
        if let Some(function) = &self.function {
            let value = match function {
                function::Function::Increment(f) => {
                    let value = match self.value {
                        Value::Int(v) => v,
                        Value::String(ref v) => v.parse::<i32>().unwrap(),
                    };
                    let value = f.apply(value);
                    Value::Int(value)
                }
                function::Function::Random(f) => {
                    let value = f.apply();
                    Value::Int(value)
                }
                function::Function::Split(f) => {
                    let value = match self.value {
                        Value::Int(v) => v.to_string(),
                        Value::String(ref v) => v.to_string(),
                    };
                    let value = f.apply(value);
                    Value::String(value)
                }
            };

            self.value = value;
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(i32),
}
