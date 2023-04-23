use crate::{
    lexer::{Lexer, LexerToken},
    q_value::QValue,
    MimeTypeMatch, MimeTypeMatchType,
};

pub(crate) struct MimeTypeMatcher<'a> {
    lexer: Lexer<'a>,
    state: State,
    cur_result: Option<MimeTypeMatch>,
    best_result: Option<MimeTypeMatch>,
}

#[derive(Debug)]
enum State {
    SearchingMainType,
    SeenMainType,
    SeenSlash,
    SeenSubType,
    SeenSemicolon,
    SeenParameterName,
    SeenEqual,
    SeenParameterValue,
}

impl<'a> MimeTypeMatcher<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingMainType,
            cur_result: None,
            best_result: None,
        }
    }

    pub(crate) fn match_mime_type(&mut self, mime_type: &[u8]) -> Option<MimeTypeMatch> {
        let (want_main_type, want_subtype) = match split_mime_type(mime_type) {
            Some((main_type, subtype)) => (main_type, subtype),
            None => return None,
        };

        let mut cur_main_type = None;
        let mut is_q_param = false;
        while !self.lexer.eof() {
            match self.state {
                State::SearchingMainType => {
                    if let Some(LexerToken::Token(token)) = self.lexer.token() {
                        cur_main_type = Some(token);
                        self.state = State::SeenMainType;
                    } else {
                        return None;
                    }
                }
                State::SeenMainType => {
                    if let Some(LexerToken::Semicolon) = self.lexer.slash() {
                        self.state = State::SeenSlash;
                    } else {
                        return None;
                    }
                }
                State::SeenSlash => {
                    if let Some(LexerToken::Token(subtype)) = self.lexer.token() {
                        let main_type = cur_main_type.take().unwrap();
                        if let Some(match_type) = get_mime_type_match_type(
                            main_type,
                            subtype,
                            want_main_type,
                            want_subtype,
                        ) {
                            self.cur_result = Some(MimeTypeMatch {
                                match_type,
                                q: QValue::from_millis(1000).unwrap(),
                            })
                        }
                        self.state = State::SeenSubType;
                    } else {
                        return None;
                    }
                }
                State::SeenSubType => {
                    self.lexer.ows();
                    if let Some(LexerToken::Semicolon) = self.lexer.semicolon() {
                        self.lexer.ows();
                        self.state = State::SeenSemicolon;
                    } else if let Some(LexerToken::Comma) = self.lexer.comma() {
                        self.lexer.ows();
                        self.state = State::SearchingMainType;
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
                    self.lexer.ows();
                    if let Some(LexerToken::Comma) = self.lexer.comma() {
                        self.lexer.ows();
                        self.may_update_best_result();
                        self.state = State::SearchingMainType;
                    } else if let Some(LexerToken::Semicolon) = self.lexer.semicolon() {
                        self.lexer.ows();
                        self.state = State::SeenSemicolon;
                    } else {
                        return None;
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

fn split_mime_type(mime_type: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut s = mime_type.splitn(2, |c| *c == b'/');
    match s.next() {
        Some(main_type) => s.next().map(|subtype| (main_type, subtype)),
        None => None,
    }
}

fn get_mime_type_match_type(
    main_type: &[u8],
    subtype: &[u8],
    want_main_type: &[u8],
    want_subtype: &[u8],
) -> Option<MimeTypeMatchType> {
    if main_type == b"*" {
        if subtype == b"*" {
            Some(MimeTypeMatchType::MainTypeWildcard)
        } else {
            None
        }
    } else if main_type == want_main_type {
        if subtype == want_subtype {
            Some(MimeTypeMatchType::Exact)
        } else if subtype == b"*" {
            Some(MimeTypeMatchType::SubTypeWildcard)
        } else {
            None
        }
    } else {
        None
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
    fn test_split_mime_type() {
        assert_eq!(
            Some((b"image".as_slice(), b"webp".as_slice())),
            split_mime_type(b"image/webp")
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
