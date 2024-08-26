#![allow(dead_code)]
use crate::config;
use crate::function;
use crate::scenario::Global;
use crate::variable::Value;
use crate::variable::Variable;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ScriptError(String);

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ScriptArgument {
    Variable(Variable),
    Constant(Value),
}

pub struct Local {
    pub variables: HashMap<String, Value>,
}

pub struct ScriptContext {
    pub local: Local,
}

impl ScriptContext {
    pub fn new() -> Self {
        let local = Local {
            variables: HashMap::new(),
        };
        ScriptContext { local }
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        let value = self.local.variables.get(name);
        // Get from local first
        if let Some(value) = value {
            return Some(value.clone());
        }
        None
    }

    pub fn set_variable(&mut self, _name: &str, value: Value) {
        self.local.variables.insert(_name.into(), value);
    }
}

pub enum Variable2 {
    Variable(String),
    Constant(Value),
}

impl Variable2 {
    pub fn get_value(&self, ctx: &ScriptContext, global: &Global) -> Result<Value, ScriptError> {
        match self {
            Variable2::Variable(name) => {
                // Check context local
                let value = ctx.get_variable(name);
                if let Some(value) = value {
                    return Ok(value.clone());
                }

                // Check global
                let value = global.get_variable_value(name);
                if let Some(value) = value {
                    return Ok(value.clone());
                }

                Err(ScriptError(format!("Variable '{}' not found", name)))
            }
            Variable2::Constant(v) => Ok(v.clone()),
        }
    }
}

pub struct Script2 {
    pub return_var_name: String,
    pub function: function::Function,
    pub args: Vec<Variable2>,
}

impl Script2 {
    pub fn new(config: config::ScriptVariable) -> Self {
        let mut args = vec![];
        if let Some(config_args) = config.args {
            for arg in config_args {
                if arg.is_string() {
                    let str_arg = arg.as_string();
                    if str_arg.starts_with("$") {
                        let var_name = &str_arg[1..];
                        args.push(Variable2::Variable(var_name.into()));
                        continue;
                    }
                }
                let arg = Variable2::Constant(arg);
                args.push(arg);
            }
        }
        Script2 {
            return_var_name: config.name,
            function: config.function,
            args,
        }
    }

