use crate::function;
use crate::variable::Value;
use crate::variable::Variable;

#[derive(Debug)]
pub struct ScriptError(String);

pub struct ScriptVariable {
    pub name: String,
    pub function: function::Function,
}

impl ScriptVariable {
    pub fn exec(&self, args: Vec<Value>) -> Result<Value, ScriptError> {
        log::debug!("executing script variable: {}", self.name);
        match &self.function {
            function::Function::Increment(f) => {
                if args.len() == 1 {
                    let arg0 = match args[0] {
                        Value::Int(v) => v,
                        Value::String(ref v) => v.parse::<i32>().unwrap(),
                    };
                    let value = f.apply(arg0);
                    log::debug!("value: {}", value);
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expected 1 argument".into()));
                }
            }
            function::Function::Split(f) => {
                if args.len() == 1 {
                    let arg0 = match args[0] {
                        Value::Int(v) => v.to_string(),
                        Value::String(ref v) => v.to_string(),
                    };
                    let value = f.apply(arg0);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expected 1 argument".into()));
                }
            }
            function::Function::Random(f) => {
                if args.len() == 0 {
                    let value = f.apply();
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expected 0 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                return if args.len() == 1 {
                    let arg0 = args[0].as_string();
                    let value = f.apply(Some(arg0));
                    Ok(Value::String(value))
                } else if args.len() == 0 {
                    let value = f.apply(None);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expected 0 or 1 argument".into()));
                };
            }
        }
    }
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Script {
    pub variables: Vec<ScriptVariable>,
}

impl Script {
    pub fn exec(&self) -> Vec<Variable> {
        let mut variables = vec![];
        for v in &self.variables {
            // TODO some script requires arguments
            // TODO remove hardcode
            let args = if v.name == "imsi" {
                vec![Value::Int(100000)]
            } else {
                vec![]
            };

            let value = v.exec(args).unwrap();
            variables.push(Variable {
                name: v.name.clone(),
                value,
                function: None,
            });
        }
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // let now = Now()
    #[test]
    fn test_script_now() {
        let now = ScriptVariable {
            name: "now".to_string(),
            function: function::Function::Now(function::NowFunction {}),
        };
        let value = now
            .exec(vec![Value::String("%Y-%m-%d".to_string())])
            .unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(value.as_string().len() > 0);
        assert!(value.as_string().starts_with(&today));
    }

    // let random = Random(1, 10)
    #[test]
    fn test_script_random() {
        let random = ScriptVariable {
            name: "random".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
        };
        let value = random.exec(vec![]).unwrap().as_int();
        assert!(value >= 1 && value <= 10);
    }

    // let counter = 5 + 1
    #[test]
    fn test_script_increment() {
        let counter = ScriptVariable {
            name: "counter".to_string(),
            function: function::Function::Increment(function::IncrementFunction {
                start: 0,
                step: 1,
                threshold: 10,
            }),
        };
        let value = counter.exec(vec![Value::Int(5)]).unwrap().as_int();
        assert_eq!(value, 6);
    }

    // let imsi = Split(":", 1)
    #[test]
    fn test_script_split() {
        let imsi = ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
        };
        let value = imsi
            .exec(vec![Value::String("123:456".to_string())])
            .unwrap()
            .as_string();
        assert_eq!(value, "456".to_string());
    }

    // TODO
    // let counter = 0
    // let counter++
    // let c1 = counter
    //
}
