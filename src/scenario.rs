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
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
    // pub body: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
}

#[derive(Clone)]
pub struct LocalVariableDefine {
    pub name: String,
    pub from: String,
}

#[derive(Clone)]
pub struct LocalVariableValue {
    pub name: String,
    pub value: String,
}

#[derive(Clone)]
pub struct Scenario<'a> {
    pub name: String,
    // pub global: &'a Global,
    pub request: Request,
    pub response: Response,
    pub global_variables: Vec<&'a Variable>,
    pub local_variables: Vec<LocalVariableDefine>,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, global: &'a Global) -> Self {
        // Global Variable
        let mut global_variables = vec![];
        let body = match &config.request.body {
            Some(body) => {
                let source = body;
                let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
                for caps in variable_pattern.captures_iter(source) {
                    let cap = caps[1].to_string();
                    log::debug!("Found global variable: {}", cap);

                    let var = global.get_variable(&cap).unwrap();
                    global_variables.push(var);
                }

                Some(body.to_string())
            }
            None => None,
        };

        //Local Variable
        let mut local_variables = vec![];
        match &config.response.define {
            Some(define) => {
                for v in define {
                    let local_variable = LocalVariableDefine {
                        name: v.name.clone(),
                        from: v.from.clone(),
                    };
                    local_variables.push(local_variable);
                }
            }
            None => {}
        }

        // Requets
        let request = Request {
            uri: config.request.path.clone(),
            method: config.request.method.parse().unwrap(),
            headers: config.request.headers.clone(),
            body,
        };

        // Response
        let response = Response {
            status: StatusCode::from_u16(config.response.assert.status).unwrap(),
        };

        Scenario {
            name: config.name.clone(),
            request,
            response,
            global_variables,
            local_variables,
        }
    }

    pub fn next_request(&self, new_variables: Vec<LocalVariableValue>) -> HttpRequest {
        // Replace variables in the body
        let body = match &self.request.body {
            Some(body) => {
                // This look really inefficient..
                let variables = &self.global_variables;
                let body = if variables.len() != 0 {
                    let mut body = body.clone();
                    for variable in variables {
                        let value = variable.function.get_next();
                        body = body.replace(&format!("${{{}}}", variable.name), &value);
                    }
                    for variable in &new_variables {
                        let value = &variable.value;
                        body = body.replace(&format!("${{{}}}", variable.name), &value);
                    }
                    body
                } else {
                    body.into()
                };
                Some(serde_json::from_str(&body).unwrap())
            }
            None => None,
        };
        log::debug!("Body: {:?}", body);

        let uri = {
            let mut uri = self.request.uri.clone();
            for variable in new_variables {
                // TODO replace regex with something better
                // TODO throw error if variable not found
                uri = uri.replace(&format!("${{{}}}", variable.name), &variable.value);
            }
            uri
        };

        let http_request = HttpRequest {
            uri,
            method: self.request.method.clone(),
            headers: self.request.headers.clone(),
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

    pub fn update_variables(&self, response: &HttpResponse) -> Vec<LocalVariableValue> {
        let mut values = vec![];

        let _body = match &response.body {
            Some(body) => {
                for v in &self.local_variables {
                    let value = jsonpath_lib::select(&body, &v.from).unwrap();
                    let value = value.get(0).unwrap().as_str().unwrap();
                    log::debug!(
                        "Set local var from json field: '{}', name: '{}' value: '{}'",
                        v.from,
                        v.name,
                        value
                    );
                    let value = LocalVariableValue {
                        name: v.name.clone(),
                        value: value.to_string(),
                    };
                    values.push(value);
                }

                Some(body)
            }
            None => None,
        };

        values
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

        let global_variables = vec![&var1, &var2];

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let scenario = Scenario {
            name: "Scenario_1".into(),
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: Some(vec![headers]),
                body: Some(r#"{"test": "${VAR1}_${VAR2}"}"#.into()),
            },
            response: Response {
                status: StatusCode::OK,
            },
            global_variables,
            local_variables: vec![],
        };

        // First request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
        );

        // Second request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
        );

        // Third request
        let request = scenario.next_request(vec![]);
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
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            global_variables: vec![],
            local_variables: vec![],
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
    fn test_scenario_update_variables() {
        let local_variables = vec![LocalVariableDefine {
            name: "ObjectId".into(),
            from: "$.ObjectId".into(),
        }];

        let scenario = Scenario {
            name: "Scenario_1".into(),
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            global_variables: vec![],
            local_variables,
        };

        scenario.update_variables(&HttpResponse {
            status: StatusCode::OK,
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        });
    }
}
