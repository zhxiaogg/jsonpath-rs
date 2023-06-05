mod constants;
mod tokens;
use constants::*;
use std::iter::Peekable;
pub use tokens::*;
mod stream;
use crate::{JsonPathError, JsonPathResult};
use stream::clone_for_look_ahead;
use stream::PeekableExt;

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
        stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
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
        stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
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

    /// read ['a','b'] etc. properties within square brackets
    fn read_bracket_property_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        let mut working_stream = clone_for_look_ahead(stream);
        working_stream.next();
        working_stream.drop_while(|c| c.is_whitespace());

        let mut potential_delimiter = SINGLE_QUOTE;
        match working_stream.peek() {
            None => return Ok(false),
            Some(c) if *c == SINGLE_QUOTE || *c == DOUBLE_QUOTE => {
                potential_delimiter = *c;
            }
            _ => return Ok(false),
        }

        let mut props: Vec<String> = vec![];
        let mut in_property = false;
        let mut in_escape = false;
        let mut current_prop = String::new();
        while let Some(c) = working_stream.next() {
            match c {
                _ if in_escape => in_escape = false,
                ESCAPE => in_escape = true,
                CLOSE_SQUARE_BRACKET if !in_property => {
                    break;
                }
                c if c == potential_delimiter && in_property => {
                    props.push(current_prop.clone());
                    in_property = false;
                    working_stream.drop_while(|c| c.is_whitespace());
                    match working_stream.peek() {
                        Some(c) if *c != CLOSE_SQUARE_BRACKET && *c != COMMA => {
                            return Err(JsonPathError::InvalidJsonPath(
                                "Property must be separated by comma or Property must be terminated close square bracket.".to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
                c if c == potential_delimiter && !in_property => {
                    current_prop = String::new();
                    in_property = true;
                }
                COMMA if !in_property => {
                    working_stream.drop_while(|c| c.is_whitespace());
                    match working_stream.peek() {
                        // TODO: consider support diff delimiter?
                        Some(c) if *c == potential_delimiter => {}
                        _ => {
                            return Err(JsonPathError::InvalidJsonPath(
                                "Expecte delimiter after comma".to_string(),
                            ))
                        }
                    }
                }
                _ => current_prop.push(c),
            }
        }

        if in_property {
            return Err(JsonPathError::InvalidJsonPath(
                "Incomplete property".to_string(),
            ));
        }

        tokens.push(Token::properties(props));
        match working_stream.peek() {
            None => Ok(true),
            Some(_) => self.read_next_token(&mut working_stream, tokens),
        }
    }

    fn read_array_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_filter_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_placeholder_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_wildcard_token(
        &self,
        _stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        todo!("implement this")
    }

    fn read_dot_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char> + Clone>,
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

    #[test]
    fn tokenizer_supports_square_bracket_properties() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$['data', 'value']..id")?;

        let expected = vec![
            Token::root('$'),
            Token::properties(vec!["data".to_string(), "value".to_string()]),
            Token::scan(),
            Token::property("id".to_string()),
        ];
        assert_eq!(expected, tokens);
        Ok(())
    }

    #[test]
    fn tokenizer_supports_square_bracket_properties_with_white_spaces() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$[ 'data' , ' val ue '  ]..id")?;

        let expected = vec![
            Token::root('$'),
            Token::properties(vec!["data".to_string(), " val ue ".to_string()]),
            Token::scan(),
            Token::property("id".to_string()),
        ];
        assert_eq!(expected, tokens);
        Ok(())
    }

    #[test]
    fn tokenizer_should_fail_if_no_delimiter_after_comman_when_parsing_bracket_properties(
    ) -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let result = tz.tokenize("$[ 'data' , uexpected' val ue '  ]..id");
        assert!(result.is_err());
        Ok(())
    }
}
