use crate::config;
use crate::function;
use crate::http_api::HttpRequest;
use crate::http_api::HttpResponse;
use crate::script;
use crate::script::ScriptContext;
use crate::variable::Value;
use crate::variable::Variable;
use http::Method;
use http::StatusCode;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone)]
pub struct Request {
    pub uri: String,
    pub uri_var_name: Vec<String>,
    pub method: Method,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
    pub body_var_name: Vec<String>,
    // pub body: Option<serde_json::Value>,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct Response {
    pub status: http::StatusCode,
    pub headers: Option<Vec<HeadersAssert>>,
    pub body: Option<Vec<BodyAssert>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeadersAssert {
    pub name: String,
    pub value: HeadersValueAssert,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum HeadersValueAssert {
    NotNull,
    Equal(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BodyAssert {
    pub name: String,
    pub value: BodyValueAssert,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value")]
pub enum BodyValueAssert {
    NotNull,
    EqualString(String),
    EqualNumber(f64),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseDefine {
    pub name: String,
    pub from: DefineFrom,
    pub path: String,
    pub function: Option<function::Function>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Copy, Clone)]
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
    pub pre_script: Option<Vec<script::Script>>,
    pub post_script: Option<Vec<script::Script>>,
}

impl<'a> Scenario<'a> {
    pub fn new(config: &config::Scenario, base_url: &str, global: &'a Global) -> Self {
        // Find variables in body and url
        let variable_pattern = Regex::new(r"\$\{([^}]+)\}").unwrap();

        // body
        let mut body_var_name = vec![];
        let body = match &config.request.body {
            Some(body) => {
                for caps in variable_pattern.captures_iter(&body) {
                    let cap = caps[1].to_string();
                    println!("Found global variable in body: {}", cap);
                    body_var_name.push(cap);
                }

                Some(body.to_string())
            }
            None => None,
        };

        // url
        let mut uri_var_name = vec![];
        for caps in variable_pattern.captures_iter(&config.request.path) {
            let cap = caps[1].to_string();
            println!("Found global variable in url: {}", cap);
            uri_var_name.push(cap);
        }

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
            uri_var_name,
            method: config.request.method.parse().unwrap(),
            headers: config.request.headers.clone(),
            body,
            body_var_name,
            timeout: config.request.timeout,
        };

        // Response
        let response = Response {
            status: StatusCode::from_u16(config.response.assert.status).unwrap(),
            headers: config.response.assert.headers.clone(),
            body: config.response.assert.body.clone(),
        };

        let pre_script = match &config.pre_script {
            Some(s) => {
                let mut scripts: Vec<script::Script> = vec![];
                for v in &s.variables {
                    let script = script::Script::new(v.clone());
                    scripts.push(script);
                }
                Some(scripts)
            }
            None => None,
        };

        let post_script = match &config.post_script {
            Some(s) => {
                let mut scripts: Vec<script::Script> = vec![];
                for v in &s.variables {
                    let script = script::Script::new(v.clone());
                    scripts.push(script);
                }
                Some(scripts)
            }
            None => None,
        };

        Scenario {
            name: config.name.clone(),
            base_url: base_url.into(),
            global,
            request,
            response,
            response_defines,
            assert_panic: true,
            pre_script,
            post_script,
        }
    }

    fn get_value(
        &self,
        name: &str,
        ctx: &ScriptContext,
        global: &Global,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        // Check context local
        let value = ctx.get_variable(name);
        if let Some(value) = value {
            return Ok(value.clone());
        }

        // Check global
        let value = global.get_variable_value(name);
        if let Some(value) = value {
            return Ok(value.clone());
        }

        Err(format!("Variable '{}' not found", name).into())
    }

    pub fn next_request2(
        &mut self,
        ctx: &ScriptContext,
    ) -> Result<HttpRequest, Box<dyn std::error::Error>> {
        let body = match &self.request.body {
            Some(body) => {
                let mut body = body.clone();

                // Apply vairables replace in body
                for name in &self.request.body_var_name {
                    let value = self.get_value(&name, ctx, self.global)?;
                    let value = match value {
                        Value::Int(v) => v.to_string(),
                        Value::String(v) => v,
                    };
                    body = body.replace(&format!("${{{}}}", name), &value);
                }

                Some(serde_json::from_str(&body).unwrap())
            }
            None => None,
        };

        let uri = {
            let mut uri = self.request.uri.clone();

            // Apply vairables replace in uri
            for name in &self.request.uri_var_name {
                let value = self.get_value(&name, ctx, self.global)?;
                let value = match value {
                    Value::Int(v) => v.to_string(),
                    Value::String(v) => v,
                };
                uri = uri.replace(&format!("${{{}}}", name), &value);
            }
            uri
        };

        // Add base_url to uri
        let uri = format!("{}{}", self.base_url, uri);

        Ok(HttpRequest {
            uri,
            method: self.request.method.clone(),
            headers: self.request.headers.clone(),
            body,
            timeout: self.request.timeout.clone(),
        })
    }

    pub fn assert_response(&self, response: &HttpResponse) -> bool {
        match self.check_response(response) {
            Ok(_) => true,
            Err(err) => {
                if self.assert_panic {
                    panic!("{}", err);
                } else {
                    log::error!("{}", err);
                }
                false
            }
        }
    }

    fn check_response(&self, response: &HttpResponse) -> Result<(), Box<dyn std::error::Error>> {
        // Check Status
        if self.response.status != response.status {
            return Err(format!(
                "Expected status code: {:?}, got: {:?}",
                self.response.status, response.status
            )
            .into());
        }

        // Check Headers
        if self.response.headers.is_some() {
            let headers = self.response.headers.as_ref().unwrap();
            for h in headers {
                let value = h.value.clone();
                let header = response
                    .headers
                    .get(&h.name)
                    .map(|hdr| hdr.to_str().unwrap());

                match value {
                    HeadersValueAssert::NotNull => {
                        if header.is_none() {
                            return Err(
                                format!("Header '{}' is expected but not found", h.name).into()
                            );
                        }
                    }
                    HeadersValueAssert::Equal(v) => {
                        if header.is_none() {
                            return Err(
                                format!("Header '{}' is expected but not found", h.name).into()
                            );
                        }
                        if header.unwrap() != v {
                            return Err(format!(
                                "Header '{}' is expected to be '{}' but got '{}'",
                                h.name,
                                v,
                                header.unwrap()
                            )
                            .into());
                        }
                    }
                }
            }
        }

        // Check Body
        if self.response.body.is_some() {
            let body_assert = self.response.body.as_ref().unwrap();

            let body = response.body.as_ref();
            if body == None {
                return Err("Body is expected but not found".into());
            }
            let body = body.unwrap();

            for b in body_assert {
                // Support nested json
                let keys = b.name.split('.').collect::<Vec<&str>>();
                let mut current = &mut body.clone(); // not sure how to get away without clone
                for key in keys.iter().take(keys.len() - 1) {
                    match current.get_mut(*key) {
                        Some(value) => {
                            current = value;
                        }
                        None => {
                            return Err(format!(
                                "Field '{}' is expected from body assert '{}' but not found",
                                key, b.name
                            )
                            .into());
                        }
                    }
                }

                let name_assert = keys.last().unwrap();
                let value_assert = b.value.clone();
                let value = current.get(name_assert);

                if value.is_none() {
                    return Err(format!("Field '{}' is expected but not found", b.name).into());
                }

                if value.unwrap().is_array() {
                    return Err("Asserting array fields in response body is not supported".into());
                }

                if value.unwrap().is_object() {
                    return Err("Error when parsing nested json in response body".into());
                }

                let value = value.unwrap();

                match value_assert {
                    BodyValueAssert::NotNull => {}
                    BodyValueAssert::EqualString(v) => {
                        if value.as_str().unwrap() != v {
                            return Err(format!(
                                "Body '{}' is expected to be '{}' but got '{}'",
                                b.name,
                                v,
                                value.as_str().unwrap()
                            )
                            .into());
                        }
                    }
                    BodyValueAssert::EqualNumber(v) => {
                        if value.is_f64() {
                            if value.as_f64().unwrap() != v {
                                return Err(format!(
                                    "Body '{}' is expected to be '{}' but got '{}'",
                                    b.name,
                                    v,
                                    value.as_f64().unwrap()
                                )
                                .into());
                            }
                        } else if value.is_i64() {
                            if value.as_i64().unwrap() as f64 != v {
                                return Err(format!(
                                    "Body '{}' is expected to be '{}' but got '{}'",
                                    b.name,
                                    v,
                                    value.as_i64().unwrap()
                                )
                                .into());
                            }
                        } else {
                            return Err(
                                format!("Body '{}' is expected to be number", b.name).into()
                            );
                        }
                    }
                }
            }
        }

        return Ok(());
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
                        };
                        values.push(value);
                    }
                }
                DefineFrom::Body => {
                    if let Some(body) = &response.body {
                        let value = jsonpath_lib::select(&body, &v.path).unwrap();
                        let value = value.get(0).unwrap();

                        log::debug!(
                            "Set local var from json field: '{}', name: '{}' value: '{}'",
                            v.path,
                            v.name,
                            value,
                        );

                        let value = if value.is_f64() {
                            Value::Int(value.as_f64().unwrap() as i32)
                        } else if value.is_i64() {
                            Value::Int(value.as_i64().unwrap() as i32)
                        } else {
                            Value::String(value.as_str().unwrap().to_string())
                        };

                        let value = Variable {
                            name: v.name.clone(),
                            value,
                        };
                        values.push(value);
                    }
                }
            }
        }

        values
    }

    pub fn run_pre_script(&self, ctx: &mut ScriptContext) {
        log::debug!("run_pre_script");

        if let Some(script) = &self.pre_script {
            for s in script {
                s.exec2(ctx, &self.global).unwrap();
            }
        }

        // print all variables from context
        for (k, v) in ctx.local.variables.iter() {
            log::debug!("pre context variable: {} = {:?}", k, v);
        }
    }

    pub fn run_post_script(&self, ctx: &mut ScriptContext) {
        log::debug!("run_post_script");

        if let Some(script) = &self.post_script {
            for s in script {
                s.exec2(ctx, &self.global).unwrap();
            }
        }

        // print all variables from context
        for (k, v) in ctx.local.variables.iter() {
            log::debug!("post context variable: {} = {:?}", k, v);
        }
    }
}

