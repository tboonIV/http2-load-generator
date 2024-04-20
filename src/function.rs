// TODO remove me
#![allow(dead_code)]
use rand::Rng;

// pub trait Function {
//     fn apply(&self, input: String) -> String;
// }

pub enum Function {
    Split(SplitFunction),
    Increment(IncrementFunction),
    Random(RandomFunction),
}

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

pub struct IncrementFunction {
    pub step: i32,
}

impl IncrementFunction {
    fn apply(&self, input: i32) -> i32 {
        input + self.step
    }
}

pub struct RandomFunction {
    pub min: i32,
    pub max: i32,
}

impl RandomFunction {
    fn apply(&self) -> i32 {
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
        let function = Function::Increment(IncrementFunction { step: 1 });
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
