use std::cmp::Ordering;

use encoding_matcher::EncodingMatcher;
use q_value::QValue;

pub mod c;
mod encoding_matcher;
mod lexer;
mod q_value;

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
