#[derive(Debug, PartialEq)]
pub enum Token {
    Root(RootPathToken),
    Property(PropertyPathToken),
    ArrayIndex(ArrayIndexPathToken),
    ArrayPath(ArrayPathPathToken),
    ArraySlice(ArraySlicePathToken),
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
pub struct ArrayIndexPathToken {}

#[derive(Debug, PartialEq)]
pub struct ArrayPathPathToken {}

#[derive(Debug, PartialEq)]
pub struct ArraySlicePathToken {}

#[derive(Debug, PartialEq)]
pub struct PredicatePathToken {}
#[derive(Debug, PartialEq)]
pub struct FunctionPathToken {}
#[derive(Debug, PartialEq)]
pub struct ScanPathToken {}
#[derive(Debug, PartialEq)]
pub struct WildcardPathToken {}
