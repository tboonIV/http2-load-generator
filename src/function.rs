use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Function {
    Split(SplitFunction),
    Increment(IncrementFunction),
    Random(RandomFunction),
    Now(NowFunction),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SplitFunction {
    pub delimiter: String,
    pub index: SplitIndex,
}

impl SplitFunction {
    pub fn apply(&self, input: String) -> String {
        match self.index {
            SplitIndex::First => input
                .split(&self.delimiter)
                .next()
                .unwrap_or("")
                .to_string(),
            SplitIndex::Last => input
                .split(&self.delimiter)
                .last()
                .unwrap_or("")
                .to_string(),
            SplitIndex::Nth(index) => input
                .split(&self.delimiter)
                .nth(index)
                .unwrap_or("")
                .to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum SplitIndex {
    First,
    Last,
    Nth(usize),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct NowFunction {}

// impl NowFunction {
//     pub fn apply(&self) -> String {
//         // let now = chrono::Utc::now();
//         // now.to_rfc3339()
//     }
// }

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_split_function() {
        let f = SplitFunction {
            delimiter: ",".to_string(),
            index: SplitIndex::Nth(1),
        };
        assert_eq!(f.apply("a,b,c".to_string()), "b".to_string());
    }

    #[test]
    fn test_split_function_nth() {
        let f = SplitFunction {
            delimiter: ",".to_string(),
            index: SplitIndex::Nth(10),
        };
        assert_eq!(f.apply("a,b,c".to_string()), "".to_string());
    }

    #[test]
    fn test_split_function_last_index() {
        let f = SplitFunction {
            delimiter: "/".to_string(),
            index: SplitIndex::Last,
        };
        assert_eq!(
            f.apply("http://localhost:8080/test/v1/foo/12345".to_string()),
            "12345".to_string()
        );
    }

    #[test]
    fn test_increment_function() {
        let f = IncrementFunction {
            start: 0,
            threshold: 10,
            step: 1,
        };
        assert_eq!(f.apply(1), 2);
    }

    #[test]
    fn test_increment_function_ext() {
        let f = IncrementFunction {
            start: 0,
            threshold: 5,
            step: 2,
        };
        let value = 0;
        let value = f.apply(value);
        assert_eq!(value, 2);
        let value = f.apply(value);
        assert_eq!(value, 4);
        let value = f.apply(value);
        assert_eq!(value, 0);
        let value = f.apply(value);
        assert_eq!(value, 2);
        let value = f.apply(value);
        assert_eq!(value, 4);
        let value = f.apply(value);
        assert_eq!(value, 0);
    }

    #[test]
    fn test_random_function() {
        let f = RandomFunction { min: 1, max: 10 };
        let value = f.apply();
        assert!(value >= 1 && value <= 10);
    }
}
