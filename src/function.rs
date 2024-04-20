use rand::Rng;

pub trait Function {
    fn apply(&self, input: String) -> String;
}

pub struct SplitFunction {
    pub delimiter: String,
    pub index: usize,
}

impl Function for SplitFunction {
    fn apply(&self, input: String) -> String {
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

impl Function for IncrementFunction {
    fn apply(&self, input: String) -> String {
        let value = input.parse::<i32>().unwrap_or(0);
        (value + self.step).to_string()
    }
}

pub struct RandomFunction {
    pub min: i32,
    pub max: i32,
}

impl Function for RandomFunction {
    fn apply(&self, _input: String) -> String {
        let mut rng = rand::thread_rng();
        let value = rng.gen_range(self.min..=self.max);
        value.to_string()
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_split_function() {
        let function = SplitFunction {
            delimiter: ",".to_string(),
            index: 1,
        };
        assert_eq!(function.apply("a,b,c".to_string()), "b".to_string());
    }

    #[test]
    fn test_split_function_nth() {
        let function = SplitFunction {
            delimiter: ",".to_string(),
            index: 10,
        };
        assert_eq!(function.apply("a,b,c".to_string()), "".to_string());
    }

    #[test]
    fn test_increment_function() {
        let function = IncrementFunction { step: 1 };
        assert_eq!(function.apply("10".to_string()), "11".to_string());
    }

    #[test]
    fn test_random_function() {
        let function = RandomFunction { min: 1, max: 10 };
        let value = function.apply("".to_string()).parse::<i32>().unwrap();
        assert!(value >= 1 && value <= 10);
    }
}
