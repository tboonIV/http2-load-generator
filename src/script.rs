// TODO REMOVE ME
#![allow(dead_code)]

use crate::function;
use crate::variable::Value;

pub struct ScriptVariable {
    pub name: String,
    pub function: function::Function,
}

impl ScriptVariable {
    pub fn exec(&self, args: Vec<Value>) -> Value {
        match &self.function {
            function::Function::Increment(f) => {
                let arg0 = match args[0] {
                    Value::Int(v) => v,
                    Value::String(ref v) => v.parse::<i32>().unwrap(),
                };
                let value = f.apply(arg0);
                Value::Int(value)
            }
            function::Function::Random(f) => {
                let value = f.apply();
                Value::Int(value)
            }
            function::Function::Split(f) => {
                let arg0 = match args[0] {
                    Value::Int(v) => v.to_string(),
                    Value::String(ref v) => v.to_string(),
                };
                let value = f.apply(arg0);
                Value::String(value)
            }
            function::Function::Now(_f) => {
                todo!()
                // let value = f.apply();
                // Value::String(value)
            }
        }
    }
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Script {
    pub variables: Vec<ScriptVariable>,
}

// impl Script {
//     pub fn values(&self) -> Vec<Value> {
//         self.variables.iter().map(|v| v.value().clone()).collect()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
    // #[test]
    // fn test_script_now() {
    //     // let now = Now()
    //     let now = ScriptVariable {
    //         name: "now".to_string(),
    //         function: function::Function::Now(function::NowFunction {}),
    //     };
    //     let value = now.value();
    //     assert!(value.as_string().len() > 0);
    // }

    #[test]
    fn test_script_random() {
        // let random = Random(1, 10)
        let random = ScriptVariable {
            name: "random".to_string(),
            function: function::Function::Random(function::RandomFunction { min: 1, max: 10 }),
        };
        let value = random.exec(vec![]).as_int();
        assert!(value >= 1 && value <= 10);
    }

    #[test]
    fn test_script_increment() {
        // let counter = 5 + 1
        let counter = ScriptVariable {
            name: "counter".to_string(),
            function: function::Function::Increment(function::IncrementFunction {
                start: 0,
                step: 1,
                threshold: 10,
            }),
        };
        let value = counter.exec(vec![Value::Int(5)]).as_int();
        assert_eq!(value, 6);
    }

    #[test]
    fn test_script_split() {
        // let imsi = Split(":", 1)
        let imsi = ScriptVariable {
            name: "imsi".to_string(),
            function: function::Function::Split(function::SplitFunction {
                delimiter: ":".to_string(),
                index: function::SplitIndex::Nth(1),
            }),
        };
        let value = imsi
            .exec(vec![Value::String("123:456".to_string())])
            .as_string();
        assert_eq!(value, "456".to_string());
    }

    // TODO
    // let imsi = Split(":", 1)
    // let counter = 0
    // let counter++
    // let c1 = counter
    //
}
