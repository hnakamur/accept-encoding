use std::ops::Range;

pub fn match_for_encoding(header_value: &[u8], encoding: &[u8]) -> Option<MatchResult> {
    QValueFinder::new(header_value).find(encoding)
}

#[derive(Debug, PartialEq)]
pub struct MatchResult {
    pub wildcard: bool,
    pub q: f32,
}

impl MatchResult {
    pub fn better(&self, other: &MatchResult) -> bool {
        if self.wildcard == other.wildcard {
            self.q > other.q
        } else {
            !self.wildcard
        }
    }
}

struct QValueFinder<'a> {
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
    fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub fn find(&mut self, encoding: &[u8]) -> Option<MatchResult> {
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
        self.best_result.take()
    }

    fn may_update_best_result(&mut self) {
        if self.should_update_best_result() {
            self.best_result = self.cur_result.take();
        }
    }

    fn should_update_best_result(&self) -> bool {
        if let Some(cur_result) = self.cur_result.as_ref() {
            if let Some(best_result) = self.best_result.as_ref() {
                cur_result.better(best_result)
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
    b1 == b2 || {
        let b1_not_upper = b1 | 0b010_0000;
        let b2_not_upper = b2 | 0b010_0000;
        b1_not_upper >= b'a' && b1_not_upper <= b'z' && b1_not_upper == b2_not_upper
    }
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
    fn test_match_for_encoding() {
        let res = match_for_encoding(b"*", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: true,
                q: 1.0,
            }),
            res,
        );

        let res = match_for_encoding(b"*  ; q=0.5", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: true,
                q: 0.5,
            }),
            res
        );

        let res = match_for_encoding(b" gzip", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: false,
                q: 1.0,
            }),
            res
        );

        let res = match_for_encoding(b" gzip ; a=b ", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: false,
                q: 1.0,
            }),
            res
        );

        let res = match_for_encoding(b" gzip ; q=0.8 ", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: false,
                q: 0.8,
            }),
            res
        );

        let res = match_for_encoding(b" x-Gzip ; q=0.8 ", b"gzip");
        assert_eq!(
            Some(MatchResult {
                wildcard: false,
                q: 0.8,
            }),
            res
        );

        let res = match_for_encoding(b"br  ; q=1", b"gzip");
        assert_eq!(None, res);

        {
            let header_value = b"br  ; q=0.9 , gzip;q=0.8";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    wildcard: false,
                    q: 0.8,
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    wildcard: false,
                    q: 0.9,
                }),
                br_res
            );

            assert!(br_res.unwrap().better(gzip_res.as_ref().unwrap()));
        }

        {
            let header_value = b"br , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    wildcard: true,
                    q: 1.0,
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    wildcard: false,
                    q: 1.0,
                }),
                br_res
            );

            assert!(br_res.unwrap().better(gzip_res.as_ref().unwrap()));
        }

        {
            let header_value = b"br; q=0.9 , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    wildcard: true,
                    q: 1.0,
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    wildcard: false,
                    q: 0.9,
                }),
                br_res
            );

            // assert!(br_res.unwrap().better(gzip_res.as_ref().unwrap()));
        }
    }

    #[test]
    fn test_match_result_better() {
        assert!(MatchResult {
            wildcard: false,
            q: 1.0,
        }
        .better(&MatchResult {
            wildcard: false,
            q: 0.9,
        }));

        assert!(MatchResult {
            wildcard: true,
            q: 1.0,
        }
        .better(&MatchResult {
            wildcard: true,
            q: 0.9,
        }));

        assert!(MatchResult {
            wildcard: false,
            q: 0.9,
        }
        .better(&MatchResult {
            wildcard: true,
            q: 1.0,
        }));

        assert!(!MatchResult {
            wildcard: true,
            q: 1.0,
        }
        .better(&MatchResult {
            wildcard: false,
            q: 0.9,
        }));

        assert!(!MatchResult {
            wildcard: false,
            q: 1.0,
        }
        .better(&MatchResult {
            wildcard: false,
            q: 1.0,
        }));
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
