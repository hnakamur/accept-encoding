use std::{cmp::Ordering, str};

use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer::{self, Cursor},
    q_value::QValue,
};

pub fn match_for_encoding(input: &[u8], encoding: &[u8]) -> Option<EncodingMatch> {
    let mut state = State::SearchingEncoding;
    let mut cur_result: Option<EncodingMatch> = None;
    let mut best_result: Option<EncodingMatch> = None;

    let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
    let is_compress = bytes_eq_ignore_case(encoding, b"compress");

    let mut is_q_param = false;
    let mut c = Cursor(0);
    while !c.eof(input) {
        match state {
            State::SearchingEncoding => {
                let c1 = c;
                if lexer::token(input, &mut c).is_ok() {
                    let token = c1.slice(input, c);
                    cur_result = if bytes_eq_ignore_case(token, encoding)
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
                    state = State::SeenEncoding;
                } else {
                    return None;
                }
            }
            State::SeenEncoding => {
                lexer::ows(input, &mut c);
                if lexer::byte(b';')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    state = State::SeenSemicolon;
                } else if lexer::byte(b',')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingEncoding;
                }
            }
            State::SeenSemicolon => {
                let c1 = c;
                lexer::token(input, &mut c).ok()?;
                let param_name = c1.slice(input, c);
                is_q_param = bytes_eq_ignore_case(param_name, b"q");
                state = State::SeenParameterName;
            }
            State::SeenParameterName => {
                lexer::byte(b'=')(input, &mut c).ok()?;
                state = State::SeenEqual;
            }
            State::SeenEqual => {
                if is_q_param {
                    let c1 = c;
                    lexer::q_value(input, &mut c).ok()?;
                    if let Some(cur_result) = cur_result.as_mut() {
                        cur_result.q =
                            QValue::try_from(str::from_utf8(c1.slice(input, c)).unwrap()).unwrap();
                    }
                } else {
                    lexer::alt(lexer::token, lexer::quoted_string)(input, &mut c).ok()?;
                }
                state = State::SeenParameterValue;
            }
            State::SeenParameterValue => {
                lexer::ows(input, &mut c);
                if lexer::byte(b',')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingEncoding;
                } else if lexer::byte(b';')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    state = State::SeenSemicolon;
                }
            }
        }
    }
    may_update_best_result(&mut cur_result, &mut best_result);
    best_result.take()
}

fn may_update_best_result(
    cur_result: &mut Option<EncodingMatch>,
    best_result: &mut Option<EncodingMatch>,
) {
    if cur_result.gt(&best_result) {
        *best_result = cur_result.take();
    }
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

#[derive(Debug)]
enum State {
    SearchingEncoding,
    SeenEncoding,
    SeenSemicolon,
    SeenParameterName,
    SeenEqual,
    SeenParameterValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_for_encoding_gzip_deflate_br_to_br() {
        assert_eq!(
            Some(EncodingMatch {
                match_type: EncodingMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_encoding(b"gzip, deflate, br", b"br")
        );
    }

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
