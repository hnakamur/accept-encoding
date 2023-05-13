#[derive(Debug, PartialEq)]
pub(crate) struct ParseError(Cursor);

pub(crate) type ParseResult = Result<Cursor, ParseError>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct Cursor(pub usize);

impl Cursor {
    #[inline]
    pub fn eof(&self, input: &[u8]) -> bool {
        self.0 >= input.len()
    }

    #[inline]
    pub fn peek(&self, input: &[u8]) -> Option<u8> {
        if self.0 < input.len() {
            Some(input[self.0])
        } else {
            None
        }
    }

    #[inline]
    pub fn advanced(&self, n: usize) -> Self {
        Self(self.0 + n)
    }

    #[inline]
    pub fn slice<'a>(&self, input: &'a [u8], end: Cursor) -> &'a [u8] {
        &input[self.0..end.0]
    }
}

pub(crate) fn byte(b: u8) -> impl Fn(&[u8], Cursor) -> ParseResult {
    move |input: &[u8], c: Cursor| {
        if let Some(b2) = c.peek(input) {
            if b2 == b {
                return Ok(c.advanced(1));
            }
        }
        Err(ParseError(c))
    }
}

fn match_m_n<F>(pred: F, m: usize, n: usize) -> impl Fn(&[u8], Cursor) -> ParseResult
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: Cursor| {
        let mut c = c;
        let mut count = 0;
        while let Some(b) = c.peek(input) {
            if pred(b) {
                c = c.advanced(1);
                count += 1;
                if count == n {
                    return Ok(c);
                }
            } else {
                break;
            }
        }
        if count >= m {
            Ok(c)
        } else {
            Err(ParseError(c))
        }
    }
}

fn match_one_or_more<F>(pred: F) -> impl Fn(&[u8], Cursor) -> ParseResult
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: Cursor| {
        let mut c2 = c;
        while let Some(b) = c2.peek(input) {
            if pred(b) {
                c2 = c2.advanced(1);
            } else {
                break;
            }
        }
        if c2.0 > c.0 {
            Ok(c2)
        } else {
            Err(ParseError(c))
        }
    }
}

fn match_zero_or_more<F>(pred: F) -> impl Fn(&[u8], Cursor) -> ParseResult
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: Cursor| {
        let mut c = c;
        while let Some(b) = c.peek(input) {
            if pred(b) {
                c = c.advanced(1);
            } else {
                break;
            }
        }
        Ok(c)
    }
}

fn pair(
    parser1: impl Fn(&[u8], Cursor) -> ParseResult,
    parser2: impl Fn(&[u8], Cursor) -> ParseResult,
) -> impl Fn(&[u8], Cursor) -> ParseResult {
    move |input: &[u8], c: Cursor| {
        let c = parser1(input, c)?;
        parser2(input, c)
    }
}

fn opt(parser: impl Fn(&[u8], Cursor) -> ParseResult) -> impl Fn(&[u8], Cursor) -> ParseResult {
    move |input: &[u8], c: Cursor| match parser(input, c) {
        Ok(c) => Ok(c),
        Err(_) => Ok(c),
    }
}

pub(crate) fn alt(
    parser1: impl Fn(&[u8], Cursor) -> ParseResult,
    parser2: impl Fn(&[u8], Cursor) -> ParseResult,
) -> impl Fn(&[u8], Cursor) -> ParseResult {
    move |input: &[u8], c: Cursor| match parser1(input, c) {
        Ok(c) => Ok(c),
        Err(_) => parser2(input, c),
    }
}

fn escaped<F, G>(
    is_normal_char: F,
    escape_char: u8,
    is_escapable_char: G,
) -> impl Fn(&[u8], Cursor) -> ParseResult
where
    F: Fn(u8) -> bool,
    G: Fn(u8) -> bool,
{
    move |input: &[u8], c: Cursor| {
        let mut seen_escape_char = false;
        let mut c = c;
        while let Some(b) = c.peek(input) {
            if seen_escape_char {
                if is_escapable_char(b) {
                    c = c.advanced(1);
                    seen_escape_char = false;
                } else {
                    return Err(ParseError(c));
                }
            } else if is_normal_char(b) {
                c = c.advanced(1);
            } else if b == escape_char {
                c = c.advanced(1);
                seen_escape_char = true;
            } else {
                break;
            }
        }
        Ok(c)
    }
}

