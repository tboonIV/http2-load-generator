use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub log_level: LogLevel,
    pub parallel: u8,
    pub runner: RunnerConfig,
}

#[derive(Debug, Deserialize, PartialEq, Copy, Clone)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Into<log::LevelFilter> for LogLevel {
    fn into(self) -> log::LevelFilter {
        match self {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RunnerConfig {
    pub target_rps: u32,
    #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub duration: Duration,
    pub batch_size: BatchSize,
    // pub auto_throttle: bool,
    pub base_url: String,
    pub global: Global,
    // #[serde(deserialize_with = "humantime_duration_deserializer")]
    // pub delay_between_scenario: Duration,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum BatchSize {
    Auto(String),
    Fixed(u32),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Global {
    pub variables: Vec<Variable>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Variable {
    pub name: String,
    pub function: Function,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Function {
    Incremental(IncrementalFunction),
    Random(RandomFunction),
    Split(SplitFunction),
    // ThreadId,
    // RunnerId,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct IncrementalFunction {
    pub start: i32,
    pub threshold: i32,
    pub step: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct RandomFunction {
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SplitFunction {
    pub delimiter: String,
    pub index: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Scenario {
    pub name: String,
    pub request: Request,
    pub response: Response,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Option<Vec<HashMap<String, String>>>,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Response {
    pub assert: ResponseAssert,
    pub define: Option<Vec<ResponseDefine>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResponseAssert {
    pub status: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResponseDefine {
    pub name: String,
    pub from: DefineFrom,
    pub path: String,
    pub function: Option<Function>,
}

#[derive(Debug, Deserialize, PartialEq, Copy, Clone)]
pub enum DefineFrom {
    Header,
    Body,
}

pub fn read_yaml_file(path: &str) -> Result<Config, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let value: Value = serde_yaml::from_str(&contents)?;
    let config: Config = serde_yaml::from_value(value)?;
    Ok(config)
}

fn humantime_duration_deserializer<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    humantime::parse_duration(&s).map_err(|e| serde::de::Error::custom(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serde() {
        let yaml_str = r#"
        log_level: "Debug"
        parallel: 1
        runner:
          target_rps: 100
          duration: 10s
          batch_size: 5
          # auto_throttle: true
          base_url: "http://localhost:8080/"
          global:
            variables:
              - name: COUNTER
                type: Incremental
                function:
                  type: Incremental
                  start: 0
                  threshold: 100
                  step: 1
              - name: RANDOM
                type: Random
                function:
                  type: Random
                  min: 0
                  max: 100
          # delay_between_scenario: 500ms
          scenarios:
            - name: createSubscriber
              request:
                method: POST
                path: "/rsgateway/data/json/subscriber"
                headers:
                - content-type: "application/json"
                body: |
                  {
                    "$": "MtxRequestSubscriberCreate",
                    "Name": "James Bond",
                    "FirstName": "James",
                    "LastName": "Bond",
                    "ContactEmail": "james.bond@email.com"
                  }
              response:
                assert:
                  status: 200
                define:
                  - name: externalId
                    from: Body
                    path: "$.ObjectId"
            - name: querySubscriber
              request:
                method: GET
                path: "/rsgateway/data/json/subscriber/query/ExternalId/${externalId}"
              response:
                assert:
                  status: 200
    "#;
        let config: Config = serde_yaml::from_str(yaml_str).unwrap();

        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.parallel, 1);
        assert_eq!(config.runner.target_rps, 100);
        assert_eq!(config.runner.duration, Duration::from_secs(10));
        assert_eq!(config.runner.batch_size, BatchSize::Fixed(5));
        assert_eq!(config.runner.base_url, "http://localhost:8080/".to_string());
        assert_eq!(config.runner.global.variables.len(), 2);
        assert_eq!(config.runner.global.variables[0].name, "COUNTER");
        assert_eq!(
            config.runner.global.variables[0].function,
            Function::Incremental(IncrementalFunction {
                start: 0,
                threshold: 100,
                step: 1,
            })
        );
        assert_eq!(config.runner.global.variables[1].name, "RANDOM");
        assert_eq!(
            config.runner.global.variables[1].function,
            Function::Random(RandomFunction { min: 0, max: 100 })
        );
        assert_eq!(config.runner.scenarios.len(), 2);
        assert_eq!(config.runner.scenarios[0].name, "createSubscriber");
        assert_eq!(config.runner.scenarios[0].request.method, "POST");
        assert_eq!(
            config.runner.scenarios[0].request.path,
            "/rsgateway/data/json/subscriber"
        );
        assert_eq!(
            config.runner.scenarios[0].request.headers.as_ref().unwrap()[0]["content-type"],
            "application/json"
        );
        assert_eq!(
            config.runner.scenarios[0].request.body,
            Some(
                r#"{
  "$": "MtxRequestSubscriberCreate",
  "Name": "James Bond",
  "FirstName": "James",
  "LastName": "Bond",
  "ContactEmail": "james.bond@email.com"
}
"#
                .to_string()
            )
        );
        assert_eq!(config.runner.scenarios[0].response.assert.status, 200);
        assert_eq!(config.runner.scenarios[1].name, "querySubscriber");
        assert_eq!(config.runner.scenarios[1].request.method, "GET");
        assert_eq!(
            config.runner.scenarios[1].request.path,
            "/rsgateway/data/json/subscriber/query/ExternalId/${externalId}"
        );
        assert_eq!(config.runner.scenarios[1].request.headers, None);
        assert_eq!(config.runner.scenarios[1].request.body, None);
        assert_eq!(config.runner.scenarios[1].response.assert.status, 200);
    }
}
