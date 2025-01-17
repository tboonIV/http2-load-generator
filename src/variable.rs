use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
}

// TODO remove?
// impl Variable {
//     pub fn update_value(&mut self, value: Value) {
//         self.value = value;
//     }
// }

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

    pub fn is_string(&self) -> bool {
        match self {
            Value::String(_) => true,
            Value::Int(_) => false,
        }
    }

    // pub fn is_int(&self) -> bool {
    //     match self {
    //         Value::String(_) => false,
    //         Value::Int(_) => true,
    //     }
    // }
}