    pub fn exec2(&self, ctx: &mut ScriptContext, global: &Global) -> Result<(), ScriptError> {
        let value = match &self.function {
            function::Function::Plus(f) => {
                if self.args.len() == 2 {
                    let arg0 = self.args[0].get_value(ctx, global)?.as_int();
                    let arg1 = self.args[1].get_value(ctx, global)?.as_int();
                    let value = f.apply(arg0, arg1);
                    Value::Int(value)
                } else {
                    return Err(ScriptError("Expects 2 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                if self.args.len() == 1 {
                    let arg0 = self.args[0].get_value(ctx, global)?;
                    let arg0 = arg0.as_string();
                    let value = f.apply(Some(arg0));
                    Value::String(value)
                } else if self.args.len() == 0 {
                    let value = f.apply(None);
                    Value::String(value)
                } else {
                    return Err(ScriptError("Expects 0 or 1 argument".into()));
                }
            }
            // TODO implement other functions
            _ => {
                todo!()
            }
        };

        // Set the return value to the context
        ctx.set_variable(self.return_var_name.as_str(), value);

        Ok(())
    }

    pub fn exec(
        ctx: &ScriptContext,
        global: &Global,
        function: function::Function,
        args: Vec<Variable2>,
    ) -> Result<Value, ScriptError> {
        match &function {
            function::Function::Plus(f) => {
                return if args.len() == 2 {
                    let arg0 = args[0].get_value(ctx, global)?.as_int();
                    let arg1 = args[1].get_value(ctx, global)?.as_int();
                    let value = f.apply(arg0, arg1);
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 2 arguments".into()));
                }
            }
            function::Function::Now(f) => {
                return if args.len() == 1 {
                    let arg0 = args[0].get_value(ctx, global)?;
                    let arg0 = arg0.as_string();
                    let value = f.apply(Some(arg0));
                    Ok(Value::String(value))
                } else if args.len() == 0 {
                    let value = f.apply(None);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expects 0 or 1 argument".into()));
                };
            }
            // everything else
            _ => {
                todo!()
            }
        };
    }
}

pub struct ScriptVariable {
    pub name: String,
    pub function: function::Function,
}

impl ScriptVariable {
    pub fn exec(&self, args: Vec<Value>) -> Result<Value, ScriptError> {
        // log::debug!("Executing script variable: {}", self.name);
        match &self.function {
            function::Function::Split(f) => {
                if args.len() == 1 {
                    let arg0 = args[0].as_string();
                    let value = f.apply(arg0);
                    Ok(Value::String(value))
                } else {
                    return Err(ScriptError("Expects 1 argument".into()));
                }
            }
            function::Function::Random(f) => {
                if args.len() == 0 {
                    let value = f.apply();
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 0 arguments".into()));
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
                    return Err(ScriptError("Expects 0 or 1 argument".into()));
                };
            }
            function::Function::Plus(f) => {
                return if args.len() == 2 {
                    let arg0 = args[0].as_int();
                    let arg1 = args[1].as_int();
                    let value = f.apply(arg0, arg1);
                    Ok(Value::Int(value))
                } else {
                    return Err(ScriptError("Expects 2 arguments".into()));
                }
            }
            function::Function::Copy(f) => {
                if args.len() == 1 {
                    let value = f.apply(&args[0]);
                    Ok(value)
                } else {
                    return Err(ScriptError("Expects 1 argument".into()));
                }
            }
        }
    }
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Script {
    pub variables: Vec<(ScriptVariable, Vec<ScriptArgument>)>,
}

impl Script {
    pub fn exec(&self, new_variables: Vec<Variable>) -> Vec<Variable> {
        let mut variables: Vec<Variable> = vec![];
        for (v, args) in &self.variables {
            let args = args
                .iter()
                .map(|arg| match arg {
                    ScriptArgument::Variable(v) => {
                        // Check if the variable is in the previous executed variables
                        let new_variable = variables.iter().find(|nv| nv.name == v.name);
                        if let Some(nv) = new_variable {
                            // If it is, use the new value
                            nv.value.clone()
                        } else {
                            // Check if the variable is in the new_variables
                            let new_variable = new_variables.iter().find(|nv| nv.name == v.name);
                            if let Some(nv) = new_variable {
                                // If it is, use the new value
                                nv.value.clone()
                            } else {
                                // Otherwise, use the old value
                                v.value.clone()
                            }
                        }
                    }
                    ScriptArgument::Constant(v) => v.clone(),
                })
                .collect::<Vec<Value>>();

            let value = v.exec(args).unwrap();
            log::debug!(
                "Executed variable:{}, new value: '{}'",
                v.name.clone(),
                value.as_string()
            );
            variables.push(Variable {
                name: v.name.clone(),
                value,
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

    // let var1 = var2
    #[test]
    fn test_script_copy() {
        let var1 = ScriptVariable {
            name: "var1".to_string(),
            function: function::Function::Copy(function::CopyFunction {}),
        };

        let var2 = Value::Int(123456789);

        let value = var1.exec(vec![var2]).unwrap().as_int();
        assert_eq!(value, 123456789);
    }

    // let chargingDataRef = Split(":", 1)
    #[test]
    fn test_script_split() {
        let charging_data_ref = ScriptVariable {
            name: "chargingDataRef".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
        };
        let value = charging_data_ref
            .exec(vec![Value::String("123:456".to_string())])
            .unwrap()
            .as_string();
        assert_eq!(value, "456".to_string());
    }

    // let imsi = 1 + 2
    #[test]
    fn test_script_plus() {
        let imsi = ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
        };
        let value = imsi
            .exec(vec![Value::Int(1), Value::Int(2)])
            .unwrap()
            .as_int();
        assert_eq!(value, 3);
    }

    // let var1 = 10
    // let var2 = var1 + 20
    // let var1 = 100
    // let var2 = var1 + 20
    #[test]
    fn test_script_exec_plus() {
        // var1 = 10
        let var1 = ScriptArgument::Variable(Variable {
            name: "var1".to_string(),
            value: Value::Int(10),
        });

        let var2 = ScriptVariable {
            name: "var2".to_string(),
            function: function::Function::Plus(function::PlusFunction {}),
        };

        let const20 = ScriptArgument::Constant(Value::Int(20));

        let script = Script {
            variables: vec![(var2, vec![var1, const20])],
        };
        // var2 = var1 + 20
        let variables = script.exec(vec![]);
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "var2".to_string());
        assert_eq!(variables[0].value.as_int(), 30);

        // var1 = 100
        let var1 = Variable {
            name: "var1".to_string(),
            value: Value::Int(100),
        };

        // var2 = var1 + 20
        let variables = script.exec(vec![var1]);
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "var2".to_string());
        assert_eq!(variables[0].value.as_int(), 120);
    }

    #[test]
    fn test_script_exec2_now() {
        let global = Global { variables: vec![] };
        let local = Local {
            variables: HashMap::new(),
        };
        let ctx = ScriptContext { local };

        let function = function::Function::Now(function::NowFunction {});
        let args = vec![];
        let result = Script2::exec(&ctx, &global, function, args);
        println!("{:?}", result);
    }

    #[test]
    fn test_script_exec2_plus_constant() {
        // Global
        let global = Global { variables: vec![] };

        // Script Context
        let mut ctx = ScriptContext::new();
        ctx.set_variable("var2", Value::Int(22));

        // local var2 = 22
        // local var3 = var2 + 1
        let function = function::Function::Plus(function::PlusFunction {});
        let args = vec![
            Variable2::Variable("var2".into()),
            Variable2::Constant(Value::Int(1)),
        ];
        let result = Script2::exec(&ctx, &global, function, args).unwrap();
        assert_eq!(result.as_int(), 23);

        // insert var3 into context
        ctx.set_variable("var3", result);
        assert_eq!(ctx.get_variable("var3").unwrap().as_int(), 23);
    }

    #[test]
    fn test_script_exec2_plus_global_var() {
        // Global
        let mut global = Global { variables: vec![] };
        global.add_variable(Variable {
            name: "VAR1".into(),
            value: Value::Int(11),
        });

        // Script Context
        let mut ctx = ScriptContext::new();
        ctx.set_variable("var2", Value::Int(22));

        // global VAR1 = 11
        // local var2 = 22
        // local var3 = VAR1 + var2
        let function = function::Function::Plus(function::PlusFunction {});
        let args = vec![
            Variable2::Variable("VAR1".into()),
            Variable2::Variable("var2".into()),
        ];
        let result = Script2::exec(&ctx, &global, function, args).unwrap();
        assert_eq!(result.as_int(), 33);

        // insert var3 into context
        ctx.set_variable("var3", result);
        assert_eq!(ctx.get_variable("var3").unwrap().as_int(), 33);
    }
}

#[test]
fn test_script2_exec2() {
    let global = Global { variables: vec![] };

    // local var2 = 22
    // local var3 = var2 + 1
    let script = Script2::new(config::ScriptVariable {
        name: "var3".to_string(),
        function: function::Function::Plus(function::PlusFunction {}),
        args: Some(vec![Value::String("$var2".to_string()), Value::Int(1)]),
    });

    let mut ctx = ScriptContext::new();
    ctx.set_variable("var2", Value::Int(22));

    script.exec2(&mut ctx, &global).unwrap();

    let var3 = ctx.get_variable("var3").unwrap();
    assert_eq!(var3.as_int(), 23);
}
