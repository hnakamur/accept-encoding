use std::cmp::Ordering;

use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer2::{self, Cursor},
    q_value::QValue,
};

pub fn match_for_encoding(header_value: &[u8], encoding: &[u8]) -> Option<EncodingMatch> {
    EncodingMatcher::new(header_value).match_encoding(encoding)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum EncodingMatchType {
    Wildcard,
    Exact,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct EncodingMatch {
    pub match_type: EncodingMatchType,
    pub q: QValue,
}

impl Ord for EncodingMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.match_type, &self.q).cmp(&(other.match_type, &other.q))
    }
}

impl PartialOrd for EncodingMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) struct EncodingMatcher<'a> {
    input: &'a [u8],
    state: State,
    cur_result: Option<EncodingMatch>,
    best_result: Option<EncodingMatch>,
}

#[derive(Debug)]
enum State {
    SearchingEncoding,
    SeenEncoding,
    SeenSemicolon,
    SeenParameterName,
    SeenEqual,
    SeenParameterValue,
}

impl<'a> EncodingMatcher<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub(crate) fn match_encoding(&mut self, encoding: &[u8]) -> Option<EncodingMatch> {
        let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
        let is_compress = bytes_eq_ignore_case(encoding, b"compress");

        let mut is_q_param = false;
        let mut c = Cursor(0);
        while !c.eof(self.input) {
            match self.state {
                State::SearchingEncoding => {
                    if let Ok(c2) = lexer2::token(self.input, c) {
                        let token = c.slice(self.input, c2);
                        c = c2;
                        self.cur_result = if bytes_eq_ignore_case(token, encoding)
                            || (is_gzip && bytes_eq_ignore_case(token, b"x-gzip"))
                            || (is_compress && bytes_eq_ignore_case(token, b"x-compress"))
                        {
                            Some(EncodingMatch {
                                match_type: EncodingMatchType::Exact,
                                q: QValue::from_millis(1000).unwrap(),
                            })
                        } else if token == b"*" {
                            Some(EncodingMatch {
                                match_type: EncodingMatchType::Wildcard,
                                q: QValue::from_millis(1000).unwrap(),
                            })
                        } else {
                            None
                        };
                        self.state = State::SeenEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenEncoding => {
                    if let Ok(c2) = lexer2::ows_semicolon_ows(self.input, c) {
                        c = c2;
                        self.state = State::SeenSemicolon;
                    } else if let Ok(c2) = lexer2::ows_comma_ows(self.input, c) {
                        c = c2;
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else if lexer2::consume_ows_till_eof(self.input, c).is_ok() {
                        break;
                    }
                }
                State::SeenSemicolon => {
                    let c1 = c;
                    c = lexer2::token(self.input, c).ok()?;
                    let param_name = c1.slice(self.input, c);
                    is_q_param = bytes_eq_ignore_case(param_name, b"q");
                    self.state = State::SeenParameterName;
                }
                State::SeenParameterName => {
                    c = lexer2::byte(b'=')(self.input, c).ok()?;
                    self.state = State::SeenEqual;
                }
                State::SeenEqual => {
                    if is_q_param {
                        let c1 = c;
                        c = lexer2::q_value(self.input, c).ok()?;
                        if let Some(cur_result) = self.cur_result.as_mut() {
                            cur_result.q = QValue::try_from(c1.slice(self.input, c)).unwrap();
                        }
                    } else {
                        c = lexer2::alt(lexer2::token, lexer2::quoted_string)(self.input, c)
                            .ok()?;
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => {
                    if let Ok(c2) = lexer2::ows_comma_ows(self.input, c) {
                        c = c2;
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else if let Ok(c2) = lexer2::ows_semicolon_ows(self.input, c) {
                        c = c2;
                        self.state = State::SeenSemicolon;
                    } else if lexer2::consume_ows_till_eof(self.input, c).is_ok() {
                        break;
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_for_encoding() {
        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_encoding(b"*", b"gzip"),
        );

        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(0.5).unwrap(),
            }),
            match_for_encoding(b"*  ; q=0.5", b"gzip")
        );

        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_encoding(b"gzip", b"gzip")
        );

        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_encoding(b"gzip ; a=b", b"gzip")
        );

        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(0.8).unwrap(),
            }),
            match_for_encoding(b"gzip ; q=0.8", b"gzip")
        );

        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(0.8).unwrap(),
            }),
            match_for_encoding(b"x-Gzip ; q=0.8", b"gzip")
        );

        assert_eq!(None, match_for_encoding(b"br  ; q=1", b"gzip"));

        {
            let header_value = b"br  ; q=0.9 , gzip;q=0.8";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Exact,
                    q: QValue::try_from(0.8).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Exact,
                    q: QValue::try_from(0.9).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }

        {
            let header_value = b"br , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Wildcard,
                    q: QValue::try_from(1.0).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Exact,
                    q: QValue::try_from(1.0).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }

        {
            let header_value = b"br; q=0.9 , *";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Wildcard,
                    q: QValue::try_from(1.0).unwrap(),
                }),
                gzip_res
            );

            let br_res = match_for_encoding(header_value, b"br");
            assert_eq!(
                Some(EncodingMatch {
                    match_type: EncodingMatchType::Exact,
                    q: QValue::try_from(0.9).unwrap(),
                }),
                br_res
            );

            assert!(br_res.gt(&gzip_res));
        }

        {
            let header_value = b"gzip; q =0.9";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(None, gzip_res);
        }

        {
            let header_value = b"gzip; q= 0.9";
            let gzip_res = match_for_encoding(header_value, b"gzip");
            assert_eq!(None, gzip_res);
        }
    }

    #[test]
    fn test_match_result_cmp() {
        assert_eq!(
            Ordering::Greater,
            EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Greater,
            EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(1.0).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Equal,
            EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(0.9).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(0.9).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Equal,
            EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Greater,
            EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(1.0).unwrap(),
            })
        );

        assert_eq!(
            Ordering::Less,
            EncodingMatch {
                match_type: EncodingMatchType::Wildcard,
                q: QValue::try_from(1.0).unwrap(),
            }
            .cmp(&EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(0.9).unwrap(),
            })
        );
    }
}
