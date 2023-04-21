use std::ops::Range;

use crate::Token;

pub(crate) struct Lexer<'a> {
    value: &'a [u8],
    pos: usize,
    in_quoted_str: bool,
    quoted_str_escaped: bool,
    token_range: Option<Range<usize>>,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self {
            value,
            pos: 0,
            in_quoted_str: false,
            quoted_str_escaped: false,
            token_range: None,
        }
    }

    pub(crate) fn next_token(&mut self) -> Option<Token> {
        let value = self.value;
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
                            return Some(Token::DoubleQuotedString(token));
                        }
                        b'\\' => self.quoted_str_escaped = true,
                        _ => {}
                    }
                }
            } else {
                match c {
                    b',' | b';' | b'=' => {
                        if let Some(range) = self.token_range.take() {
                            return Some(Token::TokenOrValue(&value[range.start..range.end]));
                        }

                        self.pos = self.pos + 1;
                        return match c {
                            b',' => Some(Token::Comma),
                            b';' => Some(Token::Semicolon),
                            b'=' => Some(Token::Equal),
                            _ => unreachable!(),
                        };
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
            Some(Token::TokenOrValue(&value[range.start..range.end]))
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
        let mut lexer = Lexer::new(b",");
        assert_eq!(Some(Token::Comma), lexer.next_token());
        assert_eq!(None, lexer.next_token());
    }

    #[test]
    fn test_lexer_quoted_string() {
        let mut lexer = Lexer::new(b" foo  ;a=\"bar, \\\"baz\"; q=1, bar ");
        assert_eq!(Some(Token::TokenOrValue(b"foo")), lexer.next_token());
        assert_eq!(Some(Token::Semicolon), lexer.next_token());
        assert_eq!(Some(Token::TokenOrValue(b"a")), lexer.next_token());
        assert_eq!(Some(Token::Equal), lexer.next_token());
        assert_eq!(
            Some(Token::DoubleQuotedString(b"\"bar, \\\"baz\"")),
            lexer.next_token()
        );
        assert_eq!(Some(Token::Semicolon), lexer.next_token());
        assert_eq!(Some(Token::TokenOrValue(b"q")), lexer.next_token());
        assert_eq!(Some(Token::Equal), lexer.next_token());
        assert_eq!(Some(Token::TokenOrValue(b"1")), lexer.next_token());
        assert_eq!(Some(Token::Comma), lexer.next_token());
        assert_eq!(Some(Token::TokenOrValue(b"bar")), lexer.next_token());
        assert_eq!(None, lexer.next_token());
    }
}
