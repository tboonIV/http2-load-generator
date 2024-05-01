// TODO REMOVE ME
#![allow(dead_code)]
// use serde::Deserialize;
use std::error::Error;

// Future Features:
// Support simple scripting language similiar to Karate
//
// def location = responseHeaders.location[0]
// def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
//
// def count = 0
// def count = count + 1
//
#[derive(Debug)]
pub struct Scripting {
    pub raw: String,
    pub variables: Vec<Variable>,
}

impl Scripting {
    pub fn new(raw: &str) -> Self {
        Scripting {
            raw: raw.into(),
            variables: vec![],
        }
    }

    pub fn eval(&mut self) -> Result<(), Box<dyn Error>> {
        let lines: Vec<&str> = self.raw.split('\n').collect();
        for line in lines {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() > 0 {
                let def = parts[0];
                if def == "def" {
                    if parts.len() != 4 {
                        return Err("Invalid script".into());
                    }
                    let name = parts[1];
                    let equal = parts[2];
                    if equal != "=" {
                        return Err("Invalid script".into());
                    }
                    let value = parts[3];
                    let variable = Variable {
                        name: name.into(),
                        value: Value::Int(value.parse().unwrap()),
                    };
                    self.variables.push(variable);
                } else {
                    return Err("Invalid script".into());
                }
            }
        }
        Ok(())
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        for variable in &self.variables {
            if variable.name == name {
                return Some(variable.value.clone());
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i32),
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_scripting() {
        let mut scripting = Scripting::new("def count = 10");
        scripting.eval().unwrap();
        let count = scripting.get_variable("count").unwrap();
        assert_eq!(count, Value::Int(10));
    }
}
