use serde::Deserialize;
use serde_yaml::Value;
use std::error::Error;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub target_tps: u32,
    pub duration: u32,
    pub parallel: u8,
    pub batch_size: BatchSize,
    pub base_url: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BatchSize {
    Auto(String),
    Fixed(u32),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serde() {
        let yaml_str = r#"
        target_tps: 100
        duration: 10
        parallel: 1
        batch_size: 5
        base_url: "http://localhost:8080/"
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

        assert_eq!(config.target_tps, 100);
        assert_eq!(config.duration, 10);
        assert_eq!(config.parallel, 1);
        assert_eq!(config.batch_size, BatchSize::Fixed(5));
        assert_eq!(config.base_url, "http://localhost:8080/".to_string());
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