pub struct Global {
    // TODO: Change to HashMap
    pub variables: Vec<Arc<Mutex<Variable>>>,
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

    pub fn add_variable(&mut self, variable: Variable) {
        self.variables.push(Arc::new(Mutex::new(variable)));
    }

    pub fn get_variable_value(&self, variable_name: &str) -> Option<Value> {
        for v in &self.variables {
            let variable = v.lock().unwrap();
            if variable.name == variable_name {
                return Some(variable.value.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_next_request2() {
        let global = Global { variables: vec![] };
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        // TODO: test this as well
        let uri_var_name = vec!["foo_id".into()];
        let body_var_name = vec!["var1".into(), "var2".into()];

        let mut scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint/foo/${foo_id}".into(),
                uri_var_name,
                method: Method::GET,
                headers: Some(vec![headers]),
                body: Some(r#"{"test": "${var1}_${var2}"}"#.into()),
                body_var_name,
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        let mut ctx = ScriptContext::new();
        ctx.set_variable("var1", Value::Int(0));
        ctx.set_variable("var2", Value::Int(100));
        ctx.set_variable("foo_id", Value::String("1-2-3-4".into()));

        let request = scenario.next_request2(&ctx).unwrap();
        assert_eq!(request.uri, "http://localhost:8080/endpoint/foo/1-2-3-4");
        assert_eq!(request.method, Method::GET);
        assert_eq!(
            request.body,
            Some(serde_json::from_str(r#"{"test": "0_100"}"#).unwrap())
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
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
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
    fn test_scenario_check_response_with_body() {
        let global = Global { variables: vec![] };
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: Some(vec![HeadersAssert {
                    name: "Content-Type".into(),
                    value: HeadersValueAssert::NotNull,
                }]),
                body: Some(vec![BodyAssert {
                    name: "Result".into(),
                    value: BodyValueAssert::EqualNumber(0.0),
                }]),
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        // Missing content-type header
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Header 'Content-Type' is expected but not found",
                err.to_string()
            ),
        }

        // Missing response body
        let mut headers = http::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: None,
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!("Body is expected but not found", err.to_string()),
        }

        // Missing field 'Result' in response body
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!("Field 'Result' is expected but not found", err.to_string()),
        }

        // Mismatch value in response body
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"Result": 1, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Body 'Result' is expected to be '0' but got '1'",
                err.to_string()
            ),
        }

        // All good
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: Some(serde_json::from_str(r#"{"Result": 0, "ObjectId": "0-1-2-3"}"#).unwrap()),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }
    }

    #[test]
    fn test_scenario_check_response_with_nested_body() {
        let global = Global { variables: vec![] };
        let scenario = Scenario {
            name: "Scenario_1".into(),
            base_url: "http://localhost:8080".into(),
            global: &global,
            request: Request {
                uri: "/endpoint".into(),
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: Some(vec![BodyAssert {
                    name: "Foo.Bar".into(),
                    value: BodyValueAssert::EqualString("Baz".into()),
                }]),
            },
            response_defines: vec![],
            assert_panic: false,
            pre_script: None,
            post_script: None,
        };

        // Test Missing Field 'Foo'
        let body = serde_json::json!({
            "Result": 0,
            "Bar": "Baz"
        });

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Some(body),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => panic!("Expected error"),
            Err(err) => assert_eq!(
                "Field 'Foo' is expected from body assert 'Foo.Bar' but not found",
                err.to_string()
            ),
        }

        // ALl Good
        let body = serde_json::json!({
            "Result": 0,
            "Foo": {
                "Bar": "Baz"
            }
        });

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: Some(body),
            request_start: std::time::Instant::now(),
            retry_count: 0,
        };

        match scenario.check_response(&response) {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }
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
                uri_var_name: vec![],
                method: Method::GET,
                headers: None,
                body: None,
                body_var_name: vec![],
                timeout: Duration::from_secs(3),
            },
            response: Response {
                status: StatusCode::OK,
                headers: None,
                body: None,
            },
            response_defines,
            assert_panic: false,
            pre_script: None,
            post_script: None,
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
