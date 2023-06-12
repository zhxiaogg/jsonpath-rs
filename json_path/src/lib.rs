mod errors;
pub mod eval;
pub mod tokenizer;
pub use errors::*;
use eval::Eval;
use serde_json::Value;
use tokenizer::Tokenizer;

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
