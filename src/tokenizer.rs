mod constants;
mod tokens;
use constants::*;
use std::iter::Peekable;
pub use tokens::*;

pub struct Tokenizer {}

pub enum TokenizerError {
    Unknown,
    InvalidJsonPath(&'static str),
}

impl From<&'static str> for TokenizerError {
    fn from(value: &'static str) -> Self {
        TokenizerError::InvalidJsonPath(value)
    }
}

type TokenizerResult<T> = Result<T, TokenizerError>;

impl Tokenizer {
    pub fn tokenize(&self, jsonpath: &str) -> TokenizerResult<Vec<Token>> {
        let stream = jsonpath.chars().peekable();
        let mut stream = stream.skip_while(|c| c.is_whitespace()).peekable();

        let root_path_char = stream.next_if(|c| self.is_root_path_char(c)).ok_or(
            TokenizerError::InvalidJsonPath("The jsonpath must start with '$' or '@'"),
        )?;

        let root_path_token = RootPathToken { root_path_char };

        match stream.peek() {
            None => return Ok(vec![Token::Root(root_path_token)]),
            Some(n) if *n != PERIOD && *n != OPEN_SQUARE_BRACKET => {
                // TODO: add position info into the error
                return Err(TokenizerError::InvalidJsonPath(
                    "Illegal character, expected '.' or '['",
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
    ) -> TokenizerResult<bool> {
        match *stream.peek().unwrap() {
            OPEN_SQUARE_BRACKET => {
                let r = self.read_bracket_property_token(stream, tokens)?
                    || self.read_array_token(stream, tokens)?
                    || self.read_wildcard_token(stream, tokens)?
                    || self.read_filter_token(stream, tokens)?
                    || self.read_placeholder_token(stream, tokens)?;
                if !r {
                    return Err("Invalid jsonpath.")?;
                }
                Ok(true)
            }
            PERIOD => match self.read_dot_token(stream, tokens)? {
                true => Ok(true),
                false => return Err("Invalid jsonpath.")?,
            },
            WILDCARD => match self.read_wildcard_token(stream, tokens)? {
                true => Ok(true),
                false => return Err("Invalid jsonpath.")?,
            },
            _ => match self.read_property_or_function_token(stream, tokens)? {
                true => Ok(true),
                false => return Err("Invalid jsonpath.")?,
            },
        }
    }

    fn read_property_or_function_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_bracket_property_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_array_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_filter_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_placeholder_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_wildcard_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        todo!("implement this")
    }

    fn read_dot_token(
        &self,
        stream: &mut Peekable<impl Iterator<Item = char>>,
        tokens: &mut Vec<Token>,
    ) -> TokenizerResult<bool> {
        stream.next();
        match stream.peek() {
            Some(c) if *c == PERIOD => {
                stream.next();
                // create scan token
                tokens.push(Token::Scan(ScanPathToken {}));
                if let Some(PERIOD) = stream.peek().map(|c| *c) {
                    // TODO: add position info
                    return Err(TokenizerError::InvalidJsonPath(
                        "Unexpected '.' in the jsonpath.",
                    ));
                }
                self.read_next_token(stream, tokens)
            }
            None => {
                return Err(TokenizerError::InvalidJsonPath(
                    "the jsonpath must not end with a '.'",
                ))
            }
            _ => self.read_next_token(stream, tokens),
        }
    }

    fn is_root_path_char(&self, c: &char) -> bool {
        *c == DOC_CONTEXT || *c == EVAL_CONTEXT
    }
}
