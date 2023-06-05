mod result_acceptor;
use result_acceptor::*;

use std::iter::Peekable;

use serde_json::Value;

use crate::{
    tokenizer::{PropertyPathToken, RootPathToken, ScanPathToken, Token},
    JsonPathError, JsonPathResult,
};

pub struct Eval {
    result_acceptor: Box<dyn ResultAcceptor>,
}

impl Eval {
    pub fn new() -> Self {
        Eval {
            result_acceptor: Box::new(ScalarResultAcceptor::new()),
        }
    }
    pub fn eval(&mut self, json: &Value, tokens: Vec<Token>) -> JsonPathResult<Value> {
        let mut tokens = tokens.iter().peekable();

        match tokens.next() {
            Some(Token::Root(root)) => self.visit_root(root, json, &mut tokens)?,
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

        self.result_acceptor.result()
    }

    fn push_result(&mut self, value: Option<Value>) -> JsonPathResult<()> {
        self.result_acceptor.accept(value)
    }

    fn visit_next_token<'a>(
        &mut self,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        match tokens.next() {
            Some(Token::Root(_root)) => unimplemented!(),
            Some(Token::Property(property)) => self.visit_property(property, json, tokens),
            Some(Token::ArrayIndex(_)) => todo!(),
            Some(Token::ArrayPath(_)) => todo!(),
            Some(Token::ArraySlice(_)) => todo!(),
            Some(Token::Predicate(_)) => todo!(),
            Some(Token::Function(_)) => todo!(),
            Some(Token::Scan(scan)) => self.visit_scan(scan, json, tokens),
            Some(Token::Wildcard(_)) => todo!(),
            None => todo!(),
        }
    }

    fn visit_root<'a>(
        &mut self,
        _token: &RootPathToken,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        match tokens.peek() {
            None => self.push_result(Some(json.clone())),
            Some(_) => self.visit_next_token(json, tokens),
        }
    }

    fn visit_property<'a>(
        &mut self,
        token: &PropertyPathToken,
        object: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        let object = object
            .as_object()
            .ok_or(JsonPathError::EvaluationError(format!(
                "Expected to find an object with property {:?}",
                token
            )))?;

        if token.properties.len() > 1 {
            unimplemented!()
        }
        let prop = token.properties.iter().next().unwrap();
        match object.get(prop) {
            Some(v) => match tokens.peek() {
                None => self.push_result(Some(v.clone())),
                Some(_t) => self.visit_next_token(v, tokens),
            },
            None => self.push_result(None),
        }
    }
}

// visit ScanPathToken
impl Eval {
    /// upgrade the Eval to return array results
    fn use_array_result_register(&mut self) {
        if self.result_acceptor.is_scalar() {
            self.result_acceptor = Box::new(ArrayResultRegister::new())
        }
    }

    fn visit_scan<'a>(
        &mut self,
        _token: &ScanPathToken,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        if !json.is_array() && !json.is_object() {
            return Err(JsonPathError::EvaluationError(
                "Properties scan ('..') can only run on array or object values.".to_string(),
            ));
        }
        self.use_array_result_register();
        self.walk(json, tokens)
    }

    fn walk<'a>(
        &mut self,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        match json {
            Value::Object(_object) => self.walk_object(json, tokens),
            Value::Array(_array) => self.walk_array(json, tokens),
            _ => Ok(()),
        }
    }

    fn walk_object<'a>(
        &mut self,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        self.visit_next_token(json, &mut tokens.clone())?;
        let object = json.as_object().unwrap();
        for (_k, v) in object {
            self.walk(v, &mut tokens.clone())?;
        }
        Ok(())
    }

    fn walk_array<'a>(
        &mut self,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        let array = json.as_array().unwrap();
        for v in array {
            self.walk(v, &mut tokens.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{tokenizer::Tokenizer, JsonPathResult};

    use super::Eval;

    #[test]
    fn can_query_root_node() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$")?;
        let mut eval = Eval::new();
        let json = json!({"data": {"msg": "hello"}});
        let r = eval.eval(&json, tokens)?;
        assert_eq!(json, r);
        Ok(())
    }

    #[test]
    fn can_query_single_property() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$.data.msg")?;
        let mut eval = Eval::new();
        let r = eval.eval(&json!({"data": {"msg": "hello"}}), tokens)?;
        assert_eq!(json!("hello"), r);
        Ok(())
    }

    #[test]
    fn can_query_single_bracket_property() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$[\"data\"].msg")?;
        let mut eval = Eval::new();
        let r = eval.eval(
            &json!({"data": {"msg": "hello"}, "value": {"msg": "jsonpath"}}),
            tokens,
        )?;
        assert_eq!(json!("hello"), r);
        Ok(())
    }

    #[test]
    fn can_query_multiple_bracket_properties() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$['data','value'].msg")?;
        let mut eval = Eval::new();
        let r = eval.eval(&json!({"data": {"msg": "hello"}}), tokens)?;
        assert_eq!(json!(["hello", "jsonpath"]), r);
        Ok(())
    }

    #[test]
    fn can_scan_properties() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$.data..msg")?;
        let mut eval = Eval::new();
        let r = eval.eval(
            &json!({"data": {"item1": {"msg": "hello"}, "item2": {"msg": "jsonpath"}}}),
            tokens,
        )?;
        assert_eq!(json!(["hello", "jsonpath"]), r);
        Ok(())
    }

    #[test]
    fn can_scan_properties_with_arrays() -> JsonPathResult<()> {
        let tz = Tokenizer::new();
        let tokens = tz.tokenize("$.data..msg")?;
        let mut eval = Eval::new();
        let r = eval.eval(
            &json!({"data": {"items": [{"msg": "jsonpath"},  {"msg": "!"}], "msg": "hello"}}),
            tokens,
        )?;
        assert_eq!(json!(["hello", "jsonpath", "!"]), r);
        Ok(())
    }
}
