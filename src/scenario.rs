use crate::config;
use http::Method;
use std::sync::atomic::AtomicI32;

#[derive(Debug, Clone)]
pub struct ScenarioParameter {
    pub name: String,
    pub uri: String,
    pub method: Method,
    pub body: Option<serde_json::Value>,
}

impl From<&config::Scenario> for ScenarioParameter {
    fn from(config: &config::Scenario) -> Self {
        // let body2 = match &config.body {
        //     Some(body) => {
        //         let body = "";
        //         Some(body)
        //     }
        //     None => None,
        // };

        ScenarioParameter {
            name: config.name.clone(),
            uri: config.path.clone(),
            method: config.method.parse().unwrap(),
            body: match &config.body {
                Some(body) => Some(serde_json::from_str(body).unwrap()),
                None => None,
            },
        }
    }
}

pub trait Function {
    fn get_next(&self) -> String;
}

#[derive(Debug)]
pub struct IncrementalVariable {
    pub name: String,
    pub value: AtomicI32,
}

impl IncrementalVariable {
    pub fn new(name: &str) -> IncrementalVariable {
        IncrementalVariable {
            name: name.into(),
            value: AtomicI32::new(0),
        }
    }
}

impl Function for IncrementalVariable {
    fn get_next(&self) -> String {
        let value = &self.value;
        let next = value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        next.to_string()
    }
}

#[derive(Debug)]
pub struct RandomVariable {
    pub name: String,
    pub min: u32,
    pub max: u32,
}

impl RandomVariable {
    pub fn new(name: &str, min: u32, max: u32) -> RandomVariable {
        RandomVariable {
            name: name.into(),
            min,
            max,
        }
    }
}

impl Function for RandomVariable {
    fn get_next(&self) -> String {
        let value = rand::random::<u32>() % (self.max - self.min) + self.min;
        value.to_string()
    }
}
