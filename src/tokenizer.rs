mod constants;
mod tokens;
use constants::*;
use std::iter::Peekable;
pub use tokens::*;

use crate::{JsonPathError, JsonPathResult};

pub struct Tokenizer {}

impl Tokenizer {
    pub fn new() -> Tokenizer {
        Tokenizer {}
    }

    pub fn tokenize(&self, jsonpath: &str) -> JsonPathResult<Vec<Token>> {
        let stream = jsonpath.chars().peekable();
        let mut stream = stream.skip_while(|c| c.is_whitespace()).peekable();

        let root_path_char =
            stream
                .next_if(|c| self.is_root_path_char(c))
                .ok_or(JsonPathError::InvalidJsonPath(
                    "The jsonpath must start with '$' or '@'".to_string(),
                ))?;

        let root_path_token = RootPathToken { root_path_char };

        match stream.peek() {
            None => return Ok(vec![Token::Root(root_path_token)]),
            Some(n) if *n != PERIOD && *n != OPEN_SQUARE_BRACKET => {
                // TODO: add position info into the error
                return Err(JsonPathError::InvalidJsonPath(
                    "Illegal character, expected '.' or '['".to_string(),
                ));
            }
            _ => {}
        }

        let mut tokens = vec![Token::Root(root_path_token)];
        self.read_next_token(&mut stream, &mut tokens)?;

        Ok(tokens)
    }

    fn read_next_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match *stream.peek().unwrap() {
            OPEN_SQUARE_BRACKET => {
                let r = self.read_bracket_property_token(stream, tokens)?
                    || self.read_array_token(stream, tokens)?
                    || self.read_wildcard_token(stream, tokens)?
                    || self.read_filter_token(stream, tokens)?
                    || self.read_placeholder_token(stream, tokens)?;
                if !r {
                    return Err(JsonPathError::InvalidJsonPath(
                        "Invalid jsonpath.".to_string(),
                    ));
                }
                Ok(true)
            }
            PERIOD => match self.read_dot_token(stream, tokens)? {
                true => Ok(true),
                false => {
                    return Err(JsonPathError::InvalidJsonPath(
                        "Invalid jsonpath.".to_string(),
                    ))
                }
            },
            WILDCARD => match self.read_wildcard_token(stream, tokens)? {
                true => Ok(true),
                false => {
                    return Err(JsonPathError::InvalidJsonPath(
                        "Invalid jsonpath.".to_string(),
                    ))
                }
            },
            _ => match self.read_property_or_function_token(stream, tokens)? {
                true => Ok(true),
                false => {
                    return Err(JsonPathError::InvalidJsonPath(
                        "Invalid jsonpath.".to_string(),
                    ))
                }
            },
        }
    }

    fn read_property_or_function_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match *stream.peek().unwrap() {
            OPEN_SQUARE_BRACKET | WILDCARD | PERIOD | SPACE => return Ok(false),
            _ => {}
        };

        let mut is_function = false;
        let mut s: String = String::new();
        while let Some(c) = stream.peek() {
            match *c {
                SPACE => return Err(JsonPathError::InvalidJsonPath(
                    "Use bracket notion ['my prop'] if your property contains blank characters."
                        .to_string(),
                )),
                PERIOD | OPEN_SQUARE_BRACKET => break,
                OPEN_PARENTHESIS => {
                    is_function = true;
                    break;
                }
                _ => {
                    s.push(*c);
                    stream.next();
                }
            }
        }
        if is_function {
            unimplemented!("function is not supported")
        } else {
            tokens.push(Token::property(s))
        }

        match stream.peek().is_some() {
            true => self.read_next_token(stream, tokens),
            false => Ok(true),
        }
    }

    fn read_bracket_property_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char>>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_array_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char>>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_filter_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char>>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_placeholder_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char>>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_wildcard_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char>>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_dot_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        stream.next();
        match stream.peek() {
            Some(c) if *c == PERIOD => {
                stream.next();
                // create scan token
                tokens.push(Token::scan());
                if let Some(PERIOD) = stream.peek().map(|c| *c) {
                    // TODO: add position info
                    return Err(JsonPathError::InvalidJsonPath(
                        "Unexpected '.' in the jsonpath.".to_string(),
                    ));
                }
                self.read_next_token(stream, tokens)
            }
            None => {
                return Err(JsonPathError::InvalidJsonPath(
                    "the jsonpath must not end with a '.'".to_string(),
                ))
            }
            _ => self.read_next_token(stream, tokens),
        }
    }

    fn is_root_path_char(&self, c: &char) -> bool {
        *c == DOC_CONTEXT || *c == EVAL_CONTEXT
    }
}

#[cfg(test)]
mod test {
    use crate::tokenizer::Token;

    use super::*;

    #[test]
    fn tokenizer_supports_query_properties() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data.id")?;

        let expected = vec![
            Token::root('$'),
            Token::property("data".to_string()),
            Token::property("id".to_string()),
        ];
        assert_eq!(expected, tokens);
        Ok(())
    }

    #[test]
    fn tokenizer_supports_scan_properties() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data..id")?;

        let expected = vec![
            Token::root('$'),
            Token::property("data".to_string()),
            Token::scan(),
            Token::property("id".to_string()),
        ];
        assert_eq!(expected, tokens);
        Ok(())
    }
}
