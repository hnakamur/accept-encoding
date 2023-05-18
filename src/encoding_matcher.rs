use std::cmp::Ordering;

use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer::{self, LexerToken},
    q_value::QValue,
};

pub fn match_for_encoding(input: &[u8], encoding: &[u8]) -> Option<EncodingMatch> {
    let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
    let is_compress = bytes_eq_ignore_case(encoding, b"compress");

    let mut state: State = State::SearchingEncoding;
    let mut cur_result: Option<EncodingMatch> = None;
    let mut best_result: Option<EncodingMatch> = None;
    let mut is_q_param = false;
    let mut pos: usize = 0;
    while pos < input.len() {
        match state {
            State::SearchingEncoding => {
                if let (pos2, Some(LexerToken::Token(tok_or_val))) = lexer::token(input, pos) {
                    pos = pos2;
                    cur_result = if bytes_eq_ignore_case(tok_or_val, encoding)
                        || (is_gzip && bytes_eq_ignore_case(tok_or_val, b"x-gzip"))
                        || (is_compress && bytes_eq_ignore_case(tok_or_val, b"x-compress"))
                    {
                        Some(EncodingMatch {
                            match_type: EncodingMatchType::Exact,
                            q: QValue::from_millis(1000).unwrap(),
                        })
                    } else if tok_or_val == b"*" {
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
                pos = lexer::ows(input, pos);
                if let (pos2, Some(LexerToken::Semicolon)) = lexer::semicolon(input, pos) {
                    pos = lexer::ows(input, pos2);
                    state = State::SeenSemicolon;
                } else if let (pos2, Some(LexerToken::Comma)) = lexer::comma(input, pos) {
                    pos = lexer::ows(input, pos2);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingEncoding;
                } else {
                    return None;
                }
            }
            State::SeenSemicolon => {
                if let (pos2, Some(LexerToken::Token(param_name))) = lexer::token(input, pos) {
                    pos = pos2;
                    is_q_param = bytes_eq_ignore_case(param_name, b"q");
                    state = State::SeenParameterName;
                } else {
                    return None;
                }
            }
            State::SeenParameterName => {
                if (pos + 1, Some(LexerToken::Equal)) == lexer::equal(input, pos) {
                    pos += 1;
                    state = State::SeenEqual;
                } else {
                    return None;
                }
            }
            State::SeenEqual => {
                if is_q_param {
                    if let (pos2, Some(LexerToken::QValue(q))) = lexer::q_value(input, pos) {
                        pos = pos2;
                        if let Some(cur_result) = cur_result.as_mut() {
                            cur_result.q = q;
                        }
                    } else {
                        return None;
                    }
                } else if let (pos2, Some(_)) = lexer::parameter_value(input, pos) {
                    pos = pos2;
                } else {
                    return None;
                }
                state = State::SeenParameterValue;
            }
            State::SeenParameterValue => {
                pos = lexer::ows(input, pos);
                if let (pos2, Some(LexerToken::Comma)) = lexer::comma(input, pos) {
                    pos = lexer::ows(input, pos2);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingEncoding;
                } else if let (pos2, Some(LexerToken::Semicolon)) = lexer::semicolon(input, pos) {
                    pos = lexer::ows(input, pos2);
                    state = State::SeenSemicolon;
                } else {
                    return None;
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
