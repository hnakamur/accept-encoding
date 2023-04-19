use std::ops::Range;

pub fn resolve_q_value(value: &[u8], encoding: &[u8]) -> Option<f32> {
    let mut resolver = QValueResolver::new(value);
    resolver.resolve(encoding)
}

struct QValueResolver<'a> {
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

struct MatchResult {
    wildcard: bool,
    q: f32,
}

impl<'a> QValueResolver<'a> {
    fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub fn resolve(&mut self, encoding: &[u8]) -> Option<f32> {
        let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
        let is_compress = bytes_eq_ignore_case(encoding, b"compress");

        let mut is_q_param = false;
        while let Some(token) = self.lexer.next_token() {
            dbg!(&token);
            match self.state {
                State::SearchingEncoding => match token {
                    Token::TokenOrValue(tok_or_val) => {
                        self.cur_result = if bytes_eq_ignore_case(tok_or_val, encoding)
                            || (is_gzip && bytes_eq_ignore_case(tok_or_val, b"x-gzip"))
                            || (is_compress && bytes_eq_ignore_case(tok_or_val, b"x-compress"))
                        {
                            Some(MatchResult {
                                wildcard: false,
                                q: 1.0,
                            })
                        } else if tok_or_val == b"*" {
                            Some(MatchResult {
                                wildcard: true,
                                q: 1.0,
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
                                    cur_result.q = f.clamp(0.0, 1.0);
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
        self.best_result.take().map(|result| result.q)
    }

    fn may_update_best_result(&mut self) {
        if self.should_update_best_result() {
            self.best_result = self.cur_result.take();
        }
    }

    fn should_update_best_result(&self) -> bool {
        if let Some(cur_result) = self.cur_result.as_ref() {
            if let Some(best_result) = self.best_result.as_ref() {
                if cur_result.wildcard {
                    !best_result.wildcard && cur_result.q > best_result.q
                } else {
                    best_result.wildcard || cur_result.q > best_result.q
                }
            } else {
                true
            }
        } else {
            false
        }
    }
}

fn bytes_eq_ignore_case(bytes1: &[u8], bytes2: &[u8]) -> bool {
    if bytes1.len() != bytes2.len() {
        return false;
    }
    for i in 0..bytes1.len() {
        if !byte_eq_ignore_case(bytes1[i], bytes2[i]) {
            return false;
        }
    }
    true
}

fn byte_eq_ignore_case(b1: u8, b2: u8) -> bool {
    // Apapted from https://docs.rs/ascii/1.1.0/src/ascii/ascii_char.rs.html#726-732
    if b1 == b2 {
        return true;
    }
    let b1_not_upper = b1 | 0b010_0000;
    let b2_not_upper = b2 | 0b010_0000;
    b1_not_upper >= b'a' && b1_not_upper <= b'z' && b1_not_upper == b2_not_upper
}

struct Lexer<'a> {
    value: &'a [u8],
    pos: usize,
    in_quoted_str: bool,
    quoted_str_escaped: bool,
    token_range: Option<Range<usize>>,
}

#[derive(Debug, PartialEq)]
enum Token<'a> {
    TokenOrValue(&'a [u8]),
    DoubleQuotedString(&'a [u8]),
    Comma,
    Semicolon,
    Equal,
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
    fn test_q_value() {
        let q = resolve_q_value(b"*", b"gzip");
        assert_eq!(Some(1.0), q);

        let q = resolve_q_value(b"*  ; q=0.5", b"gzip");
        assert_eq!(Some(0.5), q);

        let q = resolve_q_value(b" gzip", b"gzip");
        assert_eq!(Some(1.0), q);

        let q = resolve_q_value(b" gzip ; a=b ", b"gzip");
        assert_eq!(Some(1.0), q);

        let q = resolve_q_value(b" gzip ; q=0.8 ", b"gzip");
        assert_eq!(Some(0.8), q);

        let q = resolve_q_value(b" x-Gzip ; q=0.8 ", b"gzip");
        assert_eq!(Some(0.8), q);

        let q = resolve_q_value(b"br  ; q=1", b"gzip");
        assert_eq!(None, q);

        let q = resolve_q_value(b"br  ; q=0.9 , gzip;q=0.8", b"gzip");
        assert_eq!(Some(0.8), q);

        let q = resolve_q_value(b"br  ; q=0.9 , gzip;q=0.8", b"br");
        assert_eq!(Some(0.9), q);
    }

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

    #[test]
    fn test_bytes_eq_ignore_case() {
        assert!(bytes_eq_ignore_case(b"gzip", b"gzip"));
        assert!(bytes_eq_ignore_case(b"gzip", b"GZip"));
        assert!(bytes_eq_ignore_case(b"bzip2", b"bziP2"));

        assert!(!bytes_eq_ignore_case(b"gzip", b"zip"));
        assert!(!bytes_eq_ignore_case(b"gzip", b"gzi2"));
    }
}
