pub type JsonPathResult<T> = Result<T, JsonPathError>;

#[derive(Debug)]
pub enum JsonPathError {
    Unknown,
    InvalidJsonPath(String),
    EvaluationError(String),
}
