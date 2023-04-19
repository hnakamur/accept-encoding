use std::ops::Range;

use http::HeaderValue;

struct Lexer {
    value: HeaderValue,
    pos: usize,
    in_quoted_str: bool,
    quoted_str_escaped: bool,
    token_range: Option<Range<usize>>,
}

impl Lexer {
    fn new(value: HeaderValue) -> Self {
        Self {
            value,
            pos: 0,
            in_quoted_str: false,
            quoted_str_escaped: false,
            token_range: None,
        }
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        let value = self.value.as_bytes();
        while self.pos < value.len() {
            let c = value[self.pos];
            if self.in_quoted_str {
                if self.quoted_str_escaped {
                    self.quoted_str_escaped = false;
                } else {
                    match c {
                        b'"' => {
                            self.in_quoted_str = false;
                            let range = self.token_range.take().unwrap();
                            let token = &value[range.start..self.pos + 1];
                            self.pos = self.pos + 1;
                            return Some(token);
                        }
                        b'\\' => self.quoted_str_escaped = true,
                        _ => {}
                    }
                }
            } else {
                match c {
                    b',' | b';' | b'=' => {
                        let token = if let Some(range) = self.token_range.take() {
                            &value[range.start..range.end]
                        } else {
                            let token = &value[self.pos..self.pos + 1];
                            self.pos = self.pos + 1;
                            token
                        };
                        return Some(token);
                    }
                    b' ' | b'\t' => {}
                    b'"' => {
                        self.in_quoted_str = true;
                        self.token_range = Some(Range {
                            start: self.pos,
                            end: self.pos + 1,
                        });
                    }
                    _ => {
                        if let Some(mut token_range) = self.token_range.as_mut() {
                            token_range.end = self.pos + 1;
                        } else {
                            self.token_range = Some(Range {
                                start: self.pos,
                                end: self.pos + 1,
                            });
                        }
                    }
                }
            }
            self.pos += 1;
        }
        if self.in_quoted_str {
            None
        } else if let Some(range) = self.token_range.take() {
            Some(&value[range.start..range.end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_just_comma() {
        let header = HeaderValue::from_str(",").unwrap();
        let mut lexer = Lexer::new(header);
        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b",", tok.as_ref().unwrap());
        assert!(lexer.next_token().is_none());
    }

    #[test]
    fn test_lexer_quoted_string() {
        let header = HeaderValue::from_str(" foo  ;a=\"bar, \\\"baz\"; q=1, bar ").unwrap();
        let mut lexer = Lexer::new(header);

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"foo", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b";", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"a", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"=", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"\"bar, \\\"baz\"", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b";", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"q", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"=", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"1", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b",", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"bar", tok.as_ref().unwrap());

        assert!(lexer.next_token().is_none());
    }
}
