use ordered_float::NotNan;

use crate::{bytes_eq_ignore_case, MatchResult, MatchType, Token};

pub(crate) struct QValueFinder<'a> {
    lexer: Lexer<'a>,
    state: State,
    cur_result: Option<MatchResult>,
    best_result: Option<MatchResult>,
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
        self.lexer.ows();
        while !self.lexer.eof() {
            match self.state {
                State::SearchingEncoding => {
                    if let Some(Token::TokenOrValue(tok_or_val)) = self.lexer.token_or_value() {
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
                    } else {
                        return None;
                    }
                }
                State::SeenSomeEncoding => {
                    if let Some(Token::Semicolon) = self.lexer.semicolon() {
                        self.state = State::SeenSemicolon;
                    } else if let Some(Token::Comma) = self.lexer.comma() {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else {
                        return None;
                    }
                }
                State::SeenSemicolon => {
                    if let Some(Token::TokenOrValue(tok_or_val)) = self.lexer.token_or_value() {
                        is_q_param = tok_or_val == b"q";
                        self.state = State::SeenParameterName;
                    } else {
                        return None;
                    }
                }
                State::SeenParameterName => {
                    if Some(Token::Equal) == self.lexer.equal() {
                        self.state = State::SeenEqual;
                    } else {
                        return None;
                    }
                }
                State::SeenEqual => {
                    if is_q_param {
                        if let Some(Token::QValue(q)) = self.lexer.q_value() {
                            if let Some(cur_result) = self.cur_result.as_mut() {
                                cur_result.q = q;
                            }
                        } else {
                            return None;
                        }
                    } else {
                        if self.lexer.parameter_value().is_none() {
                            return None;
                        }
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => {
                    if let Some(Token::Comma) = self.lexer.comma() {
                        self.may_update_best_result();
                        self.state = State::SearchingEncoding;
                    } else if let Some(Token::Semicolon) = self.lexer.semicolon() {
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

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn ows(&mut self) {
        ows(self.input, &mut self.pos)
    }

    fn comma(&mut self) -> Option<Token> {
        comma(self.input, &mut self.pos)
    }

    fn semicolon(&mut self) -> Option<Token> {
        semicolon(self.input, &mut self.pos)
    }

    fn equal(&mut self) -> Option<Token> {
        equal(self.input, &mut self.pos)
    }

    fn token_or_value(&mut self) -> Option<Token> {
        token_or_value(self.input, &mut self.pos)
    }

    fn q_value(&mut self) -> Option<Token> {
        q_value(self.input, &mut self.pos)
    }

    fn parameter_value(&mut self) -> Option<Token> {
        if let Some(v) = token_or_value(self.input, &mut self.pos) {
            Some(v)
        } else if let Some(v) = double_quoted_string(self.input, &mut self.pos) {
            Some(v)
        } else {
            None
        }
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

const Q_VALUE_FRAC_MAX_DIGITS: u32 = 3;

fn q_value<'a>(input: &'a [u8], pos: &mut usize) -> Option<Token<'a>> {
    let mut i = *pos;
    if i < input.len() {
        let int_part = match input[i] {
            b'0' => 0,
            b'1' => 1,
            _ => return None,
        };
        i += 1;
        let mut frac_part: i32 = 0;
        if i < input.len() && input[i] == b'.' {
            i += 1;
            if int_part == 0 {
                let mut done = false;
                for _ in 0..Q_VALUE_FRAC_MAX_DIGITS as usize {
                    frac_part *= 10;
                    if !done && i < input.len() {
                        let c = input[i];
                        match c {
                            b'0'..=b'9' => {
                                frac_part += (c - b'0') as i32;
                                i += 1;
                            }
                            _ => done = true,
                        }
                    }
                }
            } else {
                for _ in 0..Q_VALUE_FRAC_MAX_DIGITS as usize {
                    if i < input.len() && input[i] == b'0' {
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        let q = if frac_part == 0 {
            int_part as f32
        } else {
            int_part as f32 + frac_part as f32 / 10_i32.pow(Q_VALUE_FRAC_MAX_DIGITS) as f32
        };
        *pos = i;
        return Some(Token::QValue(NotNan::new(q).unwrap()));
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

    #[test]
    fn test_q_value() {
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1", &mut pos)
            );
            assert_eq!(1, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1.", &mut pos)
            );
            assert_eq!(2, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1.0", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1.01", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1.000", &mut pos)
            );
            assert_eq!(5, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(1.0).unwrap())),
                q_value(b"1.0000", &mut pos)
            );
            assert_eq!(5, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(0.0).unwrap())),
                q_value(b"0", &mut pos)
            );
            assert_eq!(1, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(0.0).unwrap())),
                q_value(b"0.", &mut pos)
            );
            assert_eq!(2, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(0.8).unwrap())),
                q_value(b"0.8", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(0.82).unwrap())),
                q_value(b"0.82", &mut pos)
            );
            assert_eq!(4, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(Token::QValue(NotNan::new(0.823).unwrap())),
                q_value(b"0.8235", &mut pos)
            );
            assert_eq!(5, pos);
        }
    }
}
