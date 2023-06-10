mod constants;
mod tokens;
use constants::*;
use peekmore::PeekMore;
use peekmore::PeekMoreIterator;
use serde_json::Value;

use std::str::Chars;
pub use tokens::*;
mod stream;
use crate::{JsonPathError, JsonPathResult};
use stream::PeekableExt;

pub struct Tokenizer {}
pub type TokenStream<'a> = PeekMoreIterator<Chars<'a>>;

impl Tokenizer {
    pub fn new() -> Tokenizer {
        Tokenizer {}
    }

    pub fn tokenize(&self, jsonpath: &str) -> JsonPathResult<Vec<Token>> {
        let mut stream = jsonpath.chars().peekmore();
        match self.read_json_path(&mut stream)? {
            tokens if stream.next_significant().is_none() => Ok(tokens),
            tokens => Err(JsonPathError::InvalidJsonPath(
                format!(
                    "Cannot parse the full jsonpath string, parsed tokens: {:?}, next char: {:?}",
                    tokens,
                    stream.next_significant()
                ),
                stream.count(),
            )),
        }
    }

    fn read_json_path(&self, stream: &mut TokenStream<'_>) -> JsonPathResult<Vec<Token>> {
        let root_path_char = match stream.next_significant() {
            Some(c) if c == DOC_CONTEXT || c == EVAL_CONTEXT => c,
            x => {
                return Err(JsonPathError::InvalidJsonPath(
                    format!("The jsonpath must start with '$' or '@', found: {:?}", x),
                    stream.cursor(),
                ))
            }
        };

        let root_path_token = RootPathToken { root_path_char };

        match stream.peek() {
            None => Ok(vec![Token::Root(root_path_token)]),
            _ => {
                let mut tokens = vec![Token::Root(root_path_token)];
                self.read_next_token(stream, &mut tokens)?;
                Ok(tokens)
            }
        }
    }

