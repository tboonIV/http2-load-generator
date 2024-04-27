use crate::config;
use crate::function;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use crate::variable::Value;
use crate::variable::Variable;
use http::Method;
use http::StatusCode;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

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
pub struct Scenario<'a> {
    pub name: String,
    pub global: &'a Global,
    pub request: Request,
    pub response: Response,
    pub response_defines: Vec<config::ResponseDefine>,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, global: &'a Global) -> Self {
        // Global Variable
        // let mut new_global_variables = vec![];
        // TODO

        // let mut global_variables = vec![];
        let body = match &config.request.body {
            Some(body) => {
                let source = body;
                let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();
                for caps in variable_pattern.captures_iter(source) {
                    let cap = caps[1].to_string();
                    log::debug!("Found global variable: {}", cap);

                    // let var = global.get_variable(&cap).unwrap();
                    // global_variables.push(var);
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
            global,
            request,
            response,
            response_defines,
        }
    }

    pub fn next_request(&mut self, new_variables: Vec<Variable>) -> HttpRequest {
        // Replace variables in the body
        let body = match &self.request.body {
            Some(body) => {
                let variables = &self.global.variables;

                let body = if variables.len() != 0 {
                    let mut body = body.clone();
                    for v in variables {
                        let mut variable = v.lock().unwrap();

                        let value = variable.value.clone();
                        if let Some(function) = &variable.function {
                            // println!("!!!Before Value: {:?}", value);
                            let v = match function {
                                function::Function::Increment(f) => {
                                    let value = match value {
                                        Value::Int(v) => v,
                                        Value::String(ref v) => v.parse::<i32>().unwrap(),
                                    };
                                    let value = f.apply(value);
                                    Value::Int(value)
                                }
                                function::Function::Random(f) => {
                                    let value = f.apply();
                                    Value::Int(value)
                                }
                                function::Function::Split(f) => {
                                    let value = match value {
                                        Value::Int(v) => v.to_string(),
                                        Value::String(ref v) => v.to_string(),
                                    };
                                    let value = f.apply(value);
                                    Value::String(value)
                                }
                            };
                            //println!("!!!After Value: {:?}", new_value);
                            variable.set_value(v);
                        };

                        body = match value {
                            Value::Int(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v.to_string())
                            }
                            Value::String(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v)
                            }
                        }
                    }
                    for variable in &new_variables {
                        // TODO replace scenario::Function with function::Function
                        let value = &variable.value;
                        body = match value {
                            Value::Int(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v.to_string())
                            }
                            Value::String(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v)
                            }
                        }
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
                let value = variable.value;
                let value = match &variable.function {
                    Some(f) => match f {
                        function::Function::Increment(f) => {
                            let value = match value {
                                Value::Int(v) => v,
                                Value::String(v) => v.parse::<i32>().unwrap(),
                            };
                            let value = f.apply(value);
                            Value::Int(value)
                        }
                        function::Function::Random(f) => {
                            let value = f.apply();
                            Value::Int(value)
                        }
                        function::Function::Split(f) => {
                            let value = match value {
                                Value::Int(v) => v.to_string(),
                                Value::String(v) => v,
                            };
                            let value = f.apply(value);
                            Value::String(value)
                        }
                    },
                    None => value,
                };
                match value {
                    Value::Int(v) => {
                        uri = uri.replace(&format!("${{{}}}", variable.name), &v.to_string());
                    }
                    Value::String(v) => {
                        uri = uri.replace(&format!("${{{}}}", variable.name), &v);
                    }
                }
                // uri = uri.replace(&format!("${{{}}}", variable.name), &value);
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

    pub fn update_variables(&self, response: &HttpResponse) -> Vec<Variable> {
        let mut values = vec![];

        for v in &self.response_defines {
            match v.from {
                config::DefineFrom::Header => {
                    //
                    if let Some(headers) = &response.headers {
                        for header in headers {
                            // TODO should be case-insensitive
                            if let Some(value) = header.get(&v.path) {
                                // let function = match &v.function {
                                //     Some(f) => {
                                //         // TODO solve duplicate config::Function and function::Function
                                //         // TODO remove scenario::Function
                                //         let f: function::Function = f.into();
                                //         Some(f)
                                //     }
                                //     None => None,
                                // };
                                log::debug!(
                                    "Set local var from header: '{}', name: '{}' value: '{}'",
                                    v.path,
                                    v.name,
                                    value
                                );

                                let value = Variable {
                                    name: v.name.clone(),
                                    value: Value::String(value.clone()), // TODO also support Int
                                    function: v.function.clone(),
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
                        let value = Variable {
                            name: v.name.clone(),
                            value: Value::String(value.to_string()), // TODO also support Int
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
    variables: Vec<Arc<Mutex<Variable>>>,
}

impl Global {
    pub fn new(configs: config::Global) -> Self {
        let mut variables = vec![];

        for variable in configs.variables {
            // if let Some(function) = &variable.function {
            // let f: function::Function = function.into();
            // let f = function;
            //let name = variable.name.clone();

            // TODO remove duplicate
            // let value = match variable.value {
            //     config::Value::Int(v) => Value::Int(v),
            //     config::Value::String(v) => Value::String(v),
            // };
            // let v = Variable {
            //     name,
            //     value,
            //     function: variable.function,
            // };
            let v = variable;
            variables.push(Arc::new(Mutex::new(v)));
            // }
        }

        Global { variables }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_next_request() {
        let new_var1 = Arc::new(Mutex::new(Variable {
            name: "VAR1".into(),
            value: Value::Int(0),
            function: Some(function::Function::Increment(function::IncrementFunction {
                start: 0,
                threshold: 10,
                step: 1,
            })),
        }));

        let new_var2 = Arc::new(Mutex::new(Variable {
            name: "VAR2".into(),
            value: Value::Int(100),
            function: Some(function::Function::Increment(function::IncrementFunction {
                start: 100,
                threshold: 1000,
                step: 20,
            })),
        }));

        let variables = vec![new_var1, new_var2];
        let global = Global { variables };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let mut scenario = Scenario {
            name: "Scenario_1".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: Some(vec![headers]),
                body: Some(r#"{"test": "${VAR1}_${VAR2}"}"#.into()),
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines: vec![],
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
        let global = Global { variables: vec![] };
        let scenario = Scenario {
            name: "Scenario_1".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines: vec![],
        };

        let response1 = HttpResponse {
            status: StatusCode::OK,
            headers: None,
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        let response2 = HttpResponse {
            status: StatusCode::NOT_FOUND,
            headers: None,
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        assert_eq!(true, scenario.assert_response(&response1));
        assert_eq!(false, scenario.assert_response(&response2));
    }

    #[test]
    fn test_scenario_update_variables() {
        let response_defines = vec![config::ResponseDefine {
            name: "ObjectId".into(),
            from: config::DefineFrom::Body,
            path: "$.ObjectId".into(),
            function: None,
        }];
        let global = Global { variables: vec![] };

        let scenario = Scenario {
            name: "Scenario_1".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines,
        };

        scenario.update_variables(&HttpResponse {
            status: StatusCode::OK,
            headers: None,
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        });
    }
}
