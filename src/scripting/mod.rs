// TODO REMOVE ME
#![allow(dead_code)]
// use serde::Deserialize;
use std::collections::HashMap;
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
//
#[derive(Debug)]
pub struct Context {
    pub variables: HashMap<String, Value>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            variables: HashMap::new(),
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }
}

#[derive(Debug)]
pub struct Scripting {
    pub raw: String,
}

impl Scripting {
    pub fn new(raw: &str) -> Self {
        Scripting { raw: raw.into() }
    }

    // Might be a bad idea to use split to parse the script
    pub fn eval(&mut self, context: &mut Context) -> Result<(), Box<dyn Error>> {
        let lines: Vec<&str> = self.raw.split('\n').collect();
        for line in lines {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() > 0 {
                let def = parts[0];
                if def == "def" {
                    // Validate
                    if parts.len() < 4 {
                        return Err(format!(
                            "invalid script, expected at least 4 parts, got {}",
                            parts.len()
                        )
                        .into());
                    }
                    // Process variable name
                    let name = parts[1];

                    // Process operator, only support '='
                    let equal = parts[2];
                    if equal != "=" {
                        return Err("Invalid script, must be '='".into());
                    }

                    // Process value
                    let value = parts[3];

                    // Check if value is constant or variable
                    let value = match value.parse::<i32>() {
                        Ok(v) => Value::Int(v),
                        Err(_) => {
                            // Check if variable exists
                            match context.get_variable(value) {
                                Some(v) => v.clone(),
                                None => {
                                    return Err(format!("variable '{}' not found", value).into())
                                }
                            }
                        }
                    };

                    // Check if there is an operator
                    let value = if parts.len() == 6 {
                        if parts[4] == "+" {
                            match value {
                                Value::Int(v) => {
                                    let v2 = parts[5].parse::<i32>()?;
                                    Value::Int(v + v2)
                                }
                                _ => return Err("invalid script, expected integer".into()),
                            }
                        } else {
                            return Err(
                                format!("invalid script, unknown operator '{}'", parts[4]).into()
                            );
                        }
                    } else {
                        value
                    };

                    context.variables.insert(name.into(), value);
                } else {
                    return Err(format!("invalid script, unknown command '{}'", def).into());
                }
            }
        }
        Ok(())
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
    fn test_scripting_basic() {
        let mut context = Context::new();
        let mut scripting = Scripting::new("def foo = 16");
        scripting.eval(&mut context).unwrap();
        let count = context.get_variable("foo").unwrap();
        assert_eq!(*count, Value::Int(16));

        let mut scripting = Scripting::new("def count = foo");
        scripting.eval(&mut context).unwrap();
        let count = context.get_variable("count").unwrap();
        assert_eq!(*count, Value::Int(16));

        let mut scripting = Scripting::new("def count = count + 1");
        scripting.eval(&mut context).unwrap();
        let count = context.get_variable("count").unwrap();
        assert_eq!(*count, Value::Int(17));

        let mut scripting = Scripting::new("def foo = foo + 10");
        scripting.eval(&mut context).unwrap();
        let count = context.get_variable("foo").unwrap();
        assert_eq!(*count, Value::Int(26));
    }
}
