use std::fmt::Display;

pub type JsonPathResult<T> = Result<T, JsonPathError>;

#[derive(Debug, PartialEq)]
pub enum JsonPathError {
    InvalidJsonPath(String),
    EvaluationError(String),
}

impl Display for JsonPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonPathError::InvalidJsonPath(e) => {
                f.write_fmt(format_args!("Invalid JsonPath: {}", e))
            }
            JsonPathError::EvaluationError(e) => {
                f.write_fmt(format_args!("JsonPath evaluation error: {}", e))
            }
        }
    }
}
