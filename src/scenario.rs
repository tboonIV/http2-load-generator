use crate::config;
use crate::config::VariableType;
use http::Method;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::AtomicI32;

pub struct Global {
    variables: HashMap<String, Variable>,
}

impl Global {
    pub fn new(configs: Vec<config::Variable>) -> Self {
        let mut variables = HashMap::new();
        for variable in configs {
            let v: Box<dyn Function> = match variable.variable_type {
                VariableType::Incremental => Box::new(IncrementalVariable::new(&variable.name)),
                VariableType::Random => Box::new(RandomVariable::new(&variable.name, 0, 100)), // TODO configurable min and max
            };
            variables.insert(
                variable.name.clone(),
                Variable {
                    name: variable.name,
                    function: v,
                },
            );
        }
        Global { variables }
    }
}

#[derive(Clone)]
pub struct ScenarioParameter<'a> {
    pub name: String,
    pub global: &'a Global,
    pub uri: String,
    pub method: Method,
    pub body: Option<serde_json::Value>,
}
impl<'a> ScenarioParameter<'a> {
    pub fn new(config: &config::Scenario, global: &'a Global) -> Self {
        let body = match &config.request.body {
            Some(body) => {
                let source = body;
                let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
                for caps in variable_pattern.captures_iter(source) {
                    let cap = caps[1].to_string();
                    log::info!("Found variable: {}", cap);
                    // let var = global.get_variable(&cap).unwrap();
                    // variables.push(var);
                }

                Some(serde_json::from_str(body).unwrap())
            }
            None => None,
        };

        ScenarioParameter {
            name: config.name.clone(),
            global,
            uri: config.request.path.clone(),
            method: config.request.method.parse().unwrap(),
            body,
        }
    }

    // TODO next_request()
}

pub struct Variable {
    pub name: String,
    pub function: Box<dyn Function>,
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
