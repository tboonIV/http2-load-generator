use crate::config;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use http::Method;
use http::StatusCode;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::AtomicI32;

#[derive(Clone)]
pub struct Request {
    pub uri: String,
    pub method: Method,
    pub body: Option<String>,
    // pub body: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
}

#[derive(Clone)]
pub struct LocalVariable {
    pub name: String,
    pub from: String,
}

#[derive(Clone)]
pub struct Scenario<'a> {
    pub name: String,
    pub global: &'a Global, // Maybe don't need this
    pub request: Request,
    pub response: Response,
    pub variables: Vec<&'a Variable>,

    pub define: Vec<LocalVariable>,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, global: &'a Global) -> Self {
        let mut variables = vec![];
        let body = match &config.request.body {
            Some(body) => {
                let source = body;
                let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
                for caps in variable_pattern.captures_iter(source) {
                    let cap = caps[1].to_string();
                    log::info!("Found variable: {}", cap);

                    let var = global.get_variable(&cap).unwrap();
                    variables.push(var);
                }

                Some(body.to_string())
            }
            None => None,
        };

        let request = Request {
            uri: config.request.path.clone(),
            method: config.request.method.parse().unwrap(),
            body,
        };

        let response = Response {
            status: StatusCode::from_u16(config.response.assert.status).unwrap(),
        };

        Scenario {
            name: config.name.clone(),
            global,
            request,
            response,
            variables,
            define: vec![],
        }
    }

    pub fn next_request(&self) -> HttpRequest {
        // Replace variables in the body
        let body = match &self.request.body {
            Some(body) => {
                // This look really inefficient..
                let variables = &self.variables;
                if variables.len() != 0 {
                    let mut result = body.clone();
                    for variable in variables {
                        let value = variable.function.get_next();
                        result = result.replace(&format!("${{{}}}", variable.name), &value);
                    }
                    Some(serde_json::from_str(&result).unwrap())
                } else {
                    Some(serde_json::from_str(&body).unwrap())
                }
            }
            None => None,
        };
        log::debug!("Body: {:?}", body);

        // TODO - Handle URI too

        let http_request = HttpRequest {
            uri: self.request.uri.clone(),
            method: self.request.method.clone(),
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

    pub fn define_variables(&self, response: &HttpResponse) {
        let _body = match &response.body {
            Some(body) => {
                for v in &self.define {
                    // Simple regex for now
                    let (_, field_name) = v.from.split_at(2);
                    println!("field_name is {}", field_name);
                    let value = body.get(field_name).unwrap().as_str().unwrap();
                    println!("value is {}", value);
                }

                Some(body)
            }
            None => None,
        };
    }
}

pub struct Global {
    variables: HashMap<String, Variable>,
}

impl Global {
    pub fn new(configs: config::Global) -> Self {
        let mut variables = HashMap::new();
        for variable in configs.variables {
            let v: Box<dyn Function> = match variable.function {
                config::Function::Incremental(prop) => Box::new(IncrementalVariable::new(prop)),
                config::Function::Random(prop) => Box::new(RandomVariable::new(prop)),
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

    pub fn get_variable(&self, name: &str) -> Option<&Variable> {
        self.variables.get(name)
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
    pub fn new(properties: config::IncrementalFunction) -> IncrementalVariable {
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
    pub fn new(properties: config::RandomFunction) -> RandomVariable {
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
    use crate::config::IncrementalFunction;

    #[test]
    fn test_incremental_variable() {
        let variable = IncrementalVariable::new(IncrementalFunction {
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
        let variable = RandomVariable::new(config::RandomFunction { min: 0, max: 10 });

        let value = variable.get_next().parse::<i32>().unwrap();
        assert!(value >= 0 && value <= 10);
    }

    #[test]
    fn test_scenario_next_request() {
        let var1 = Variable {
            name: "VAR1".into(),
            function: Box::new(IncrementalVariable::new(IncrementalFunction {
                start: 0,
                threshold: 10,
                steps: 1,
            })),
        };
        let var2 = Variable {
            name: "VAR2".into(),
            function: Box::new(IncrementalVariable::new(IncrementalFunction {
                start: 100,
                threshold: 1000,
                steps: 20,
            })),
        };

        let mut variables = HashMap::new();
        variables.insert("VAR1".into(), var1);
        variables.insert("VAR2".into(), var2);
        let global = Global { variables };

        let variables = vec![&global.variables["VAR1"], &global.variables["VAR2"]];

        let scenario = Scenario {
            name: "Scenario_1".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                body: Some(r#"{"test": "${VAR1}_${VAR2}"}"#.into()),
            },
            response: Response {
                status: StatusCode::OK,
            },
            variables,
            define: vec![],
        };

        // First request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
        );

        // Second request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
        );

        // Third request
        let request = scenario.next_request();
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "2_140"}"#).unwrap())
        );
    }

    #[test]
    fn test_scenario_assert_response() {
        let scenario = Scenario {
            name: "Scenario_1".into(),
            global: &Global {
                variables: HashMap::new(),
            },
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            variables: vec![],
            define: vec![],
        };

        let response1 = HttpResponse {
            status: StatusCode::OK,
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        let response2 = HttpResponse {
            status: StatusCode::NOT_FOUND,
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        assert_eq!(true, scenario.assert_response(&response1));
        assert_eq!(false, scenario.assert_response(&response2));
    }

    #[test]
    fn test_scenario_define_variables() {
        let define = vec![LocalVariable {
            name: "ObjectId".into(),
            from: "$.ObjectId".into(),
        }];

        let scenario = Scenario {
            name: "Scenario_1".into(),
            global: &Global {
                variables: HashMap::new(),
            },
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            variables: vec![],
            define,
        };

        scenario.define_variables(&HttpResponse {
            status: StatusCode::OK,
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        });
    }
}
