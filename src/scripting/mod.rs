use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Scripting {
    pub raw: String,
}
