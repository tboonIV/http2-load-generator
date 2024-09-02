use std::fmt;

#[derive(Debug)]
pub enum Error {
    ScriptError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ScriptError(e) => write!(f, "Script error: {}", e),
        }
    }
}

impl std::error::Error for Error {}
