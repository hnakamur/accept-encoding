use std::ops::Range;

use ordered_float::NotNan;

use crate::{bytes_eq_ignore_case, MatchResult, MatchType, Token};

pub(crate) struct QValueFinder<'a> {
    lexer: Lexer<'a>,
    state: State,
    cur_result: Option<MatchResult>,
    best_result: Option<MatchResult>,
}

enum State {
    SearchingEncoding,
    SeenSomeEncoding,
    SeenSemicolon,
    SeenParameterName,
    SeenEqual,
    SeenParameterValue,
}

impl<'a> QValueFinder<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub(crate) fn find(&mut self, encoding: &[u8]) -> Option<MatchResult> {
        let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
        let is_compress = bytes_eq_ignore_case(encoding, b"compress");

        let mut is_q_param = false;
        while let Some(token) = self.lexer.next_token() {
            match self.state {
                State::SearchingEncoding => match token {
                    Token::TokenOrValue(tok_or_val) => {
                        self.cur_result = if bytes_eq_ignore_case(tok_or_val, encoding)
                            || (is_gzip && bytes_eq_ignore_case(tok_or_val, b"x-gzip"))
                            || (is_compress && bytes_eq_ignore_case(tok_or_val, b"x-compress"))
                        {
                            Some(MatchResult {
                                match_type: MatchType::Exact,
                                q: NotNan::new(1.0).unwrap(),
                            })
                        } else if tok_or_val == b"*" {
                            Some(MatchResult {
                                match_type: MatchType::Wildcard,
                                q: NotNan::new(1.0).unwrap(),
                            })
                        } else {
                            None
                        };
                        self.state = State::SeenSomeEncoding;
                    }
                    _ => return None,
                },
                State::SeenSomeEncoding => match token {
                    Token::Semicolon => self.state = State::SeenSemicolon,
                    Token::Comma => {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    }
                    _ => return None,
                },
                State::SeenSemicolon => match token {
                    Token::TokenOrValue(tok_or_val) => {
                        is_q_param = tok_or_val == b"q";
                        self.state = State::SeenParameterName;
                    }
                    _ => return None,
                },
                State::SeenParameterName => match token {
                    Token::Equal => self.state = State::SeenEqual,
                    _ => return None,
                },
                State::SeenEqual => {
                    if let Some(cur_result) = self.cur_result.as_mut() {
                        if is_q_param {
                            match token {
                                Token::TokenOrValue(tok_or_val) => {
                                    // In general, HTTP header value are byte string
                                    // (ASCII + obs-text (%x80-FF)).
                                    // However we want a float literal here, so it's
                                    // ok to use from_utf8.
                                    let s = std::str::from_utf8(tok_or_val).ok()?;
                                    let f = s.parse::<f32>().ok()?;
                                    cur_result.q = NotNan::new(f.clamp(0.0, 1.0)).unwrap();
                                }
                                _ => return None,
                            }
                        }
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => match token {
                    Token::Comma => {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    }
                    Token::Semicolon => self.state = State::SeenSemicolon,
                    _ => return None,
                },
            }
        }
        self.may_update_best_result();
        self.best_result.take()
    }

    fn may_update_best_result(&mut self) {
        if self.cur_result.gt(&self.best_result) {
            self.best_result = self.cur_result.take();
        }
    }
}

struct Lexer<'a> {
    value: &'a [u8],
    pos: usize,
    in_quoted_str: bool,
    quoted_str_escaped: bool,
    token_range: Option<Range<usize>>,
}

impl<'a> Lexer<'a> {
    fn new(value: &'a [u8]) -> Self {
        Self {
            value,
            pos: 0,
            in_quoted_str: false,
            quoted_str_escaped: false,
            token_range: None,
        }
    }

    fn next_token(&mut self) -> Option<Token> {
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
