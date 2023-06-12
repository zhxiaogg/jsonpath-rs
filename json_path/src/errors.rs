use std::{error::Error, fmt::Display};

use peekmore::PeekMoreError;

pub type JsonPathResult<T> = Result<T, JsonPathError>;

#[derive(Debug, PartialEq)]
pub enum JsonPathError {
    InvalidJsonPath(String, usize),
    EvaluationError(String),
}

impl Error for JsonPathError {}

impl Display for JsonPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonPathError::InvalidJsonPath(e, pos) => {
                f.write_fmt(format_args!("Invalid JsonPath: {} at {}", e, pos))
            }
            JsonPathError::EvaluationError(e) => {
                f.write_fmt(format_args!("JsonPath evaluation error: {}", e))
            }
        }
    }
}

impl From<PeekMoreError> for JsonPathError {
    fn from(_value: PeekMoreError) -> Self {
        JsonPathError::InvalidJsonPath("Unexpected token".to_string(), 0)
    }
}
