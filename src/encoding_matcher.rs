use crate::{
    lexer::{Lexer, LexerToken},
    q_value::QValue,
    EncodingMatch, MatchType,
};

pub(crate) struct EncodingMatcher<'a> {
    lexer: Lexer<'a>,
    state: State,
    cur_result: Option<EncodingMatch>,
    best_result: Option<EncodingMatch>,
}

#[derive(Debug)]
enum State {
    SearchingEncoding,
    SeenSomeEncoding,
    SeenSemicolon,
    SeenParameterName,
    SeenEqual,
    SeenParameterValue,
}

impl<'a> EncodingMatcher<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub(crate) fn match_encoding(&mut self, encoding: &[u8]) -> Option<EncodingMatch> {
        let is_gzip = bytes_eq_ignore_case(encoding, b"gzip");
        let is_compress = bytes_eq_ignore_case(encoding, b"compress");

        let mut is_q_param = false;
        self.lexer.ows();
        while !self.lexer.eof() {
            match self.state {
                State::SearchingEncoding => {
                    if let Some(LexerToken::Token(tok_or_val)) = self.lexer.token() {
                        self.cur_result = if bytes_eq_ignore_case(tok_or_val, encoding)
                            || (is_gzip && bytes_eq_ignore_case(tok_or_val, b"x-gzip"))
                            || (is_compress && bytes_eq_ignore_case(tok_or_val, b"x-compress"))
                        {
                            Some(EncodingMatch {
                                match_type: MatchType::Exact,
                                q: QValue::from_millis(1000).unwrap(),
                            })
                        } else if tok_or_val == b"*" {
                            Some(EncodingMatch {
                                match_type: MatchType::Wildcard,
                                q: QValue::from_millis(1000).unwrap(),
                            })
                        } else {
                            None
                        };
                        self.state = State::SeenSomeEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenSomeEncoding => {
                    if let Some(LexerToken::Semicolon) = self.lexer.semicolon() {
                        self.state = State::SeenSemicolon;
                    } else if let Some(LexerToken::Comma) = self.lexer.comma() {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenSemicolon => {
                    if let Some(LexerToken::Token(tok_or_val)) = self.lexer.token() {
                        is_q_param = tok_or_val == b"q";
                        self.state = State::SeenParameterName;
                    } else {
                        return None;
                    }
                }
                State::SeenParameterName => {
                    if Some(LexerToken::Equal) == self.lexer.equal() {
                        self.state = State::SeenEqual;
                    } else {
                        return None;
                    }
                }
                State::SeenEqual => {
                    if is_q_param {
                        if let Some(LexerToken::QValue(q)) = self.lexer.q_value() {
                            if let Some(cur_result) = self.cur_result.as_mut() {
                                cur_result.q = q;
                            }
                        } else {
                            return None;
                        }
                    } else if self.lexer.parameter_value().is_none() {
                        return None;
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => {
                    if let Some(LexerToken::Comma) = self.lexer.comma() {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else if let Some(LexerToken::Semicolon) = self.lexer.semicolon() {
                        self.state = State::SeenSemicolon;
                    } else {
                        return None;
                    }
                }
            }
            self.lexer.ows();
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
        b1_not_upper.is_ascii_lowercase() && b1_not_upper == b2_not_upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_eq_ignore_case() {
        assert!(bytes_eq_ignore_case(b"gzip", b"gzip"));
        assert!(bytes_eq_ignore_case(b"gzip", b"GZip"));
        assert!(bytes_eq_ignore_case(b"bzip2", b"bziP2"));

        assert!(!bytes_eq_ignore_case(b"gzip", b"zip"));
        assert!(!bytes_eq_ignore_case(b"gzip", b"gzi2"));
    }
}
