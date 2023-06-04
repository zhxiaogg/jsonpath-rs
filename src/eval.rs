use std::iter::Peekable;

use serde_json::Value;

use crate::{
    tokenizer::{PropertyPathToken, RootPathToken, Token},
    JsonPathError, JsonPathResult,
};

pub struct Eval<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
}

impl<I: Iterator<Item = Token>> Eval<I> {
    pub fn eval(&mut self, json: &Value) -> JsonPathResult<Value> {
        match self.tokens.peek() {
            Some(Token::Root(..)) => {}
            None => {
                return Err(JsonPathError::EvaluationError(
                    "Empty jsonpath provided".to_string(),
                ))
            }
            Some(_) => {
                return Err(JsonPathError::EvaluationError(
                    "Invalid start token for the given jsonpath".to_string(),
                ))
            }
        }
        let mut object = json;
        while let Some(token) = self.tokens.next() {
            match token {
                Token::Root(root) => self.visit_root(&root, object),
                Token::Property(property) => object = self.visit_property(&property, object)?,
                t => {
                    return Err(JsonPathError::EvaluationError(format!(
                        "Unexpected token: {:?}",
                        t
                    )))
                }
            }
        }

        Ok(object.clone())
    }

    fn visit_root(&mut self, token: &RootPathToken, json: &Value) {}

    fn visit_property<'a>(
        &mut self,
        token: &PropertyPathToken,
        object: &'a Value,
    ) -> JsonPathResult<&'a Value> {
        match object {
            Value::Object(object) => {
                if token.properties.len() > 1 {
                    unimplemented!()
                }
                let prop = token.properties.iter().next().unwrap();
                match object.get(prop) {
                    Some(v) => Ok(v),
                    None => unimplemented!(),
                }
            }
            _ => {
                return Err(JsonPathError::EvaluationError(format!(
                    "Expected to find an object with property {:?}",
                    token
                )))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{tokenizer::Tokenizer, JsonPathResult};

    use super::Eval;

    #[test]
    fn can_evaluate_property_queries() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$.data.msg")?;
        let mut eval = Eval {
            tokens: tokens.into_iter().peekable(),
        };
        let r = eval.eval(&json!({"data": {"msg": "hello"}}))?;
        assert_eq!(json!("hello"), r);
        Ok(())
    }
}
