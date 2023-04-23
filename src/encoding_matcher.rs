use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer::{Lexer, LexerToken},
    q_value::QValue,
    EncodingMatch, EncodingMatchType,
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
    SeenEncoding,
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
        while !self.lexer.eof() {
            match self.state {
                State::SearchingEncoding => {
                    if let Some(LexerToken::Token(tok_or_val)) = self.lexer.token() {
                        self.cur_result = if bytes_eq_ignore_case(tok_or_val, encoding)
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
                        self.state = State::SeenEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenEncoding => {
                    self.lexer.ows();
                    if let Some(LexerToken::Semicolon) = self.lexer.semicolon() {
                        self.lexer.ows();
                        self.state = State::SeenSemicolon;
                    } else if let Some(LexerToken::Comma) = self.lexer.comma() {
                        self.lexer.ows();
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenSemicolon => {
                    if let Some(LexerToken::Token(param_name)) = self.lexer.token() {
                        is_q_param = bytes_eq_ignore_case(param_name, b"q");
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
                        self.state = State::SearchingEncoding;
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
