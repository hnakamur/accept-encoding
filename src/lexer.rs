use ordered_float::NotNan;

use crate::{bytes_eq_ignore_case, MatchResult, MatchType, Token};

pub(crate) struct QValueFinder<'a> {
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
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(value),
            state: State::SearchingEncoding,
            cur_result: None,
            best_result: None,
        }
    }

    pub(crate) fn find(&mut self, encoding: &[u8]) -> Option<MatchResult> {
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

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn next_token(&mut self) -> Option<Token> {
        ows(self.input, &mut self.pos);
        if let Some(token) = token_or_value(self.input, &mut self.pos) {
            return Some(token);
        }
        if let Some(token) = double_quoted_string(self.input, &mut self.pos) {
            return Some(token);
        }
        if let Some(token) = comma(self.input, &mut self.pos) {
            return Some(token);
        }
        if let Some(token) = semicolon(self.input, &mut self.pos) {
            return Some(token);
        }
        if let Some(token) = equal(self.input, &mut self.pos) {
            return Some(token);
        }
        None
    }
}

fn ows(input: &[u8], pos: &mut usize) {
    while *pos < input.len() {
        match input[*pos] {
            b' ' | b'\t' => *pos += 1,
            _ => return,
        }
    }
}

fn comma<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    if *pos < input.len() && input[*pos] == b',' {
        *pos += 1;
        Some(Token::Comma)
    } else {
        None
    }
}

fn semicolon<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    if *pos < input.len() && input[*pos] == b';' {
        *pos += 1;
        Some(Token::Semicolon)
    } else {
        None
    }
}

fn equal<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    if *pos < input.len() && input[*pos] == b'=' {
        *pos += 1;
        Some(Token::Equal)
    } else {
        None
    }
}

fn token_or_value<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    let mut i = *pos;
    while i < input.len() {
        match input[i] {
            b' ' | b'\t' | b',' | b';' | b'=' | b'"' => break,
            _ => i += 1,
        }
    }
    if i == *pos {
        None
    } else {
        let v = &input[*pos..i];
        *pos = i;
        Some(Token::TokenOrValue(v))
    }
}

fn double_quoted_string<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    let i = *pos;
    if i < input.len() && input[i] == b'"' {
        let mut escaped = false;
        for i in i + 1..input.len() {
            if escaped {
                escaped = false;
            } else {
                let c = input[i];
                match c {
                    b'"' => {
                        let v = &input[*pos..i + 1];
                        *pos = i + 1;
                        return Some(Token::DoubleQuotedString(v));
                    }
                    b'\\' => escaped = true,
                    _ => {}
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ows() {
        {
            let input = b" \tfoo";
            let mut pos = 0;
            ows(input, &mut pos);
            assert_eq!(2, pos);
        }
        {
            let input = b"foo";
            let mut pos = 0;
            ows(input, &mut pos);
            assert_eq!(0, pos);
        }
    }

    #[test]
    fn test_comma() {
        {
            let mut pos = 0;
            assert_eq!(Some(Token::Comma), comma(b",", &mut pos));
            assert_eq!(1, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(None, comma(b"a", &mut pos));
            assert_eq!(0, pos);
        }
    }

    #[test]
    fn test_token_or_value() {
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::TokenOrValue(b"foo")),
                token_or_value(b"foo,", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(None, token_or_value(b",", &mut pos));
            assert_eq!(0, pos);
        }
    }

    #[test]
    fn test_double_quoted_string() {
        {
            let mut pos = 0;
            let expected = b"\"a, b\"";
            assert_eq!(
                Some(Token::DoubleQuotedString(expected)),
                double_quoted_string(b"\"a, b\" , c", &mut pos)
            );
            assert_eq!(expected.len(), pos);
        }
        {
            let mut pos = 0;
            assert_eq!(None, double_quoted_string(b",", &mut pos));
            assert_eq!(0, pos);
        }
        {
            // unclosed string
            let mut pos = 0;
            assert_eq!(None, double_quoted_string(b"\"", &mut pos));
            assert_eq!(0, pos);
        }
    }
}
