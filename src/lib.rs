use std::cmp::Ordering;

use ordered_float::NotNan;

use crate::lexer::Lexer;

pub mod c;
mod lexer;
mod monolith_lexer;

pub fn match_for_encoding(header_value: &[u8], encoding: &[u8]) -> Option<MatchResult> {
    QValueFinder::new(header_value).find(encoding)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum MatchType {
    Wildcard,
    Exact,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct MatchResult {
    pub match_type: MatchType,
    pub q: NotNan<f32>,
}

impl Ord for MatchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.match_type, &self.q).cmp(&(other.match_type, &other.q))
    }
}

impl PartialOrd for MatchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

#[derive(Debug, PartialEq)]
enum Token<'a> {
    TokenOrValue(&'a [u8]),
    DoubleQuotedString(&'a [u8]),
    Comma,
    Semicolon,
    Equal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_for_encoding() {
        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(1.0).unwrap(),
            }),
            match_for_encoding(b"*", b"gzip"),
        );

        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(0.5).unwrap(),
            }),
            match_for_encoding(b"*  ; q=0.5", b"gzip")
        );

        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            }),
            match_for_encoding(b" gzip", b"gzip")
        );

        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            }),
            match_for_encoding(b" gzip ; a=b ", b"gzip")
        );

        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(0.8).unwrap(),
            }),
            match_for_encoding(b" gzip ; q=0.8 ", b"gzip")
        );

        assert_eq!(
            Some(MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(0.8).unwrap(),
            }),
            match_for_encoding(b" x-Gzip ; q=0.8 ", b"gzip")
        );

        assert_eq!(None, match_for_encoding(b"br  ; q=1", b"gzip"));

        {
            let header_value = b"br  ; q=0.9 , gzip;q=0.8";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Exact,
                    q: NotNan::new(0.8).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Exact,
                    q: NotNan::new(0.9).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }

        {
            let header_value = b"br , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Wildcard,
                    q: NotNan::new(1.0).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Exact,
                    q: NotNan::new(1.0).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }

        {
            let header_value = b"br; q=0.9 , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Wildcard,
                    q: NotNan::new(1.0).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(MatchResult {
                    match_type: MatchType::Exact,
                    q: NotNan::new(0.9).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }
    }

    #[test]
    fn test_match_result_cmp() {
        assert_eq!(
            Ordering::Greater,
            MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Greater,
            MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(1.0).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Equal,
            MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(0.9).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Equal,
            MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Greater,
            MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(1.0).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(1.0).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Less,
            MatchResult {
                match_type: MatchType::Wildcard,
                q: NotNan::new(1.0).unwrap(),
            }
            .cmp(&MatchResult {
                match_type: MatchType::Exact,
                q: NotNan::new(0.9).unwrap(),
            })
        );
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

#[cfg(kani)]
mod verification {
    use super::*;

    fn any_u8_vec(bound: usize) -> Vec<u8> {
        let size: usize = kani::any();
        kani::assume(size <= bound);

        let mut v = Vec::<u8>::with_capacity(size);
        for _ in 0..size {
            v.push(kani::any());
        }
        v
    }

    #[kani::proof]
    fn verify_match_for_encoding() {
        let header_value = any_u8_vec(64);
        let encoding = any_u8_vec(6);
        _ = match_for_encoding(&header_value, &encoding);
    }
}
