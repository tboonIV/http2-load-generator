use crate::config;
use crate::function;
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

// #[derive(Clone)]
// pub struct LocalVariableDefine {
//     pub name: String,
//     pub from: String,
// }

#[derive(Clone)]
pub struct LocalVariableValue {
    pub name: String,
    pub value: String,
    pub function: Option<function::Function>,
}

impl LocalVariableValue {
    pub fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
    }
}

// #[derive(Clone)]
pub struct Scenario<'a> {
    pub name: String,
    // pub global: &'a Global,
    pub request: Request,
    pub response: Response,
    pub global_variables: Vec<&'a Variable>,
    pub new_global_variables: Vec<&'a mut LocalVariableValue>,
    pub response_defines: Vec<config::ResponseDefine>,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, global: &'a Global) -> Self {
        // Global Variable
        let mut new_global_variables = vec![];
        // TODO

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
        let mut response_defines = vec![];
        match &config.response.define {
            Some(define) => {
                for v in define {
                    let response_define = v.clone();
                    response_defines.push(response_define);
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
            // global,
            request,
            response,
            global_variables,
            new_global_variables,
            response_defines,
        }
    }

    pub fn next_request(&mut self, new_variables: Vec<LocalVariableValue>) -> HttpRequest {
        // Replace variables in the body
        let body = match &self.request.body {
            Some(body) => {
                // let global_variables2 = &self.new_global_variables;
                let body = if self.new_global_variables.len() != 0 {
                    let mut body = body.clone();
                    for variable in &mut self.new_global_variables {
                        // TODO replace scenario::Function with function::Function
                        let value = variable.value.clone();
                        println!("!!!Before Value: {:?}", value);
                        let value = if let Some(function) = &variable.function {
                            match function {
                                function::Function::Increment(f) => {
                                    let value = value.parse::<i32>().unwrap();
                                    let value = f.apply(value);
                                    let value = value.to_string();
                                    value
                                }
                                // TODO
                                _ => value,
                            }
                        } else {
                            value
                        };

                        // Update variable.value
                        variable.set_value(&value);
                        // self.global.update_variable(&variable.name, &value);

                        println!("!!!After Value: {:?}", value);
                        body = body.replace(&format!("${{{}}}", variable.name), &value);
                    }
                    for variable in &new_variables {
                        // TODO replace scenario::Function with function::Function
                        let value = &variable.value;
                        body = body.replace(&format!("${{{}}}", variable.name), &value);
                    }
                    body
                } else {
                    body.into()
                };

                // This look really inefficient..
                // let variables = &self.global_variables;
                // let body = if variables.len() != 0 {
                //     let mut body = body.clone();
                //     for variable in variables {
                //         // TODO replace scenario::Function with function::Function
                //         let value = variable.function.get_next();
                //         body = body.replace(&format!("${{{}}}", variable.name), &value);
                //     }
                //     for variable in &new_variables {
                //         // TODO replace scenario::Function with function::Function
                //         let value = &variable.value;
                //         body = body.replace(&format!("${{{}}}", variable.name), &value);
                //     }
                //     body
                // } else {
                //     body.into()
                // };
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
                let value = variable.value;
                let value = match &variable.function {
                    Some(f) => match f {
                        function::Function::Increment(f) => {
                            let value = value.parse::<i32>().unwrap();
                            let value = f.apply(value);
                            value.to_string()
                        }
                        function::Function::Random(f) => {
                            let value = f.apply();
                            value.to_string()
                        }
                        function::Function::Split(f) => {
                            let value = f.apply(value.clone());
                            value
                        }
                    },
                    None => value,
                };
                uri = uri.replace(&format!("${{{}}}", variable.name), &value);
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

        for v in &self.response_defines {
            match v.from {
                config::DefineFrom::Header => {
                    //
                    if let Some(headers) = &response.headers {
                        for header in headers {
                            // TODO should be case-insensitive
                            if let Some(value) = header.get(&v.path) {
                                let function = match &v.function {
                                    Some(f) => {
                                        // TODO solve duplicate config::Function and function::Function
                                        // TODO remove scenario::Function
                                        let f: function::Function = f.into();
                                        Some(f)
                                    }
                                    None => None,
                                };
                                log::debug!(
                                    "Set local var from header: '{}', name: '{}' value: '{}'",
                                    v.path,
                                    v.name,
                                    value
                                );
                                let value = LocalVariableValue {
                                    name: v.name.clone(),
                                    value: value.clone(),
                                    function,
                                };
                                values.push(value);
                            }
                        }
                    }
                }
                config::DefineFrom::Body => {
                    if let Some(body) = &response.body {
                        let value = jsonpath_lib::select(&body, &v.path).unwrap();
                        let value = value.get(0).unwrap().as_str().unwrap();
                        log::debug!(
                            "Set local var from json field: '{}', name: '{}' value: '{}'",
                            v.path,
                            v.name,
                            value
                        );
                        let value = LocalVariableValue {
                            name: v.name.clone(),
                            value: value.to_string(),
                            function: None,
                        };
                        values.push(value);
                    }
                }
            }
        }

        values
    }
}

pub struct Global {
    variables: HashMap<String, Variable>,
    test_variables: Vec<LocalVariableValue>,
}

impl Global {
    pub fn new(configs: config::Global) -> Self {
        let mut variables = HashMap::new();
        let mut test_variables = vec![];

        for variable in configs.variables {
            // new local variables
            let f: function::Function = (&variable.function).into();
            let name = variable.name.clone();
            let v = LocalVariableValue {
                name,
                value: variable.value,
                function: Some(f),
            };
            test_variables.push(v);

            // TODO remove this soon
            let v: Box<dyn Function> = match variable.function {
                config::Function::Incremental(prop) => Box::new(IncrementalVariable::new(prop)),
                config::Function::Random(prop) => Box::new(RandomVariable::new(prop)),
                config::Function::Split(prop) => Box::new(SplitVariable::new(prop)),
            };
            variables.insert(
                variable.name.clone(),
                Variable {
                    name: variable.name,
                    function: v,
                },
            );
        }

        Global {
            variables,
            test_variables,
        }
    }

    pub fn get_variable(&self, name: &str) -> Option<&Variable> {
        self.variables.get(name)
    }

    pub fn update_variable(&mut self, name: &str, value: &str) {
        for variable in &mut self.test_variables {
            if variable.name == name {
                variable.set_value(value);
            }
        }
    }
}

// TODO This need refactor
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
    step: i32,
}

impl IncrementalVariable {
    pub fn new(properties: config::IncrementalFunction) -> IncrementalVariable {
        IncrementalVariable {
            value: AtomicI32::new(properties.start),
            threshold: properties.threshold,
            step: properties.step,
        }
    }
}

impl Function for IncrementalVariable {
    fn get_next(&self) -> String {
        let value = &self.value;
        let next = value.fetch_add(self.step, std::sync::atomic::Ordering::SeqCst);
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

#[derive(Debug)]
pub struct SplitVariable {
    delimiter: String,
    index: i32,
}

impl SplitVariable {
    pub fn new(properties: config::SplitFunction) -> SplitVariable {
        SplitVariable {
            delimiter: properties.delimiter,
            index: properties.index,
        }
    }
}

impl Function for SplitVariable {
    fn get_next(&self) -> String {
        // TODO remove hardcode
        let value = "http://example.com/object-id";
        let parts = value.split(&self.delimiter).collect::<Vec<&str>>();
        parts[self.index as usize].to_string()
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
            step: 2,
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
    fn test_split_variable() {
        let variable = SplitVariable::new(config::SplitFunction {
            delimiter: "/".to_string(),
            index: 3,
        });

        let value = variable.get_next();
        assert_eq!(value, "object-id");
    }

    #[test]
    fn test_scenario_next_request() {
        let var1 = Variable {
            name: "VAR1".into(),
            function: Box::new(IncrementalVariable::new(IncrementalFunction {
                start: 0,
                threshold: 10,
                step: 1,
            })),
        };
        let var2 = Variable {
            name: "VAR2".into(),
            function: Box::new(IncrementalVariable::new(IncrementalFunction {
                start: 100,
                threshold: 1000,
                step: 20,
            })),
        };

        let new_var1 = &mut LocalVariableValue {
            name: "VAR1".into(),
            value: "0".into(),
            function: Some(function::Function::Increment(function::IncrementFunction {
                start: 0,
                threshold: 10,
                step: 1,
            })),
        };

        let new_var2 = &mut LocalVariableValue {
            name: "VAR2".into(),
            value: "100".into(),
            function: Some(function::Function::Increment(function::IncrementFunction {
                start: 100,
                threshold: 1000,
                step: 20,
            })),
        };

        let mut new_global_variables = vec![new_var1, new_var2];

        let global_variables = vec![&var1, &var2];

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let mut scenario = Scenario {
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
            new_global_variables,
            response_defines: vec![],
        };

        // First request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            // Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
            Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
        );

        // Second request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            // Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
            Some(serde_json::from_str(r#"{"test": "2_140"}"#).unwrap())
        );

        // Third request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            // Some(serde_json::from_str(r#"{"test": "2_140"}"#).unwrap())
            Some(serde_json::from_str(r#"{"test": "3_160"}"#).unwrap())
        );
    }

    // #[test]
    // fn test_scenario_assert_response() {
    //     let scenario = Scenario {
    //         name: "Scenario_1".into(),
    //         request: Request {
    //             uri: "/endpoint".into(),
    //             method: Method::GET,
    //             headers: None,
    //             body: None,
    //         },
    //         response: Response {
    //             status: StatusCode::OK,
    //         },
    //         global_variables: vec![],
    //         response_defines: vec![],
    //     };
    //
    //     let response1 = HttpResponse {
    //         status: StatusCode::OK,
    //         headers: None,
    //         body: None,
    //         request_start: std::time::Instant::now(),
    //         retry_count: 0,
    //     };
    //
    //     let response2 = HttpResponse {
    //         status: StatusCode::NOT_FOUND,
    //         headers: None,
    //         body: None,
    //         request_start: std::time::Instant::now(),
    //         retry_count: 0,
    //     };
    //
    //     assert_eq!(true, scenario.assert_response(&response1));
    //     assert_eq!(false, scenario.assert_response(&response2));
    // }
    //
    // #[test]
    // fn test_scenario_update_variables() {
    //     let response_defines = vec![config::ResponseDefine {
    //         name: "ObjectId".into(),
    //         from: config::DefineFrom::Body,
    //         path: "$.ObjectId".into(),
    //         function: None,
    //     }];
    //
    //     let scenario = Scenario {
    //         name: "Scenario_1".into(),
    //         request: Request {
    //             uri: "/endpoint".into(),
    //             method: Method::GET,
    //             headers: None,
    //             body: None,
    //         },
    //         response: Response {
    //             status: StatusCode::OK,
    //         },
    //         global_variables: vec![],
    //         response_defines,
    //     };
    //
    //     scenario.update_variables(&HttpResponse {
    //         status: StatusCode::OK,
    //         headers: None,
    //         body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
    //         request_start: std::time::Instant::now(),
    //         retry_count: 0,
    //     });
    // }
}
