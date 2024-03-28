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
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BatchSize {
    Auto(String),
    Fixed(u32),
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
    fn fixed_batch_size() {
        let yaml_str = r#"
        target_tps: 100
        duration: 10
        parallel: 1
        batch_size: 5
    "#;
        let config: Config = serde_yaml::from_str(yaml_str).unwrap();

        assert_eq!(config.target_tps, 100);
        assert_eq!(config.duration, 10);
        assert_eq!(config.parallel, 1);
        assert_eq!(config.batch_size, BatchSize::Fixed(5));
    }

    #[test]
    fn auto_batch_size() {
        let yaml_str = r#"
        target_tps: 15000
        duration: 600
        parallel: 8
        batch_size: "Auto"
    "#;
        let config: Config = serde_yaml::from_str(yaml_str).unwrap();

        assert_eq!(config.target_tps, 15000);
        assert_eq!(config.duration, 600);
        assert_eq!(config.parallel, 8);
        assert_eq!(config.batch_size, BatchSize::Auto("Auto".to_string()));
    }
}
