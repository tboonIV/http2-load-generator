use serde::Deserialize;

// Future Features:
// Support simple scripting language similiar to Karate
//
// def location = responseHeaders.location[0]
// def chargingDataRef = location.substring(location.lastIndexOf('/') + 1)
//
// def count = 0
// def count = count + 1
//
#[derive(Debug, Deserialize, Clone)]
pub struct Scripting {
    pub raw: String,
}