pub(crate) fn token(input: &[u8], c: Cursor) -> ParseResult {
    match_one_or_more(is_tchar)(input, c)
}

fn is_tchar(c: u8) -> bool {
    TCHAR_TABLE[c as usize]
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

pub(crate) fn quoted_string(input: &[u8], c: Cursor) -> ParseResult {
    let c = byte(b'"')(input, c)?;
    let c = escaped(is_qdtext, b'\\', is_quoted_pair_char)(input, c)?;
    byte(b'"')(input, c)
}

fn is_qdtext(c: u8) -> bool {
    QDTEXT_TABLE[c as usize]
}

#[rustfmt::skip]
const QDTEXT_TABLE: [bool; 256] = [
    // qdtext = HTAB / SP / "!" / %x23-5B ; '#'-'['
    //        / %x5D-7E ; ']'-'~'
    //        / obs-text
    // obs-text = %x80-FF
    false, false, false, false, false, false, false, false, false, true,  false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    true,  true,  false, true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
];

fn is_quoted_pair_char(c: u8) -> bool {
    QUOTED_PAIR_CHAR_TABLE[c as usize]
}

#[rustfmt::skip]
const QUOTED_PAIR_CHAR_TABLE: [bool; 256] = [
    // quoted-pair = "\" ( HTAB / SP / VCHAR / obs-text )
    // VCHAR          =  %x21-7E
    //                ; visible (printing) characters
    // obs-text = %x80-FF
    false, false, false, false, false, false, false, false, false, true,  false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
];

pub(crate) fn ows(input: &[u8], c: Cursor) -> ParseResult {
    match_zero_or_more(|b| b == b' ' || b == b'\t')(input, c)
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

pub(crate) fn q_value(input: &[u8], c: Cursor) -> ParseResult {
    alt(
        pair(byte(b'0'), opt(pair(byte(b'.'), match_m_n(is_digit, 0, 3)))),
        pair(
            byte(b'1'),
            opt(pair(byte(b'.'), match_m_n(|b| b == b'0', 0, 3))),
        ),
    )(input, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_tchar() {
        fn is_tchar_ref_impl(c: u8) -> bool {
            // token = 1*tchar
            // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
            //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
            matches!(c, b'!'
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
                | b'a'..=b'z')
        }

        for c in 0..=b'\xFF' {
            assert_eq!(is_tchar_ref_impl(c), is_tchar(c));
        }
    }

    #[test]
    fn test_is_qdtext() {
        fn is_qdtext_ref_impl(c: u8) -> bool {
            // qdtext = HTAB / SP / "!" / %x23-5B ; '#'-'['
            //        / %x5D-7E ; ']'-'~'
            //        / obs-text
            // obs-text = %x80-FF
            matches!(c, b'\t' | b' ' | b'!' | 0x23..=0x5B | 0x5D..=0x7E | 0x80..=0xFF)
        }

        for c in 0..=b'\xFF' {
            assert_eq!(is_qdtext_ref_impl(c), is_qdtext(c));
        }
    }

    #[test]
    fn test_is_quoted_pair_char() {
        fn is_quoted_pair_char_ref_impl(c: u8) -> bool {
            // quoted-pair = "\" ( HTAB / SP / VCHAR / obs-text )
            // VCHAR          =  %x21-7E
            //                ; visible (printing) characters
            // obs-text = %x80-FF
            matches!(c, b'\t' | b' ' | 0x21..=0x7E | 0x80..=0xFF)
        }

        for c in 0..=b'\xFF' {
            assert_eq!(is_quoted_pair_char_ref_impl(c), is_quoted_pair_char(c));
        }
    }

    #[test]
    fn test_token() {
        {
            let input = b"gzip";
            assert_eq!(Ok(Cursor(input.len())), token(input, Cursor(0)));
        }
        {
            let input = b"gzip, ";
            assert_eq!(Ok(Cursor(4)), token(input, Cursor(0)));
        }
        {
            let input = b"";
            assert_eq!(Err(ParseError(Cursor(0))), token(input, Cursor(0)));
        }
    }

    #[test]
    fn test_quoted_string() {
        {
            let input = br#""""#;
            assert_eq!(Ok(Cursor(input.len())), quoted_string(input, Cursor(0)));
        }
        {
            let input = br#""foo""#;
            assert_eq!(Ok(Cursor(input.len())), quoted_string(input, Cursor(0)));
        }
        {
            let input = br#""foo\\tbar""#;
            assert_eq!(Ok(Cursor(input.len())), quoted_string(input, Cursor(0)));
        }
        {
            let input = b"\"\\\"foo\\\"\"";
            assert_eq!(Ok(Cursor(input.len())), quoted_string(input, Cursor(0)));
        }
        {
            let input = b"\x00";
            assert_eq!(Err(ParseError(Cursor(0))), quoted_string(input, Cursor(0)));
        }
        {
            let input = b"\"\\\x00";
            assert_eq!(Err(ParseError(Cursor(2))), quoted_string(input, Cursor(0)));
        }
        {
            let input = b"";
            assert_eq!(Err(ParseError(Cursor(0))), quoted_string(input, Cursor(0)));
        }
    }

    #[test]
    fn test_pair() {
        fn dot_followed_by_at_most_three_zeros(input: &[u8], c: Cursor) -> ParseResult {
            pair(byte(b'.'), match_m_n(|b| b == b'0', 0, 3))(input, c)
        }

        {
            let input = b".";
            assert_eq!(
                Ok(Cursor(1)),
                dot_followed_by_at_most_three_zeros(input, Cursor(0))
            );
        }
        {
            let input = b".0";
            assert_eq!(
                Ok(Cursor(2)),
                dot_followed_by_at_most_three_zeros(input, Cursor(0))
            );
        }
        {
            let input = b".000";
            assert_eq!(
                Ok(Cursor(4)),
                dot_followed_by_at_most_three_zeros(input, Cursor(0))
            );
        }
        {
            let input = b".0000";
            assert_eq!(
                Ok(Cursor(4)),
                dot_followed_by_at_most_three_zeros(input, Cursor(0))
            );
        }
    }

    #[test]
    fn test_q_value() {
        {
            let input = b"0";
            assert_eq!(Ok(Cursor(1)), q_value(input, Cursor(0)));
        }
        {
            let input = b"0.";
            assert_eq!(Ok(Cursor(2)), q_value(input, Cursor(0)));
        }
        {
            let input = b"0.,";
            assert_eq!(Ok(Cursor(2)), q_value(input, Cursor(0)));
        }
        {
            let input = b"0.8";
            assert_eq!(Ok(Cursor(3)), q_value(input, Cursor(0)));
        }
        {
            let input = b"0.8,";
            assert_eq!(Ok(Cursor(3)), q_value(input, Cursor(0)));
        }
        {
            let input = b"0.1239";
            assert_eq!(Ok(Cursor(5)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1";
            assert_eq!(Ok(Cursor(1)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.";
            assert_eq!(Ok(Cursor(2)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.0";
            assert_eq!(Ok(Cursor(3)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.00";
            assert_eq!(Ok(Cursor(4)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.000";
            assert_eq!(Ok(Cursor(5)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.0000";
            assert_eq!(Ok(Cursor(5)), q_value(input, Cursor(0)));
        }
        {
            let input = b"1.1";
            assert_eq!(Ok(Cursor(2)), q_value(input, Cursor(0)));
        }
    }
}
