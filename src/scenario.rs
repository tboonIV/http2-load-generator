use crate::config;
use crate::config::VariableProperties;
use crate::http_api::HttpRequest;
use http::Method;
use rand::Rng;
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
            let v: Box<dyn Function> = match variable.properties {
                VariableProperties::Incremental(prop) => {
                    Box::new(IncrementalVariable::new(&variable.name, prop))
                }
                VariableProperties::Random(prop) => {
                    Box::new(RandomVariable::new(&variable.name, prop))
                }
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
pub struct Scenario<'a> {
    pub name: String,
    pub global: &'a Global,
    pub uri: String,
    pub method: Method,
    // pub body: Option<serde_json::Value>,
    pub body: Option<String>,
}
impl<'a> Scenario<'a> {
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

                Some(body.to_string())
            }
            None => None,
        };

        Scenario {
            name: config.name.clone(),
            global,
            uri: config.request.path.clone(),
            method: config.request.method.parse().unwrap(),
            body,
        }
    }

    pub fn next_request(&self) -> HttpRequest {
        // TODO - Skip varaible replacement if no varaibles are detected in the body
        // TODO - Add unit test
        // TODO - Handle URI
        //
        // Replace variables in the body
        let body = match &self.body {
            Some(body) => {
                // This look really inefficient..
                let mut result = body.clone();
                for variable in self.global.variables.values() {
                    let value = variable.function.get_next();
                    result = result.replace(&format!("${{{}}}", variable.name), &value);
                }
                Some(serde_json::from_str(&result).unwrap())
            }
            None => None,
        };
        log::info!("Body: {:?}", body);

        let http_request = HttpRequest {
            uri: self.uri.clone(),
            method: self.method.clone(),
            body,
        };

        http_request
    }
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
    pub min: i32,
    pub max: i32,
    pub steps: i32,
}

impl IncrementalVariable {
    pub fn new(name: &str, properties: config::IncrementalProperties) -> IncrementalVariable {
        IncrementalVariable {
            name: name.into(),
            value: AtomicI32::new(0),
            min: properties.min,
            max: properties.max,
            steps: properties.steps,
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
    pub min: i32,
    pub max: i32,
}

impl RandomVariable {
    pub fn new(name: &str, properties: config::RandomProperties) -> RandomVariable {
        log::info!("Creating RandomVariable: {}", name);
        log::info!("min = {}", properties.min);
        log::info!("max = {}", properties.max);
        RandomVariable {
            name: name.into(),
            min: properties.min,
            max: properties.max,
        }
    }
}

impl Function for RandomVariable {
    fn get_next(&self) -> String {
        let mut rng = rand::thread_rng();
        let value = rng.gen_range(self.min..=self.max);
        value.to_string()
    }
}