    fn read_next_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match stream.peek().copied().unwrap() {
            OPEN_SQUARE_BRACKET => {
                let r = self.read_bracket_property_token(stream, tokens)?
                    || self.read_array_token(stream, tokens)?
                    || self.read_wildcard_token(stream, tokens)?
                    || self.read_filter_token(stream, tokens)?
                    || self.read_placeholder_token(stream, tokens)?;
                Ok(r)
            }
            PERIOD => {
                let r = self.read_dot_token(stream, tokens)?
                    || self.read_property_or_function_token(stream, tokens)?;
                Ok(r)
            }
            // TODO: support this scenario
            WILDCARD => self.read_wildcard_token(stream, tokens),
            _ => Ok(false),
        }
    }

    fn read_property_or_function_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        stream.next();

        let mut is_function = false;
        let mut s: String = String::new();
        while let Some(c) = stream.peek() {
            match *c {
                SPACE | PERIOD | OPEN_SQUARE_BRACKET | CLOSE_PARENTHESIS | CLOSE_SQUARE_BRACKET
                | '&' | '|' | '>' | '<' | '=' | '!' | '~' => break,
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
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        let potential_delimiter = match (
            stream.peek_significant_and_move_on(),
            stream.peek_significant().copied(),
        ) {
            (Some(OPEN_SQUARE_BRACKET), Some(c)) if c == SINGLE_QUOTE || c == DOUBLE_QUOTE => {
                stream.truncate_iterator_to_cursor();
                c
            }
            _ => {
                stream.reset_cursor();
                return Ok(false);
            }
        };

        let mut props: Vec<String> = vec![];
        let mut in_property = false;
        let mut in_escape = false;
        let mut current_prop = String::new();
        while let Some(c) = stream.next() {
            match c {
                _ if in_escape => in_escape = false,
                ESCAPE => in_escape = true,
                CLOSE_SQUARE_BRACKET if !in_property => {
                    break;
                }
                c if c == potential_delimiter && in_property => {
                    props.push(current_prop.clone());
                    in_property = false;
                    stream.drop_while(|c| c.is_whitespace());
                    match stream.peek() {
                        Some(c) if *c != CLOSE_SQUARE_BRACKET && *c != COMMA => {
                            return Err(JsonPathError::InvalidJsonPath(
                                format!("Read square bracket property failed, expect , or ], found: {:?}", c),
                                stream.cursor(),
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
                    stream.drop_while(|c| c.is_whitespace());
                    match stream.peek() {
                        // TODO: consider support diff delimiter?
                        Some(c) if *c == potential_delimiter => {}
                        _ => {
                            return Err(JsonPathError::InvalidJsonPath(
                                "Expecte delimiter after comma".to_string(),
                                stream.cursor(),
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
                stream.cursor(),
            ));
        }

        tokens.push(Token::properties(props));
        match stream.peek() {
            None => Ok(true),
            Some(_) => self.read_next_token(stream, tokens),
        }
    }

    fn read_array_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match (
            stream.peek_significant_and_move_on(),
            stream.peek_significant().copied(),
        ) {
            (Some(OPEN_SQUARE_BRACKET), Some(c))
                if c.is_ascii_digit() || c == MINUS || c == SPLIT => {}
            _ => {
                stream.reset_cursor();
                return Ok(false);
            }
        }
        stream.truncate_iterator_to_cursor();

        // try get array index, after the loop, next token should be ]
        let mut expr = String::new();
        while let Some(c) = stream.peek() {
            if c.is_ascii_digit() || *c == MINUS || *c == SPLIT || c.is_whitespace() || *c == COMMA
            {
                expr.push(*c);
                stream.next();
            } else {
                break;
            }
        }

        // check expr is present, next token is ]
        match stream.next() {
            Some(CLOSE_SQUARE_BRACKET) => {
                if expr.contains(SPLIT) {
                    tokens.push(Token::array_slice(expr)?);
                } else {
                    tokens.push(Token::array_index(expr)?);
                }
                match stream.peek() {
                    None => Ok(true),
                    Some(_) => self.read_next_token(stream, tokens),
                }
            }
            _ => Err(JsonPathError::InvalidJsonPath(
                "Expect ] to close array index/slice query.".to_string(),
                0,
            )),
        }
    }

    fn read_placeholder_token(
        &self,
        _stream: &mut TokenStream<'_>,
        _tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        Ok(false)
    }

    fn read_wildcard_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match (
            stream.peek_significant_and_move_on(),
            stream.peek_significant_and_move_on(),
        ) {
            (Some(OPEN_SQUARE_BRACKET), Some(WILDCARD)) => {
                stream.truncate_iterator_to_cursor();
                if let Some(CLOSE_SQUARE_BRACKET) = stream.next_significant() {
                    tokens.push(Token::Wildcard);
                    match stream.peek() {
                        None => Ok(true),
                        Some(_) => self.read_next_token(stream, tokens),
                    }
                } else {
                    Err(JsonPathError::InvalidJsonPath(
                        "Expect ] to close wildcard query.".to_string(),
                        0,
                    ))
                }
            }
            _ => {
                stream.reset_cursor();
                Ok(false)
            }
        }
    }

    fn read_dot_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match stream.peek_next() {
            Some(c) if *c == PERIOD => {
                stream.next();
                // create scan token
                tokens.push(Token::scan());
                if let Some(PERIOD) = stream.peek_next().copied() {
                    // TODO: add position info
                    return Err(JsonPathError::InvalidJsonPath(
                        "Unexpected '.' in the jsonpath.".to_string(),
                        stream.cursor(),
                    ));
                }
                self.read_property_or_function_token(stream, tokens)
            }
            _ => {
                stream.reset_cursor();
                Ok(false)
            }
        }
    }
}

impl Tokenizer {
    fn read_filter_token(
        &self,
        stream: &mut TokenStream<'_>,
        tokens: &mut Vec<Token>,
    ) -> JsonPathResult<bool> {
        match (
            stream.peek_significant_and_move_on(),
            stream.peek_significant_and_move_on(),
            stream.peek_significant_and_move_on(),
        ) {
            (Some(OPEN_SQUARE_BRACKET), Some(BEGIN_FILTER), Some(OPEN_PARENTHESIS)) => {
                stream.truncate_iterator_to_cursor();
                // it starts with "[?(", so assuming it's a filter: [?(Expression)]
                let expression = self.expr(0, stream)?;
                tokens.push(Token::Predicate(expression));
                match (stream.next_significant(), stream.next_significant()) {
                    (Some(CLOSE_PARENTHESIS), Some(CLOSE_SQUARE_BRACKET)) => match stream.peek() {
                        None => Ok(true),
                        _ => self.read_next_token(stream, tokens),
                    },
                    (x, y) => Err(JsonPathError::InvalidJsonPath(
                        format!(
                            "Expect close of filter, found: {:?}, {:?}, tokens: {:?}",
                            x, y, tokens
                        ),
                        stream.cursor(),
                    )),
                }
            }
            _ => {
                stream.reset_cursor();
                Ok(false)
            }
        }
    }

    fn expr(&self, bp: i32, tokens: &mut TokenStream<'_>) -> JsonPathResult<Expression> {
        let mut expression = self.nud(bp, tokens)?;
        while let Some(t) = tokens.peek_significant() {
            if self.expr_eof(t) {
                tokens.truncate_iterator_to_cursor();
                break;
            }

            // peek next comparator
            let c = self.peek_comparator(tokens)?;
            match c {
                None => break,
                Some(comparator) if bp >= self.bp(&comparator) => {
                    tokens.reset_cursor();
                    break;
                }
                Some(comparator) => {
                    tokens.truncate_iterator_to_cursor();
                    expression = self.led(expression, comparator, tokens)?;
                }
            }
        }
        Ok(expression)
    }

    fn peek_comparator(&self, stream: &mut TokenStream<'_>) -> JsonPathResult<Option<Comparator>> {
        stream.peek_drop_while(|c| c.is_whitespace());

        if stream.peek_matches_ignore_case("==")? {
            Ok(Some(Comparator::Eq))
        } else if stream.peek_matches_ignore_case("!=")? {
            Ok(Some(Comparator::Neq))
        } else if stream.peek_matches_ignore_case(">=")? {
            Ok(Some(Comparator::GtEq))
        } else if stream.peek_matches_ignore_case(">")? {
            Ok(Some(Comparator::Gt))
        } else if stream.peek_matches_ignore_case("<=")? {
            Ok(Some(Comparator::LtEq))
        } else if stream.peek_matches_ignore_case("<")? {
            Ok(Some(Comparator::Lt))
        } else if stream.peek_matches_ignore_case("~=")? {
            Ok(Some(Comparator::RegExpMatch))
        } else if stream.peek_matches_ignore_case("&&")? {
            Ok(Some(Comparator::AND))
        } else if stream.peek_matches_ignore_case("||")? {
            Ok(Some(Comparator::OR))
        } else if stream.peek_matches_ignore_case("in")? {
            Ok(Some(Comparator::IN))
        } else if stream.peek_matches_ignore_case("nin")? {
            Ok(Some(Comparator::NIN))
        } else if stream.peek_matches_ignore_case("subsetof")? {
            Ok(Some(Comparator::SubsetOf))
        } else if stream.peek_matches_ignore_case("anyof")? {
            Ok(Some(Comparator::AnyOf))
        } else if stream.peek_matches_ignore_case("noneof")? {
            Ok(Some(Comparator::NoneOf))
        } else if stream.peek_matches_ignore_case("contains")? {
            Ok(Some(Comparator::Contains))
        } else if stream.peek_matches_ignore_case("size")? {
            Ok(Some(Comparator::SizeOf))
        } else if stream.peek_matches_ignore_case("empty")? {
            Ok(Some(Comparator::Empty))
        } else {
            Ok(None)
        }
    }

    fn expr_eof(&self, c: &char) -> bool {
        *c == CLOSE_PARENTHESIS
    }

    fn bp(&self, c: &Comparator) -> i32 {
        match c {
            Comparator::AND => 3,
            Comparator::OR => 2,
            _ => 10,
        }
    }

    fn led(
        &self,
        left: Expression,
        comparator: Comparator,
        streams: &mut TokenStream<'_>,
    ) -> JsonPathResult<Expression> {
        let bp = self.bp(&comparator);
        let right = self.expr(bp, streams)?;
        Ok(Expression::CompareExpr {
            op: comparator,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn read_literal(&self, stream: &mut TokenStream<'_>) -> JsonPathResult<Value> {
        let c = stream.peek_significant().copied();
        stream.truncate_iterator_to_cursor();
        match c {
            Some(SINGLE_QUOTE) | Some(DOUBLE_QUOTE) => {
                let s = stream.read_quoted_string()?;
                Ok(Value::String(s))
            }
            Some(c) if c.is_ascii_digit() || c == MINUS => {
                let n = stream.read_number()?;
                Ok(n)
            }
            Some('t') | Some('T') => {
                if stream.peek_matches_ignore_case("true")? {
                    stream.truncate_iterator_to_cursor();
                    Ok(Value::Bool(true))
                } else {
                    Err(JsonPathError::InvalidJsonPath(
                        "Expect boolean true literal".to_string(),
                        0,
                    ))
                }
            }
            Some('f') | Some('F') => {
                if stream.peek_matches_ignore_case("false")? {
                    stream.truncate_iterator_to_cursor();
                    Ok(Value::Bool(false))
                } else {
                    Err(JsonPathError::InvalidJsonPath(
                        "Expect boolean false literal".to_string(),
                        0,
                    ))
                }
            }
            _ => Err(JsonPathError::InvalidJsonPath(
                format!("Expect literal, found {:?}", c),
                0,
            )),
        }
    }

    fn nud(&self, _bp: i32, stream: &mut TokenStream<'_>) -> JsonPathResult<Expression> {
        let c = stream.peek_significant().copied();
        stream.truncate_iterator_to_cursor();
        match c {
            Some(DOC_CONTEXT) | Some(EVAL_CONTEXT) => {
                let tokens = self.read_json_path(stream)?;
                Ok(Expression::JsonQuery(tokens))
            }
            Some(OPEN_PARENTHESIS) => {
                stream.next();
                let expression = self.expr(0, stream)?;
                match stream.next_significant() {
                    Some(CLOSE_PARENTHESIS) => Ok(expression),
                    x => Err(JsonPathError::InvalidJsonPath(
                        format!("expect ), found: {:?}", x),
                        0,
                    )),
                }
            }
            Some(NOT) => {
                stream.next();
                let expression = self.expr(1000, stream)?;
                Ok(Expression::Not(Box::new(expression)))
            }
            Some(OPEN_SQUARE_BRACKET) => {
                // array or set literal
                let mut values = Vec::new();
                loop {
                    match stream.next_significant() {
                        Some(CLOSE_SQUARE_BRACKET) => {
                            break;
                        }
                        Some(COMMA) | Some(OPEN_SQUARE_BRACKET) => {
                            let expression = self.read_literal(stream)?;
                            values.push(expression);
                        }
                        x => {
                            return Err(JsonPathError::InvalidJsonPath(
                                format!("expect array literals, found: {:?}", x),
                                0,
                            ))
                        }
                    }
                }
                Ok(Expression::Literal(Value::Array(values)))
            }
            Some(SINGLE_QUOTE) | Some(DOUBLE_QUOTE) => {
                self.read_literal(stream).map(Expression::Literal)
            }
            Some(c) if c.is_ascii_digit() || c == MINUS => {
                self.read_literal(stream).map(Expression::Literal)
            }
            Some('t') | Some('T') => self.read_literal(stream).map(Expression::Literal),
            Some('f') | Some('F') => self.read_literal(stream).map(Expression::Literal),
            _ => Err(JsonPathError::InvalidJsonPath(
                "Expect expressions.".to_string(),
                stream.cursor(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tokenizer::Token;

    use super::*;

    #[test]
    fn tokenizer_supports_query_root() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize(" $ ")?;
        let expected = vec![Token::root('$')];
        assert_eq!(expected, tokens);
        Ok(())
    }

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

    #[test]
    fn tokenizer_supports_array_index() {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$[ 101 ]..id");

        let expected = vec![
            Token::root('$'),
            Token::array_index("101".to_string()).unwrap(),
            Token::scan(),
            Token::property("id".to_string()),
        ];
        assert_eq!(Ok(expected), tokens);
    }

    #[test]
    fn tokenizer_supports_array_slice() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$[101 : 200 ]..id")?;

        let expected = vec![
            Token::root('$'),
            Token::array_slice("101:200".to_string())?,
            Token::scan(),
            Token::property("id".to_string()),
        ];
        assert_eq!(expected, tokens);
        Ok(())
    }

    #[test]
    fn tokenizer_reports_error_for_invalid_array_slice() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$[ 101 : 2 00 ]..id");
        assert!(tokens.is_err());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(@['id']!=' xxx' )]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter2() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(@['id']>=2)]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter3() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(@['id'] >= 2 || @.msg empty false)]");

        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter4() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens =
            tz.tokenize("$.data[?(@['id'] >= 2 || @.msg[?(@.value contains 'xx')] empty false)]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter5() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(@ empty false)]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_basic_filter_in() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(@.id in ['a', 'b', 1])]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_not_filter() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(!@.is_true||@.is_false)]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_filter_with_parenthesis() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(!(@.id empty true && @.id < 100))]");
        assert!(tokens.is_ok());
        Ok(())
    }

    #[test]
    fn tokenizer_supports_filter_with_parenthesis2() -> JsonPathResult<()> {
        let tz = Tokenizer {};
        let tokens = tz.tokenize("$.data[?(!(@.id empty true) || (@.msg empty true))]");
        assert!(tokens.is_ok());
        Ok(())
    }
}
