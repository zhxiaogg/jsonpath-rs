use serde_json::Value;

use crate::{JsonPathError, JsonPathResult};

pub trait ResultAcceptor {
    fn accept(&mut self, result: Option<Value>) -> JsonPathResult<()>;
    fn result(&mut self) -> JsonPathResult<Value>;
    fn is_scalar(&self) -> bool;
}

pub struct ScalarResultAcceptor {
    result: Option<Value>,
}

impl ScalarResultAcceptor {
    pub fn new() -> Self {
        Self { result: None }
    }
}

impl ResultAcceptor for ScalarResultAcceptor {
    fn accept(&mut self, result: Option<Value>) -> JsonPathResult<()> {
        if self.result.is_some() {
            return Err(JsonPathError::EvaluationError(
                "Invalid state, there is already a scalar result.".to_string(),
            ));
        }
        self.result = result;
        Ok(())
    }

    fn result(&mut self) -> JsonPathResult<Value> {
        match self.result.take() {
            Some(v) => Ok(v),
            None => Ok(Value::Null),
        }
    }

    fn is_scalar(&self) -> bool {
        true
    }
}

pub struct ArrayResultRegister {
    result: Vec<Value>,
}

impl ArrayResultRegister {
    pub fn new() -> Self {
        Self { result: vec![] }
    }
}

impl ResultAcceptor for ArrayResultRegister {
    fn accept(&mut self, result: Option<Value>) -> JsonPathResult<()> {
        match result {
            Some(r) => self.result.push(r),
            None => {}
        }
        Ok(())
    }

    fn result(&mut self) -> JsonPathResult<Value> {
        Ok(Value::Array(self.result.clone()))
    }

    fn is_scalar(&self) -> bool {
        false
    }
}
