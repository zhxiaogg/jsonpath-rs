use serde_json::{json, Value};

use crate::{JsonPathError, JsonPathResult};

use super::{
    constants::{DOUBLE_QUOTE, ESCAPE, MINUS, PERIOD, SINGLE_QUOTE},
    TokenStream,
};
pub trait PeekableExt {
    fn drop_while<P>(&mut self, predicate: P)
    where
        P: FnMut(&char) -> bool;

    fn peek_drop_while<P>(&mut self, predicate: P)
    where
        P: FnMut(&char) -> bool;

    fn next_significant(&mut self) -> Option<char>;

    fn peek_significant(&mut self) -> Option<&char>;

    fn peek_significant_and_move_on(&mut self) -> Option<char>;

    /**
     * peek next word separated by spaces, e.g. " hello "
     */
    fn peek_next_word(&mut self) -> Option<String>;

    fn read_quoted_string(&mut self) -> JsonPathResult<String>;

    fn peek_matches_ignore_case(&mut self, pattern: &str) -> JsonPathResult<bool>;

    fn read_number(&mut self) -> JsonPathResult<Value>;
}

impl<'a> PeekableExt for TokenStream<'a> {
    fn drop_while<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&char) -> bool,
    {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.next();
            } else {
                break;
            }
        }
    }

    fn peek_drop_while<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&char) -> bool,
    {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance_cursor();
            } else {
                break;
            }
        }
    }

    fn next_significant(&mut self) -> Option<char> {
        self.drop_while(|c| c.is_whitespace());
        self.next()
    }

    fn peek_significant(&mut self) -> Option<&char> {
        self.peek_drop_while(|c| c.is_whitespace());
        self.peek()
    }

    fn peek_significant_and_move_on(&mut self) -> Option<char> {
        self.peek_drop_while(|c| c.is_whitespace());
        let c = self.peek().copied();
        self.advance_cursor();
        c
    }

    fn peek_matches_ignore_case(&mut self, pattern: &str) -> JsonPathResult<bool> {
        let cursor = self.cursor();
        let mut chars = pattern.chars();
        loop {
            match (chars.next(), self.peek()) {
                (Some(l), Some(r)) if l.to_ascii_lowercase() == r.to_ascii_lowercase() => {
                    self.advance_cursor();
                }
                (None, _) => break,
                _ => {
                    self.move_cursor_back_by(self.cursor() - cursor)?;
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn peek_next_word(&mut self) -> Option<String> {
        let mut w = String::new();
        while let Some(c) = self.peek() {
            if !c.is_whitespace() {
                w.push(*c); // append to word for any non space chars
            } else if !w.is_empty() {
                break; // when any space
            }
            self.advance_cursor();
        }
        Some(w).filter(|w| !w.is_empty())
    }

    fn read_number(&mut self) -> JsonPathResult<Value> {
        let mut w = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || *c == PERIOD || *c == MINUS {
                w.push(*c); // append to word for any non space chars
            } else if !w.is_empty() {
                break; // when any space
            }
            self.next();
        }
        if w.is_empty() {
            return Err(JsonPathError::InvalidJsonPath(
                "Expect number.".to_string(),
                0,
            ));
        }
        let n = if w.contains('.') {
            let f = w.parse::<f64>().map_err(|_e| {
                JsonPathError::InvalidJsonPath("expect float number.".to_string(), 0)
            })?;
            json!(f)
        } else if w.contains(MINUS) {
            let i = w.parse::<i64>().map_err(|_e| {
                JsonPathError::InvalidJsonPath("expect float number.".to_string(), 0)
            })?;
            json!(i)
        } else {
            let u = w.parse::<u64>().map_err(|_e| {
                JsonPathError::InvalidJsonPath("expect u64 number.".to_string(), 0)
            })?;
            json!(u)
        };
        Ok(n)
    }

    fn read_quoted_string(&mut self) -> JsonPathResult<String> {
        let quote = match self.next_significant() {
            Some(c) if c == SINGLE_QUOTE || c == DOUBLE_QUOTE => c,
            _x => {
                return Err(JsonPathError::InvalidJsonPath(
                    "Expect quoted string.".to_string(),
                    self.cursor(),
                ));
            }
        };

        let mut s = String::new();
        let mut in_escape = false;
        for c in self.by_ref() {
            if in_escape {
                s.push(c);
                in_escape = false;
            } else if c == ESCAPE {
                s.push(c);
                in_escape = true;
            } else if c == quote {
                // end of string
                break;
            } else {
                s.push(c);
            }
        }

        Ok(s)
    }
}
