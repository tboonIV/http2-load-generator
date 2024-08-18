use crate::function;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
    // TODO maybe remove function from variable
    pub function: Option<function::Function>,
}

impl Variable {
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
                function::Function::Now(f) => {
                    let value = f.apply(None);
                    Value::String(value)
                }
                function::Function::Plus(_f) => {
                    todo!()
                }
            };

            self.value = value;
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(i32),
    // TODO Support float
}

impl Value {
    pub fn as_string(&self) -> String {
        match self {
            Value::String(ref v) => v.clone(),
            Value::Int(v) => v.to_string(),
        }
    }

    pub fn as_int(&self) -> i32 {
        match self {
            Value::String(ref v) => v.parse::<i32>().unwrap(),
            Value::Int(v) => *v,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Variable2 {
    Variable(Variable),
    Constant(Value),
}
