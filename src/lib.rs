use std::cmp::Ordering;

use ordered_float::NotNan;

use lexer::QValueFinder;

pub mod c;
mod lexer;
mod monolith_lexer;

pub fn match_for_encoding(header_value: &[u8], encoding: &[u8]) -> Option<MatchResult> {
    QValueFinder::new(header_value).find(encoding)
}

pub fn match_for_encoding_monolith_for_benchmark(header_value: &[u8], encoding: &[u8]) -> Option<MatchResult> {
    monolith_lexer::QValueFinder::new(header_value).find(encoding)
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
