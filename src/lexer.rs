use crate::q_value::{QValue, Q_VALUE_FRAC_MAX_DIGITS};

pub(crate) struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) enum LexerToken<'a> {
    Token(&'a [u8]),
    DoubleQuotedString(&'a [u8]),
    Comma,
    Semicolon,
    Equal,
    QValue(QValue),
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub(crate) fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    pub(crate) fn ows(&mut self) {
        ows(self.input, &mut self.pos)
    }

    pub(crate) fn comma(&mut self) -> Option<LexerToken> {
        comma(self.input, &mut self.pos)
    }

    pub(crate) fn semicolon(&mut self) -> Option<LexerToken> {
        semicolon(self.input, &mut self.pos)
    }

    pub(crate) fn equal(&mut self) -> Option<LexerToken> {
        equal(self.input, &mut self.pos)
    }

    pub(crate) fn token(&mut self) -> Option<LexerToken> {
        token(self.input, &mut self.pos)
    }

    pub(crate) fn q_value(&mut self) -> Option<LexerToken> {
        q_value(self.input, &mut self.pos)
    }

    pub(crate) fn parameter_value(&mut self) -> Option<LexerToken> {
        if let Some(v) = token(self.input, &mut self.pos) {
            Some(v)
        } else {
            double_quoted_string(self.input, &mut self.pos)
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

fn comma<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
    if *pos < input.len() && input[*pos] == b',' {
        *pos += 1;
        Some(LexerToken::Comma)
    } else {
        None
    }
}

fn semicolon<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
    if *pos < input.len() && input[*pos] == b';' {
        *pos += 1;
        Some(LexerToken::Semicolon)
    } else {
        None
    }
}

fn equal<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
    if *pos < input.len() && input[*pos] == b'=' {
        *pos += 1;
        Some(LexerToken::Equal)
    } else {
        None
    }
}

#[rustfmt::skip]
const TCHAR_TABLE: [bool; 256] = [
    // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
    //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, true,  false, true,  true,  true,  true,  true,  false, false, true,  true,  false, true,  true,  false,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, false, false, false, false, false,
    false, true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, false, false, true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, true,  false, true,  false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
];

#[inline]
fn is_tchar(c: u8) -> bool {
    TCHAR_TABLE[c as usize]
}

fn token<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
    let mut i = *pos;
    while i < input.len() && is_tchar(input[i]) {
        i += 1
    }
    if i == *pos {
        None
    } else {
        let v = &input[*pos..i];
        *pos = i;
        Some(LexerToken::Token(v))
    }
}

fn double_quoted_string<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
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
                        return Some(LexerToken::DoubleQuotedString(v));
                    }
                    b'\\' => escaped = true,
                    _ => {}
                }
            }
        }
    }
    None
}

fn q_value<'a>(input: &'a [u8], pos: &mut usize) -> Option<LexerToken<'a>> {
    let mut i = *pos;
    if i < input.len() {
        let mut millis: u16 = match input[i] {
            b'0' => 0,
            b'1' => 1,
            _ => return None,
        };
        i += 1;
        let mut frac_start = i;
        if i < input.len() && input[i] == b'.' {
            i += 1;
            frac_start = i;
            if millis == 0 {
                for _ in 0..Q_VALUE_FRAC_MAX_DIGITS as usize {
                    if i < input.len() {
                        let c = input[i];
                        match c {
                            b'0'..=b'9' => {
                                millis *= 10;
                                millis += (c - b'0') as u16;
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
            } else {
                for _ in 0..Q_VALUE_FRAC_MAX_DIGITS as usize {
                    if i < input.len() && input[i] == b'0' {
                        millis *= 10;
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        for _ in i - frac_start..Q_VALUE_FRAC_MAX_DIGITS as usize {
            millis *= 10;
        }
        *pos = i;
        return Some(LexerToken::QValue(QValue::from_millis(millis).unwrap()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_tchar_ref_impl(c: u8) -> bool {
        match c {
            // token = 1*tchar
            // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
            //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
            b'!'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z' => true,
            _ => false,
        }
    }

    #[test]
    fn test_is_tchar() {
        for c in 0..=b'\xFF' {
            assert_eq!(is_tchar_ref_impl(c), is_tchar(c));
        }
    }

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
            assert_eq!(Some(LexerToken::Comma), comma(b",", &mut pos));
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
            assert_eq!(Some(LexerToken::Token(b"foo")), token(b"foo,", &mut pos));
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(None, token(b",", &mut pos));
            assert_eq!(0, pos);
        }
    }

    #[test]
    fn test_double_quoted_string() {
        {
            let mut pos = 0;
            let expected = b"\"a, b\"";
            assert_eq!(
                Some(LexerToken::DoubleQuotedString(expected)),
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
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1", &mut pos)
            );
            assert_eq!(1, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1.", &mut pos)
            );
            assert_eq!(2, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1.0", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1.01", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1.000", &mut pos)
            );
            assert_eq!(5, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(1.0).unwrap())),
                q_value(b"1.0000", &mut pos)
            );
            assert_eq!(5, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(0.0).unwrap())),
                q_value(b"0", &mut pos)
            );
            assert_eq!(1, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(0.0).unwrap())),
                q_value(b"0.", &mut pos)
            );
            assert_eq!(2, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(0.8).unwrap())),
                q_value(b"0.8", &mut pos)
            );
            assert_eq!(3, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(0.82).unwrap())),
                q_value(b"0.82", &mut pos)
            );
            assert_eq!(4, pos);
        }
        {
            let mut pos = 0;
            assert_eq!(
                Some(LexerToken::QValue(QValue::try_from(0.823).unwrap())),
                q_value(b"0.8235", &mut pos)
            );
            assert_eq!(5, pos);
        }
    }
}
