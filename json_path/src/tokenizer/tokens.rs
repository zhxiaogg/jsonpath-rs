use serde_json::Value;

use crate::{JsonPathError, JsonPathResult};

use super::constants::{COMMA, SPLIT};

#[derive(Debug, PartialEq)]
pub enum Token {
    Root(RootPathToken),
    Property(PropertyPathToken),
    ArrayIndex { indices: Vec<i32> },
    ArraySlice(ArraySlice),
    Predicate(Expression),
    Function(FunctionPathToken),
    Scan(ScanPathToken),
    Wildcard,
}

#[derive(Debug, PartialEq)]
pub enum ArraySlice {
    // inclusive
    From(i32),
    // exclusive
    To(i32),
    // inclusive, exclusive
    Between(i32, i32),
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
        let indices = expr
            .split(COMMA)
            .map(Self::as_i32)
            .collect::<JsonPathResult<Vec<i32>>>()?;

        Ok(Token::ArrayIndex { indices })
    }

    pub fn array_slice(expr: String) -> JsonPathResult<Token> {
        let parts: Vec<&str> = expr.split(SPLIT).collect();
        if !parts.len() == 2 {
            return Err(JsonPathError::InvalidJsonPath(
                format!("Invalid array slice: {}", expr),
                0,
            ));
        }
        let array_slice = match (parts[0].trim(), parts[1].trim()) {
            ("", "") => {
                return Err(JsonPathError::InvalidJsonPath(
                    format!("Invalid array slice: {}", expr),
                    0,
                ))
            }
            (f, "") if !f.is_empty() => ArraySlice::From(Self::as_i32(f)?),
            ("", t) if !t.is_empty() => ArraySlice::To(Self::as_i32(t)?),
            (f, t) => ArraySlice::Between(Self::as_i32(f)?, Self::as_i32(t)?),
        };
        Ok(Token::ArraySlice(array_slice))
    }

    fn as_i32(v: &str) -> JsonPathResult<i32> {
        v.trim()
            .parse::<i32>()
            .map_err(|_e| JsonPathError::InvalidJsonPath("Invalid array index.".to_string(), 0))
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
pub enum Comparator {
    Eq,
    Neq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    RegExpMatch,
    AND,
    OR,
    IN,
    NIN, // not in
    SubsetOf,
    AnyOf,
    NoneOf,
    Contains,
    SizeOf,
    Empty,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    JsonQuery(Vec<Token>),
    Literal(Value),
    Not(Box<Expression>),
    Array(Vec<Expression>),
    CompareExpr {
        op: Comparator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[cfg(test)]
mod test {
    use crate::tokenizer::Token;

    use crate::tokenizer::tokens::ArraySlice;

    #[test]
    fn can_parse_array_slice_from() {
        assert_eq!(
            Ok(Token::ArraySlice(ArraySlice::From(3))),
            Token::array_slice(" 3 :".to_string())
        )
    }

    #[test]
    fn can_parse_array_slice_to() {
        assert_eq!(
            Ok(Token::ArraySlice(ArraySlice::To(3))),
            Token::array_slice("  : 3 ".to_string())
        )
    }

    #[test]
    fn can_parse_array_slice_between() {
        assert_eq!(
            Ok(Token::ArraySlice(ArraySlice::Between(1, 3))),
            Token::array_slice(" 1 : 3 ".to_string())
        )
    }

    #[test]
    fn can_parse_single_array_index() {
        assert_eq!(
            Ok(Token::ArrayIndex { indices: vec![-1] }),
            Token::array_index("-1".to_string())
        )
    }
    #[test]
    fn can_parse_multiple_array_index() {
        assert_eq!(
            Ok(Token::ArrayIndex {
                indices: vec![-1, 1]
            }),
            Token::array_index("-1, 1".to_string())
        )
    }
}
