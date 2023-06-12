mod result_acceptor;
use result_acceptor::*;

use std::iter::Peekable;

use serde_json::{Map, Value};

use crate::{
    tokenizer::{
        ArraySlice, Comparator, Expression, PropertyPathToken, RootPathToken, ScanPathToken, Token,
    },
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
    pub fn eval(&mut self, json: &Value, tokens: impl AsRef<Vec<Token>>) -> JsonPathResult<Value> {
        let mut tokens = tokens.as_ref().iter().peekable();

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
            Some(Token::ArrayIndex { indices }) => self.visit_array_index(indices, json, tokens),
            Some(Token::ArraySlice(array_slice)) => {
                self.visit_array_slice(array_slice, json, tokens)
            }
            Some(Token::Predicate(expression)) => self.visit_predicate(expression, json, tokens),
            Some(Token::Function(_)) => todo!(),
            Some(Token::Scan(scan)) => self.visit_scan(scan, json, tokens),
            Some(Token::Wildcard) => self.visit_wildchard(json, tokens),
            None => Ok(()),
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
        let object = match object {
            Value::Object(object) => object,
            _ => return Ok(()),
        };

        if token.properties.len() > 1 {
            match tokens.peek() {
                None => {
                    // this is a leaf token, will merge properties into one object
                    let mut result = Map::new();
                    for prop in token.properties.iter() {
                        match object.get(prop) {
                            Some(v) => result.insert(prop.to_string(), v.clone()),
                            // TODO: differentiate undefined and null with options
                            None => result.insert(prop.to_string(), Value::Null),
                        };
                    }
                    self.push_result(Some(Value::Object(result)))
                }
                Some(_) => {
                    // this is a multi property iteration
                    self.use_array_result_register();

                    for prop in token.properties.iter() {
                        self.handle_object_property(prop, object, &mut tokens.clone())?;
                    }
                    Ok(())
                }
            }
        } else {
            // single property query
            let prop = token.properties.first().unwrap();
            self.handle_object_property(prop, object, tokens)
        }
    }

    fn handle_object_property<'a>(
        &mut self,
        prop: &String,
        object: &Map<String, Value>,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        match object.get(prop) {
            Some(v) => match tokens.peek() {
                None => self.push_result(Some(v.clone())),
                Some(_) => self.visit_next_token(v, tokens),
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
        self.visit_next_token(json, &mut tokens.clone())?;
        match json {
            Value::Object(object) => {
                for (_k, v) in object {
                    self.walk(v, &mut tokens.clone())?;
                }
            }
            Value::Array(array) => {
                for v in array {
                    self.walk(v, &mut tokens.clone())?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// visit array index/slice token
impl Eval {
    fn visit_array_index<'a>(
        &mut self,
        indices: &Vec<i32>,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        let array = json.as_array().ok_or(JsonPathError::EvaluationError(
            "Running array index op on non-array object".to_string(),
        ))?;

        if indices.is_empty() {
            Err(JsonPathError::EvaluationError(
                "Invalid array index token, zero index given.".to_string(),
            ))
        } else if indices.len() == 1 {
            let index = *indices.first().unwrap();
            self.handle_array_index(array, index, tokens)
        } else {
            self.use_array_result_register();
            for index in indices {
                self.handle_array_index(array, *index, &mut tokens.clone())?;
            }
            Ok(())
        }
    }

    fn handle_array_index<'a>(
        &mut self,
        array: &Vec<Value>,
        mut index: i32,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        if index < 0 {
            // TODO: revisit the cast here
            index += array.len() as i32;
        }
        if index >= 0 && index < array.len() as i32 {
            let value = array.get(index as usize).unwrap();
            match tokens.peek() {
                None => self.push_result(Some(value.clone())),
                Some(_t) => self.visit_next_token(value, tokens),
            }
        } else {
            Ok(())
        }
    }

    fn visit_array_slice<'a>(
        &mut self,
        slice: &ArraySlice,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        let array = json.as_array().ok_or(JsonPathError::EvaluationError(
            "Running array index op on non-array object".to_string(),
        ))?;
        self.use_array_result_register();
        match slice {
            ArraySlice::From(from) => {
                let mut start = *from;
                if start < 0 {
                    start = (array.len() as i32 + start).max(0);
                }
                for index in start..array.len() as i32 {
                    self.handle_array_index(array, index, &mut tokens.clone())?;
                }
                Ok(())
            }
            ArraySlice::To(to) => {
                let mut end = *to;
                if end < 0 {
                    end += array.len() as i32;
                }
                for index in 0..end {
                    self.handle_array_index(array, index, &mut tokens.clone())?;
                }
                Ok(())
            }
            ArraySlice::Between(from, to) => {
                let mut start = *from;
                let mut end = *to;
                if end < 0 {
                    // scenario for eg. [0:-1]
                    end += array.len() as i32;
                }
                if start < 0 {
                    // scenario for eg. [0:-1]
                    start = (array.len() as i32 + start).max(0);
                }
                if start < end && !array.is_empty() {
                    for index in start..end {
                        self.handle_array_index(array, index, &mut tokens.clone())?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl Eval {
    fn visit_wildchard<'a>(
        &mut self,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        self.use_array_result_register();
        match json {
            Value::Array(array) => {
                for index in 0..array.len() {
                    self.handle_array_index(array, index as i32, &mut tokens.clone())?;
                }
            }
            Value::Object(object) => {
                for prop in object.keys() {
                    self.handle_object_property(prop, object, &mut tokens.clone());
                }
            }
            _ => {
                return Err(JsonPathError::EvaluationError(
                    "Expect array or object for wildcard query.".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Eval {
    fn visit_predicate<'a>(
        &mut self,
        expression: &Expression,
        json: &Value,
        tokens: &mut Peekable<impl Iterator<Item = &'a Token> + Clone>,
    ) -> JsonPathResult<()> {
        let result = self.eval_expr(expression, json)?;
        let bool = Self::get_bool(result);
        match (bool, tokens.peek()) {
            (true, None) => self.push_result(Some(json.clone())),
            (true, Some(_)) => self.visit_next_token(json, tokens),
            _ => Ok(()),
        }
    }

    fn get_bool(value: Value) -> bool {
        match value {
            Value::Bool(b) => b,
            Value::Null => false,
            _ => true,
        }
    }

    fn eval_expr(&self, expression: &Expression, json: &Value) -> JsonPathResult<Value> {
        let result = match expression {
            Expression::JsonQuery(tokens) => {
                let mut eval = Eval::new();
                // TODO: support jsonpath query on the root object (using $)
                eval.eval(json, tokens)?
            }
            Expression::Literal(v) => v.clone(),
            Expression::Not(inner) => {
                let r = self.eval_expr(inner, json)?;
                match r {
                    Value::Bool(b) => Value::Bool(!b),
                    Value::Null => Value::Bool(true),
                    _ => Value::Bool(false),
                }
            }
            Expression::Array(v) => {
                let values = v
                    .iter()
                    .map(|e| self.eval_expr(e, json))
                    .collect::<JsonPathResult<Vec<Value>>>()?;
                Value::Array(values)
            }
            Expression::CompareExpr { op, left, right } => {
                let left = self.eval_expr(left, json)?;
                let right = self.eval_expr(right, json)?;
                let result = match op {
                    Comparator::Eq => left.eq(&right),
                    Comparator::Neq => !left.eq(&right),
                    Comparator::Gt => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() > r.as_f64(),
                        _ => false,
                    },
                    Comparator::GtEq => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() >= r.as_f64(),
                        _ => false,
                    },
                    Comparator::Lt => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() < r.as_f64(),
                        _ => false,
                    },
                    Comparator::LtEq => match (left, right) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() <= r.as_f64(),
                        _ => false,
                    },
                    Comparator::RegExpMatch => todo!(), // TODO: implement this
                    Comparator::AND => Self::get_bool(left) && Self::get_bool(right),
                    Comparator::OR => Self::get_bool(left) || Self::get_bool(right),
                    Comparator::IN => match right {
                        Value::Array(values) => values.contains(&left),
                        _ => false,
                    },
                    Comparator::NIN => match right {
                        Value::Array(values) => !values.contains(&left),
                        _ => false,
                    },
                    Comparator::SubsetOf => match (left, right) {
                        (Value::Array(l), Value::Array(r)) => l.iter().all(|c| r.contains(c)),
                        _ => false,
                    },
                    Comparator::AnyOf => match (left, right) {
                        (Value::Array(l), Value::Array(r)) => l.iter().any(|c| r.contains(c)),
                        _ => false,
                    },
                    Comparator::NoneOf => match (left, right) {
                        (Value::Array(l), Value::Array(r)) => !l.iter().any(|c| r.contains(c)),
                        _ => false,
                    },
                    Comparator::Contains => match (left, right) {
                        (Value::Array(values), r) => values.contains(&r),
                        (Value::String(l), Value::String(r)) => l.contains(&r),
                        _ => false,
                    },
                    Comparator::SizeOf => match (left, right) {
                        (Value::Array(values), Value::Number(n)) => {
                            values.len() as i64 == n.as_i64().unwrap_or(-1)
                        }
                        (Value::String(s), Value::Number(n)) => {
                            s.len() as i64 == n.as_i64().unwrap_or(-1)
                        }
                        _ => false,
                    },
                    Comparator::Empty => match (left, right) {
                        (Value::Array(values), Value::Bool(b)) => values.is_empty() == b,
                        (Value::String(s), Value::Bool(b)) => s.is_empty() == b,
                        (Value::Null, Value::Bool(b)) => b,
                        _ => false,
                    },
                };
                Value::Bool(result)
            }
        };
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use crate::{tokenizer::Tokenizer, JsonPathResult};

    use super::Eval;

    pub trait JsonPathQuery {
        fn query(&self, json_path: &str) -> JsonPathResult<Value>;
    }

    impl JsonPathQuery for Value {
        fn query(&self, json_path: &str) -> JsonPathResult<Value> {
            let tz = Tokenizer::new();
            let tokens = tz.tokenize(json_path)?;
            let mut eval = Eval::new();
            eval.eval(self, tokens)
        }
    }

    #[test]
    fn can_query_root_node() {
        let json = json!({"data": {"msg": "hello"}});
        assert_eq!(json.query("$"), Ok(json));
    }

    #[test]
    fn can_query_single_property() {
        assert_eq!(
            Ok(json!("hello")),
            json!({"data": {"msg": "hello"}}).query("$.data.msg")
        );
    }

    #[test]
    fn can_query_single_bracket_property() {
        assert_eq!(
            Ok(json!("hello")),
            json!({"data": {"msg": "hello"}}).query("$[\"data\"].msg")
        )
    }

    #[test]
    fn can_query_multiple_bracket_properties() {
        assert_eq!(
            Ok(json!(["hello", "jsonpath"])),
            json!({"data": {"msg": "hello"}, "value": {"msg": "jsonpath"}})
                .query("$['data','value'].msg")
        );
    }

    #[test]
    fn can_query_and_merge_multiple_bracket_properties() {
        let json = json!({"data": {"msg1": "hello", "msg2": "jsonpath", "msg3": "xxx"}});
        assert_eq!(
            Ok(json!({"msg1": "hello", "msg2": "jsonpath"})),
            json.query("$.data['msg1','msg2']")
        )
    }

    #[test]
    fn can_scan_properties() {
        let json = json!({"data": {"item1": {"msg": "hello"}, "item2": {"msg": "jsonpath"}}});
        assert_eq!(Ok(json!(["hello", "jsonpath"])), json.query("$.data..msg"))
    }

    #[test]
    fn can_scan_properties_with_arrays() {
        let json = json!({"data": {"items": [{"msg": "jsonpath"},  {"msg": "!"}], "msg": "hello"}});
        assert_eq!(
            Ok(json!(["hello", "jsonpath", "!"])),
            json.query("$.data..msg")
        )
    }

    #[test]
    fn support_array_index_with_single_index() {
        let json = json!({"data": ["item 0", "item 1", "item 2"]});
        assert_eq!(Ok(json!("item 0")), json.query("$.data[0]"));
        assert_eq!(Ok(json!("item 1")), json.query("$.data[1]"));
        assert_eq!(Ok(json!("item 2")), json.query("$.data[2]"));
        assert_eq!(Ok(Value::Null), json.query("$.data[3]"));
        assert_eq!(Ok(json!("item 2")), json.query("$.data[-1]"));
        assert_eq!(Ok(json!("item 1")), json.query("$.data[-2]"));
        assert_eq!(Ok(json!("item 0")), json.query("$.data[-3]"));
        assert_eq!(Ok(Value::Null), json.query("$.data[-4]"));
    }

    #[test]
    fn support_array_index_with_empty_array() {
        let json = json!({"data": []});
        assert_eq!(Ok(Value::Null), json.query("$.data[0]"));
    }

    #[test]
    fn support_array_index_with_multiple_indices() {
        let json = json!({"data": ["item 0", "item 1", "item 2"]});
        assert_eq!(Ok(json!(["item 0", "item 2"])), json.query("$.data[0,2]"));
        assert_eq!(Ok(json!(["item 0", "item 2"])), json.query("$.data[0,-1]"));
        assert_eq!(Ok(json!(["item 1", "item 1"])), json.query("$.data[1,1]"));
    }

    #[test]
    fn support_array_slice() {
        let json = json!({"data": ["item 0", "item 1", "item 2"]});
        assert_eq!(
            Ok(json!(["item 0", "item 1", "item 2"])),
            json.query("$.data[0:3]")
        );
        assert_eq!(
            Ok(json!(["item 0", "item 1", "item 2"])),
            json.query("$.data[:3]")
        );
        assert_eq!(Ok(json!(["item 1", "item 2"])), json.query("$.data[1:]"));
        assert_eq!(Ok(json!(["item 0", "item 1"])), json.query("$.data[0:-1]"));
        assert_eq!(Ok(json!(["item 0", "item 1"])), json.query("$.data[:-1]"));
        assert_eq!(Ok(json!(["item 0", "item 1"])), json.query("$.data[-5:-1]"));
        assert_eq!(
            Ok(json!(["item 0", "item 1", "item 2"])),
            json.query("$.data[-5:]")
        );
    }

    #[test]
    fn support_wildcard_query_on_objects() {
        let json = json!({"data": {"0": {"msg": "item 0"}, "1": {"msg": "item 1"}}});
        assert_eq!(Ok(json!(["item 0", "item 1"])), json.query("$.data[*].msg"));
        let json =
            json!({"data": {"0": {"msg": {"msg": "item 0"}}, "1": {"msg": {"msg": "item 1"}}}});
        assert_eq!(
            Ok(json!(["item 0", "item 1"])),
            json.query("$.data[*].msg.msg")
        );
        assert_eq!(
            Ok(json!(["item 0", "item 1"])),
            json.query("$.data.*.msg.msg")
        );
    }

    #[test]
    fn support_wildcard_query_on_arrays() {
        let json = json!({"data": [{"msg": "item 0"}, {"msg": "item 1"}]});
        assert_eq!(Ok(json!(["item 0", "item 1"])), json.query("$.data[*].msg"));
        let json = json!({"data": [ {"msg": {"msg": "item 0"}}, {"msg": {"msg": "item 1"}}]});
        assert_eq!(
            Ok(json!(["item 0", "item 1"])),
            json.query("$.data[*].msg.msg")
        );
    }

    #[test]
    fn support_simple_filters() {
        let json = json!({"data": [{"msg": "item 0"}, {"msg": "item 1"}]});
        assert_eq!(
            Ok(json!(["item 0", "item 1"])),
            json.query("$.data[*][?(@.msg)].msg")
        );
    }

    #[test]
    fn support_simple_filters_2() {
        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 10}]});
        assert_eq!(
            Ok(json!(["item 0"])),
            json.query("$.data[*][?(@.msg && @.id == 10)].msg")
        );
    }

    #[test]
    fn support_filters_with_in() {
        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 10}]});
        assert_eq!(
            Ok(json!(["item 0"])),
            json.query("$.data[*][?(@.msg in ['item 0'])].msg")
        );
        assert_eq!(
            Ok(json!(["item 0", "item 1", null])),
            json.query("$.data[*][?(@.id in [10, 11])].msg")
        );
    }

    #[test]
    fn support_filters_with_in2() {
        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 10}]});
        assert_eq!(
            Ok(json!(["item 1", null])),
            json.query("$.data[*][?(@.msg nin ['item 0'])].msg")
        );
        assert_eq!(
            Ok(json!([])),
            json.query("$.data[*][?(@.id nin [10, 11])].msg")
        );
    }

    #[test]
    fn support_filters_with_subsetof() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": ["M"], "id": 12}]});
        assert_eq!(
            Ok(json!([10, 12])),
            json.query("$.data[*][?(@.sizes subsetof ['M', \"L\"])].id")
        );
    }

    #[test]
    fn support_filters_with_anyof() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": ["XXL"], "id": 12}]});
        assert_eq!(
            Ok(json!([10, 11])),
            json.query("$.data[*][?(@.sizes anyof ['M', \"L\"])].id")
        );
    }

    #[test]
    fn support_filters_with_noneof() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": ["XXL"], "id": 12}]});
        assert_eq!(
            Ok(json!([12])),
            json.query("$.data[*][?(@.sizes noneof ['M', \"L\"])].id")
        );
    }

    #[test]
    fn support_filters_with_contains() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": ["XXL"], "id": 12}]});
        assert_eq!(
            Ok(json!([10, 11])),
            json.query("$.data[*][?(@.sizes contains 'M')].id")
        );

        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 10}]});
        assert_eq!(
            Ok(json!(["item 0"])),
            json.query("$.data[*][?(@.msg contains '0')].msg")
        );
    }

    #[test]
    fn support_filters_with_sizeof() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": ["XXL"], "id": 12}]});
        assert_eq!(
            Ok(json!([10, 11])),
            json.query("$.data[*][?(@.sizes size 2)].id")
        );

        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 10}]});
        assert_eq!(
            Ok(json!(["item 0", "item 1"])),
            json.query("$.data[*][?(@.msg size 6)].msg")
        );
    }

    #[test]
    fn support_filters_with_empty_op() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": [], "id": 12}]});
        assert_eq!(
            Ok(json!([10, 11])),
            json.query("$.data[*][?(@.sizes empty false)].id")
        );

        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 12}]});
        assert_eq!(
            Ok(json!([12])),
            json.query("$.data[*][?(@.msg empty true)].id")
        );
    }

    #[test]
    fn support_filters_with_not_op() {
        let json = json!({"data": [{"sizes": ["M", "L"], "id": 10}, {"sizes": ["M",  "XXL"], "id": 11}, {"sizes": [], "id": 12}]});
        assert_eq!(
            Ok(json!([12])),
            json.query("$.data[*][?(!(@.sizes empty false))].id")
        );

        let json = json!({"data": [{"msg": "item 0", "id": 10}, {"msg": "item 1", "id": 11}, {"msg": null, "id": 12}]});
        assert_eq!(Ok(json!([12])), json.query("$.data[*][?(!@.msg)].id"));
    }

    #[test]
    fn support_scan_and_filter() {
        let json = json!([1, 2, 3]);
        assert_eq!(Ok(json!([1, 2, 3])), json.query("$..[?(@>=1)]"));
    }
}
