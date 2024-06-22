use crate::config;
use crate::function;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use crate::variable::Value;
use crate::variable::Variable;
use http::Method;
use http::StatusCode;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone)]
pub struct Request {
    pub uri: String,
    pub method: Method,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
    // pub body: Option<serde_json::Value>,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResponseDefine {
    pub name: String,
    pub from: DefineFrom,
    pub path: String,
    pub function: Option<function::Function>,
}

#[derive(Debug, Deserialize, PartialEq, Copy, Clone)]
pub enum DefineFrom {
    Header,
    Body,
}

// #[derive(Clone)]
pub struct Scenario<'a> {
    pub name: String,
    pub base_url: String,
    pub global: &'a Global,
    pub request: Request,
    pub response: Response,
    pub response_defines: Vec<ResponseDefine>,
    pub assert_panic: bool,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, base_url: &str, global: &'a Global) -> Self {
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
            timeout: config.request.timeout,
        };

        // Response
        let response = Response {
            status: StatusCode::from_u16(config.response.assert.status).unwrap(),
        };

        Scenario {
            name: config.name.clone(),
            base_url: base_url.into(),
            global,
            request,
            response,
            response_defines,
            assert_panic: true,
        }
    }

    pub fn next_request(&mut self, new_variables: Vec<Variable>) -> HttpRequest {
        // Replace variables in the body
        let body = match &self.request.body {
            Some(body) => {
                let variables = &self.global.variables;
                let body = if variables.len() != 0 {
                    let mut body = body.clone();
                    // Apply Global Variables
                    for v in variables {
                        let mut variable = v.lock().unwrap();
                        variable.apply();

                        let value = variable.value.clone();

                        body = match value {
                            Value::Int(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v.to_string())
                            }
                            Value::String(v) => {
                                body.replace(&format!("${{{}}}", variable.name), &v)
                            }
                        }
                    }
                    // Apply Local Variables
                    for variable in &new_variables {
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
            // Apply Local Variables
            for mut variable in new_variables {
                // TODO throw error if variable not found
                // TODO replace regex with something better
                //
                variable.apply();
                let value = variable.value;

                match value {
                    Value::Int(v) => {
                        uri = uri.replace(&format!("${{{}}}", variable.name), &v.to_string());
                    }
                    Value::String(v) => {
                        uri = uri.replace(&format!("${{{}}}", variable.name), &v);
                    }
                }
            }
            uri
        };

        let uri = format!("{}{}", self.base_url, uri);

        let http_request = HttpRequest {
            uri,
            method: self.request.method.clone(),
            headers: self.request.headers.clone(),
            body,
            timeout: self.request.timeout.clone(),
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
            if self.assert_panic {
                panic!(
                    "Expected status code: {:?}, got: {:?}",
                    self.response.status, response.status
                );
            }
            return false;
        }
        return true;
    }

    pub fn update_variables(&self, response: &HttpResponse) -> Vec<Variable> {
        let mut values = vec![];

        for v in &self.response_defines {
            match v.from {
                DefineFrom::Header => {
                    let headers = &response.headers;
                    if let Some(header) = headers.get(&v.path) {
                        let value = header.to_str().unwrap();
                        log::debug!(
                            "Set local var from header: '{}', name: '{}' value: '{}'",
                            v.path,
                            v.name,
                            value,
                        );

                        let value = Variable {
                            name: v.name.clone(),
                            value: Value::String(value.into()), // TODO also support Int
                            function: v.function.clone(),
                        };
                        values.push(value);
                    }
                }
                DefineFrom::Body => {
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
            let v = variable;
            variables.push(Arc::new(Mutex::new(v)));
        }

        Global { variables }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function;

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
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: Some(vec![headers]),
                body: Some(r#"{"test": "${VAR1}_${VAR2}"}"#.into()),
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines: vec![],
            assert_panic: false,
        };

        // First request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "http://localhost:8080/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "1_120"}"#).unwrap())
        );

        // Second request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "http://localhost:8080/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "2_140"}"#).unwrap())
        );

        // Third request
        let request = scenario.next_request(vec![]);
        assert_eq!(request.uri, "http://localhost:8080/endpoint");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "3_160"}"#).unwrap())
        );
    }

    #[test]
    fn test_scenario_assert_response() {
        let global = Global { variables: vec![] };
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines: vec![],
            assert_panic: false,
        };

        let response1 = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        let response2 = HttpResponse {
            status: StatusCode::NOT_FOUND,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        assert_eq!(true, scenario.assert_response(&response1));
        assert_eq!(false, scenario.assert_response(&response2));
    }

    #[test]
    fn test_scenario_update_variables() {
        let response_defines = vec![ResponseDefine {
            name: "ObjectId".into(),
            from: DefineFrom::Body,
            path: "$.ObjectId".into(),
            function: None,
        }];
        let global = Global { variables: vec![] };

        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                method: Method::GET,
                headers: None,
                body: None,
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
            },
            response_defines,
            assert_panic: false,
        };

        scenario.update_variables(&HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        });
    }
}
