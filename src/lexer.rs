use crate::q_value::{QValue, Q_VALUE_FRAC_MAX_DIGITS};

pub(crate) type Position = usize;

#[derive(Debug, PartialEq)]
pub(crate) enum LexerToken<'a> {
    Token(&'a [u8]),
    DoubleQuotedString(&'a [u8]),
    Comma,
    Semicolon,
    Equal,
    QValue(QValue),
    Slash,
}

pub(crate) fn ows(input: &[u8], pos: Position) -> Position {
    let mut pos = pos;
    while pos < input.len() {
        match input[pos] {
            b' ' | b'\t' => pos += 1,
            _ => break,
        }
    }
    pos
}

pub(crate) fn slash<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    if pos < input.len() && input[pos] == b'/' {
        (pos + 1, Some(LexerToken::Slash))
    } else {
        (pos, None)
    }
}

pub(crate) fn comma<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    if pos < input.len() && input[pos] == b',' {
        (pos + 1, Some(LexerToken::Comma))
    } else {
        (pos, None)
    }
}

pub(crate) fn semicolon<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    if pos < input.len() && input[pos] == b';' {
        (pos + 1, Some(LexerToken::Semicolon))
    } else {
        (pos, None)
    }
}

pub(crate) fn equal<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    if pos < input.len() && input[pos] == b'=' {
        (pos + 1, Some(LexerToken::Equal))
    } else {
        (pos, None)
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

pub(crate) fn token<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    let mut i = pos;
    while i < input.len() && is_tchar(input[i]) {
        i += 1
    }
    if i == pos {
        (i, None)
    } else {
        let v = &input[pos..i];
        (i, Some(LexerToken::Token(v)))
    }
}

pub(crate) fn parameter_value<'a>(
    input: &'a [u8],
    pos: Position,
) -> (Position, Option<LexerToken<'a>>) {
    if let (pos2, Some(v)) = token(input, pos) {
        (pos2, Some(v))
    } else {
        double_quoted_string(input, pos)
    }
}

pub(crate) fn double_quoted_string<'a>(
    input: &'a [u8],
    pos: Position,
) -> (Position, Option<LexerToken<'a>>) {
    let i = pos;
    if i < input.len() && input[i] == b'"' {
        let mut escaped = false;
        for i in i + 1..input.len() {
            if escaped {
                escaped = false;
            } else {
                let c = input[i];
                match c {
                    b'"' => {
                        let v = &input[pos..i + 1];
                        return (i + 1, Some(LexerToken::DoubleQuotedString(v)));
                    }
                    b'\\' => escaped = true,
                    _ => {}
                }
            }
        }
    }
    (i, None)
}

pub(crate) fn q_value<'a>(input: &'a [u8], pos: Position) -> (Position, Option<LexerToken<'a>>) {
    let mut i = pos;
    if i < input.len() {
        let mut millis: u16 = match input[i] {
            b'0' => 0,
            b'1' => 1,
            _ => return (i, None),
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
        return (
            i,
            Some(LexerToken::QValue(QValue::from_millis(millis).unwrap())),
        );
    }
    (i, None)
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
        assert_eq!(2, ows(b" \tfoo", 0));
        assert_eq!(0, ows(b"foo", 0));
    }

    #[test]
    fn test_comma() {
        assert_eq!((1, Some(LexerToken::Comma)), comma(b",", 0));
        assert_eq!((0, None), comma(b"a", 0));
    }

    #[test]
    fn test_token_or_value() {
        assert_eq!((3, Some(LexerToken::Token(b"foo"))), token(b"foo,", 0));
        assert_eq!((0, None), token(b",", 0));
    }

    #[test]
    fn test_double_quoted_string() {
        assert_eq!(
            (
                b"\"a, b\"".len(),
                Some(LexerToken::DoubleQuotedString(b"\"a, b\""))
            ),
            double_quoted_string(b"\"a, b\" , c", 0)
        );

        assert_eq!((0, None), double_quoted_string(b",", 0));

        // unclosed string
        assert_eq!((0, None), double_quoted_string(b"\"", 0));
    }

    #[test]
    fn test_q_value() {
        assert_eq!(
            (1, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1", 0)
        );
        assert_eq!(
            (2, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1.", 0)
        );
        assert_eq!(
            (3, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1.0", 0)
        );
        assert_eq!(
            (3, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1.01", 0)
        );
        assert_eq!(
            (5, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1.000", 0)
        );
        assert_eq!(
            (5, Some(LexerToken::QValue(QValue::try_from(1.0).unwrap()))),
            q_value(b"1.0000", 0)
        );
        assert_eq!(
            (1, Some(LexerToken::QValue(QValue::try_from(0.0).unwrap()))),
            q_value(b"0", 0)
        );
        assert_eq!(
            (2, Some(LexerToken::QValue(QValue::try_from(0.0).unwrap()))),
            q_value(b"0.", 0)
        );
        assert_eq!(
            (3, Some(LexerToken::QValue(QValue::try_from(0.8).unwrap()))),
            q_value(b"0.8", 0)
        );
        assert_eq!(
            (4, Some(LexerToken::QValue(QValue::try_from(0.82).unwrap()))),
            q_value(b"0.82", 0)
        );
        assert_eq!(
            (
                5,
                Some(LexerToken::QValue(QValue::try_from(0.823).unwrap()))
            ),
            q_value(b"0.8235", 0)
        );
    }
}
