use crate::{JsonPathError, JsonPathResult};

use super::constants::SPLIT;

#[derive(Debug, PartialEq)]
pub enum Token {
    Root(RootPathToken),
    Property(PropertyPathToken),
    ArrayIndex { index: i32 },
    ArraySlice { start: i32, end: i32 },
    Predicate(PredicatePathToken),
    Function(FunctionPathToken),
    Scan(ScanPathToken),
    Wildcard(WildcardPathToken),
}

impl Token {
    pub fn root(root_path_char: char) -> Token {
        Token::Root(RootPathToken { root_path_char })
    }
    pub fn property(name: String) -> Token {
        Token::Property(PropertyPathToken {
            properties: vec![name],
        })
    }
    pub fn properties(properties: Vec<String>) -> Token {
        Token::Property(PropertyPathToken { properties })
    }
    pub fn scan() -> Token {
        Token::Scan(ScanPathToken {})
    }

    pub fn array_index(expr: String) -> JsonPathResult<Token> {
        let index = Self::as_i32(expr.as_str())?;
        Ok(Token::ArrayIndex { index })
    }

    pub fn array_slice(expr: String) -> JsonPathResult<Token> {
        let parts: Vec<&str> = expr.split(SPLIT).collect();
        if !parts.len() == 2 {
            return Err(JsonPathError::InvalidJsonPath(format!(
                "Invalid array splice {}",
                expr
            )));
        }
        let start = Self::as_i32(parts[0])?;
        let end = Self::as_i32(parts[1])?;
        Ok(Token::ArraySlice { start, end })
    }

    fn as_i32(v: &str) -> JsonPathResult<i32> {
        v.trim()
            .parse::<i32>()
            .map_err(|_e| JsonPathError::InvalidJsonPath("Invalid array index.".to_string()))
    }
}

#[derive(Debug, PartialEq)]
pub struct RootPathToken {
    pub root_path_char: char,
}
#[derive(Debug, PartialEq)]
pub struct PropertyPathToken {
    pub properties: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct PredicatePathToken {}
#[derive(Debug, PartialEq)]
pub struct FunctionPathToken {}
#[derive(Debug, PartialEq)]
pub struct ScanPathToken {}
#[derive(Debug, PartialEq)]
pub struct WildcardPathToken {}
