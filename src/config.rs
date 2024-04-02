use serde::Deserialize;
use serde_yaml::Value;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_level: LogLevel,
    pub target_tps: u32,
    #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub duration: Duration,
    pub parallel: u8,
    pub batch_size: BatchSize,
    pub auto_throttle: bool,
    pub base_url: String,
    pub variables: Vec<Variable>,
    #[serde(deserialize_with = "humantime_duration_deserializer")]
    pub delay_between_scenario: Duration,
    pub scenarios: Vec<Scenario>,
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BatchSize {
    Auto(String),
    Fixed(u32),
}

#[derive(Debug, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
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
        target_tps: 100
        duration: 10s
        parallel: 1
        batch_size: 5
        auto_throttle: true
        base_url: "http://localhost:8080/"
        variables:
          - name: COUNTER
            type: incremental
        delay_between_scenario: 500ms
        scenarios:
          - name: createSubscriber
            method: POST
            path: "/rsgateway/data/json/subscriber"
            body: |
              {
                "$": "MtxRequestSubscriberCreate",
                "Name": "James Bond",
                "FirstName": "James",
                "LastName": "Bond",
                "ContactEmail": "james.bond@email.com"
              }
          - name: querySubscriber
            method: GET
            path: "/rsgateway/data/json/subscriber/query/ExternalId/:externalId"
    "#;
        let config: Config = serde_yaml::from_str(yaml_str).unwrap();

        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.target_tps, 100);
        assert_eq!(config.duration, Duration::from_secs(10));
        assert_eq!(config.parallel, 1);
        assert_eq!(config.batch_size, BatchSize::Fixed(5));
        assert_eq!(config.auto_throttle, true);
        assert_eq!(config.base_url, "http://localhost:8080/".to_string());
        assert_eq!(config.variables.len(), 1);
        assert_eq!(config.variables[0].name, "COUNTER");
        assert_eq!(config.variables[0].variable_type, "incremental");
        assert_eq!(config.delay_between_scenario, Duration::from_millis(500));
        assert_eq!(config.scenarios.len(), 2);
        assert_eq!(config.scenarios[0].name, "createSubscriber");
        assert_eq!(config.scenarios[0].method, "POST");
        assert_eq!(config.scenarios[0].path, "/rsgateway/data/json/subscriber");
        assert_eq!(
            config.scenarios[0].body,
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
        assert_eq!(config.scenarios[1].name, "querySubscriber");
        assert_eq!(config.scenarios[1].method, "GET");
        assert_eq!(
            config.scenarios[1].path,
            "/rsgateway/data/json/subscriber/query/ExternalId/:externalId"
        );
        assert_eq!(config.scenarios[1].body, None);
    }
}
