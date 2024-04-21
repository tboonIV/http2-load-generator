// TODO remove me
#![allow(dead_code)]
use crate::config;
use rand::Rng;

// pub trait Function {
//     fn apply(&self, input: String) -> String;
// }

#[derive(Clone)]
pub enum Function {
    Split(SplitFunction),
    Increment(IncrementFunction),
    Random(RandomFunction),
}

impl From<&config::Function> for Function {
    fn from(f: &config::Function) -> Self {
        match f {
            config::Function::Split(f) => Function::Split(SplitFunction {
                delimiter: f.delimiter.clone(),
                index: f.index as usize,
            }),
            config::Function::Incremental(f) => Function::Increment(IncrementFunction {
                start: f.start,
                threshold: f.threshold,
                step: f.step,
            }),
            config::Function::Random(f) => Function::Random(RandomFunction {
                min: f.min,
                max: f.max,
            }),
        }
    }
}

#[derive(Clone)]
pub struct SplitFunction {
    pub delimiter: String,
    pub index: usize,
}

impl SplitFunction {
    pub fn apply(&self, input: String) -> String {
        input
            .split(&self.delimiter)
            .nth(self.index)
            .unwrap_or("")
            .to_string()
    }
}

#[derive(Clone)]
pub struct IncrementFunction {
    pub start: i32,
    pub threshold: i32,
    pub step: i32,
}

impl IncrementFunction {
    pub fn apply(&self, input: i32) -> i32 {
        let output = input + self.step;
        if output > self.threshold {
            self.start
        } else {
            output
        }
    }
}

#[derive(Clone)]
pub struct RandomFunction {
    pub min: i32,
    pub max: i32,
}

impl RandomFunction {
    pub fn apply(&self) -> i32 {
        let mut rng = rand::thread_rng();
        let value = rng.gen_range(self.min..=self.max);
        value
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_split_function() {
        let function = Function::Split(SplitFunction {
            delimiter: ",".to_string(),
            index: 1,
        });
        match function {
            Function::Split(f) => {
                assert_eq!(f.apply("a,b,c".to_string()), "b".to_string());
            }
            _ => panic!("Invalid function"),
        }
    }

    #[test]
    fn test_split_function_nth() {
        let function = Function::Split(SplitFunction {
            delimiter: ",".to_string(),
            index: 10,
        });
        match function {
            Function::Split(f) => {
                assert_eq!(f.apply("a,b,c".to_string()), "".to_string());
            }
            _ => panic!("Invalid function"),
        }
    }

    #[test]
    fn test_increment_function() {
        let function = Function::Increment(IncrementFunction {
            start: 0,
            threshold: 10,
            step: 1,
        });
        match function {
            Function::Increment(f) => {
                assert_eq!(f.apply(1), 2);
            }
            _ => panic!("Invalid function"),
        }
    }

    #[test]
    fn test_random_function() {
        let function = Function::Random(RandomFunction { min: 1, max: 10 });
        match function {
            Function::Random(f) => {
                let value = f.apply();
                assert!(value >= 1 && value <= 10);
            }
            _ => panic!("Invalid function"),
        }
    }
}
