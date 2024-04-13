use crate::config;
use crate::config::VariableProperties;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use http::Method;
use http::StatusCode;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::AtomicI32;

// pub struct Request {
//     pub uri: String,
//     pub method: Method,
//     pub body: Option<String>,
// }

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
}

#[derive(Clone)]
pub struct Scenario<'a> {
    pub name: String,
    pub global: &'a Global,
    pub uri: String,
    pub method: Method,
    // pub body: Option<serde_json::Value>,
    pub body: Option<String>,
    pub response: Response,
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

        let response = Response {
            status: StatusCode::from_u16(config.response.status).unwrap(),
        };

        Scenario {
            name: config.name.clone(),
            global,
            uri: config.request.path.clone(),
            method: config.request.method.parse().unwrap(),
            body,
            response,
        }
    }

    pub fn next_request(&self) -> HttpRequest {
        // TODO - Skip varaible replacement if no varaibles are detected in the body
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
        log::debug!("Body: {:?}", body);

        let http_request = HttpRequest {
            uri: self.uri.clone(),
            method: self.method.clone(),
            body,
        };

        http_request
    }

    pub fn assert_response(&self, response: &HttpResponse) -> bool {
        if self.response.status != response.status {
            log::error!(
                "Expected status code: {:?}, got: {:?}",
                self.response.status,
                response.status
            );
            return false;
        }
        return true;
    }
}

pub struct Global {
    variables: HashMap<String, Variable>,
}

impl Global {
    pub fn new(configs: Vec<config::Variable>) -> Self {
        let mut variables = HashMap::new();
        for variable in configs {
            let v: Box<dyn Function> = match variable.properties {
                VariableProperties::Incremental(prop) => Box::new(IncrementalVariable::new(prop)),
                VariableProperties::Random(prop) => Box::new(RandomVariable::new(prop)),
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

pub struct Variable {
    pub name: String,
    pub function: Box<dyn Function>,
}

pub trait Function {
    fn get_next(&self) -> String;
}

#[derive(Debug)]
pub struct IncrementalVariable {
    value: AtomicI32,
    threshold: i32,
    steps: i32,
}

impl IncrementalVariable {
    pub fn new(properties: config::IncrementalProperties) -> IncrementalVariable {
        IncrementalVariable {
            value: AtomicI32::new(properties.start),
            threshold: properties.threshold,
            steps: properties.steps,
        }
    }
}

impl Function for IncrementalVariable {
    fn get_next(&self) -> String {
        let value = &self.value;
        let next = value.fetch_add(self.steps, std::sync::atomic::Ordering::SeqCst);
        let next = next % (self.threshold + 1);
        next.to_string()
    }
}

#[derive(Debug)]
pub struct RandomVariable {
    min: i32,
    max: i32,
}

impl RandomVariable {
    pub fn new(properties: config::RandomProperties) -> RandomVariable {
        RandomVariable {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IncrementalProperties;

    #[test]
    fn test_incremental_variable() {
        let variable = IncrementalVariable::new(IncrementalProperties {
            start: 0,
            threshold: 5,
            steps: 2,
        });

        assert_eq!(variable.get_next(), "0");
        assert_eq!(variable.get_next(), "2");
        assert_eq!(variable.get_next(), "4");
        assert_eq!(variable.get_next(), "0");
        assert_eq!(variable.get_next(), "2");
        assert_eq!(variable.get_next(), "4");
        assert_eq!(variable.get_next(), "0");
    }

    #[test]
    fn test_random_variable() {
        let variable = RandomVariable::new(config::RandomProperties { min: 0, max: 10 });

        let value = variable.get_next().parse::<i32>().unwrap();
        assert!(value >= 0 && value <= 10);
    }

    #[test]
    fn test_scenario_next_request() {
        let var1 = Variable {
            name: "VAR1".into(),
            function: Box::new(IncrementalVariable::new(IncrementalProperties {
                start: 0,
                threshold: 10,
                steps: 1,
            })),
        };
        let var2 = Variable {
            name: "VAR2".into(),
            function: Box::new(IncrementalVariable::new(IncrementalProperties {
                start: 100,
                threshold: 1000,
                steps: 20,
            })),
        };

        let mut variables = HashMap::new();
        variables.insert("VAR1".into(), var1);
        variables.insert("VAR2".into(), var2);

        let global = Global { variables };

        let scenario = Scenario {
            name: "test".into(),
            global: &global,
            uri: "/test".into(),
            method: Method::GET,
            body: Some(r#"{"test": "${VAR1}_${VAR2}"}"#.into()),
        };

        // First request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/test");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
        );

        // Second request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/test");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
        );

        // Third request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/test");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "2_140"}"#).unwrap())
        );
    }
}
