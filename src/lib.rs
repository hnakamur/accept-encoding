use std::ops::Range;

use http::HeaderValue;

pub fn resolve_q_value(value: HeaderValue, encoding: &[u8]) -> Option<f32> {
    let mut resolver = QValueResolver::new(value);
    resolver.resolve(encoding)
}

struct QValueResolver {
    lexer: Lexer,
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

impl QValueResolver {
    fn new(value: HeaderValue) -> Self {
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
            dbg!(token);
            match self.state {
                State::SearchingEncoding => {
                    self.state = State::SeenSomeEncoding;
                    self.cur_result = if bytes_eq_ignore_case(token, encoding)
                        || (is_gzip && bytes_eq_ignore_case(token, b"x-gzip"))
                        || (is_compress && bytes_eq_ignore_case(token, b"x-compress"))
                    {
                        Some(MatchResult {
                            wildcard: false,
                            q: 1.0,
                        })
                    } else if token == b"*" {
                        Some(MatchResult {
                            wildcard: true,
                            q: 1.0,
                        })
                    } else {
                        None
                    }
                }
                State::SeenSomeEncoding => {
                    if token == b";" {
                        self.state = State::SeenSemicolon;
                    } else if token == b"," {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    }
                }
                State::SeenSemicolon => {
                    is_q_param = token == b"q";
                    self.state = State::SeenParameterName;
                }
                State::SeenParameterName => {
                    if token == b"=" {
                        self.state = State::SeenEqual;
                    } else {
                        panic!("equal sign expected after parameter name");
                    }
                }
                State::SeenEqual => {
                    if let Some(cur_result) = self.cur_result.as_mut() {
                        if is_q_param {
                            cur_result.q = unsafe { std::str::from_utf8_unchecked(token) }
                                .parse::<f32>()
                                .unwrap()
                                .clamp(0.0, 1.0);
                        }
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => {
                    if token == b"," {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else if token == b";" {
                        self.state = State::SeenSemicolon;
                    } else {
                        panic!("comma or semicolon expected after parameter value");
                    }
                }
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

struct Lexer {
    value: HeaderValue,
    pos: usize,
    in_quoted_str: bool,
    quoted_str_escaped: bool,
    token_range: Option<Range<usize>>,
}

impl Lexer {
    fn new(value: HeaderValue) -> Self {
        Self {
            value,
            pos: 0,
            in_quoted_str: false,
            quoted_str_escaped: false,
            token_range: None,
        }
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        let value = self.value.as_bytes();
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
                            return Some(token);
                        }
                        b'\\' => self.quoted_str_escaped = true,
                        _ => {}
                    }
                }
            } else {
                match c {
                    b',' | b';' | b'=' => {
                        let token = if let Some(range) = self.token_range.take() {
                            &value[range.start..range.end]
                        } else {
                            let token = &value[self.pos..self.pos + 1];
                            self.pos = self.pos + 1;
                            token
                        };
                        return Some(token);
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
            Some(&value[range.start..range.end])
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
        let header = HeaderValue::from_str("*").unwrap();
        let q = resolve_q_value(header, b"gzip");
        assert!(q.is_some());
        assert_eq!(1.0, q.unwrap());

        let header = HeaderValue::from_str("*  ; q=0.5").unwrap();
        let q = resolve_q_value(header, b"gzip");
        assert!(q.is_some());
        assert_eq!(0.5, q.unwrap());

        let header = HeaderValue::from_str("br  ; q=1").unwrap();
        let q = resolve_q_value(header, b"gzip");
        assert!(q.is_none());

        let header = HeaderValue::from_str("br  ; q=0.9 , gzip;q=0.8").unwrap();
        let q = resolve_q_value(header, b"gzip");
        assert!(q.is_some());
        assert_eq!(0.8, q.unwrap());
        let header = HeaderValue::from_str("br  ; q=0.9 , gzip;q=0.8").unwrap();
        let q = resolve_q_value(header, b"br");
        assert!(q.is_some());
        assert_eq!(0.9, q.unwrap());
    }

    #[test]
    fn test_lexer_just_comma() {
        let header = HeaderValue::from_str(",").unwrap();
        let mut lexer = Lexer::new(header);
        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b",", tok.as_ref().unwrap());
        assert!(lexer.next_token().is_none());
    }

    #[test]
    fn test_lexer_quoted_string() {
        let header = HeaderValue::from_str(" foo  ;a=\"bar, \\\"baz\"; q=1, bar ").unwrap();
        let mut lexer = Lexer::new(header);

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"foo", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b";", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"a", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"=", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"\"bar, \\\"baz\"", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b";", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"q", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"=", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"1", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b",", tok.as_ref().unwrap());

        let tok = lexer.next_token();
        assert!(tok.is_some());
        assert_eq!(b"bar", tok.as_ref().unwrap());

        assert!(lexer.next_token().is_none());
    }

    #[test]
    fn test_bytes_eq_ignore_case() {
        assert!(bytes_eq_ignore_case(b"gzip", b"gzip"));
        assert!(bytes_eq_ignore_case(b"gzip", b"GZip"));
        assert!(bytes_eq_ignore_case(b"bzip2", b"bziP2"));

        assert!(!bytes_eq_ignore_case(b"gzip", b"zip"));
        assert!(!bytes_eq_ignore_case(b"gzip", b"gzi2"));
    }

    #[test]
    fn test_bytes_eq() {
        let bytes: &[u8] = &[b'*', b'*'];
        assert!(b"**" == bytes);
    }
}
