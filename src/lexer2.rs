use std::str;

use crate::q_value::QValue;

#[derive(Debug, PartialEq)]
pub(crate) struct ParseError;

pub(crate) type ParseResult<O> = Result<O, ParseError>;

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
    pub fn advance(&mut self, n: usize) {
        self.0 += n;
    }

    #[inline]
    pub fn slice<'a>(&self, input: &'a [u8], end: Cursor) -> &'a [u8] {
        &input[self.0..end.0]
    }
}

pub(crate) fn byte(b: u8) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()> {
    move |input: &[u8], c: &mut Cursor| {
        if let Some(b2) = c.peek(input) {
            if b2 == b {
                c.advance(1);
                return Ok(());
            }
        }
        Err(ParseError)
    }
}

fn match_m_n<F>(pred: F, m: usize, n: usize) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()>
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: &mut Cursor| {
        let mut count = 0;
        while let Some(b) = c.peek(input) {
            if pred(b) {
                c.advance(1);
                count += 1;
                if count == n {
                    return Ok(());
                }
            } else {
                break;
            }
        }
        if count >= m {
            Ok(())
        } else {
            Err(ParseError)
        }
    }
}

fn match_one_or_more<F>(pred: F) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()>
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: &mut Cursor| {
        let c0 = *c;
        while let Some(b) = c.peek(input) {
            if pred(b) {
                c.advance(1);
            } else {
                break;
            }
        }
        if c.0 > c0.0 {
            Ok(())
        } else {
            Err(ParseError)
        }
    }
}

fn match_zero_or_more<F>(pred: F) -> impl Fn(&[u8], &mut Cursor)
where
    F: Fn(u8) -> bool,
{
    move |input: &[u8], c: &mut Cursor| {
        while let Some(b) = c.peek(input) {
            if pred(b) {
                c.advance(1);
            } else {
                break;
            }
        }
    }
}

fn pair(
    parser1: impl Fn(&[u8], &mut Cursor) -> ParseResult<()>,
    parser2: impl Fn(&[u8], &mut Cursor) -> ParseResult<()>,
) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()> {
    move |input: &[u8], c: &mut Cursor| {
        parser1(input, c)?;
        parser2(input, c)
    }
}

fn opt(
    parser: impl Fn(&[u8], &mut Cursor) -> ParseResult<()>,
) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()> {
    move |input: &[u8], c: &mut Cursor| {
        let c0 = *c;
        match parser(input, c) {
            Ok(()) => Ok(()),
            Err(_) => {
                *c = c0;
                Ok(())
            }
        }
    }
}

pub(crate) fn alt(
    parser1: impl Fn(&[u8], &mut Cursor) -> ParseResult<()>,
    parser2: impl Fn(&[u8], &mut Cursor) -> ParseResult<()>,
) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()> {
    move |input: &[u8], c: &mut Cursor| {
        let c0 = *c;
        match parser1(input, c) {
            Ok(()) => Ok(()),
            Err(_) => {
                *c = c0;
                parser2(input, c)
            }
        }
    }
}

fn escaped<F, G>(
    is_normal_char: F,
    escape_char: u8,
    is_escapable_char: G,
) -> impl Fn(&[u8], &mut Cursor) -> ParseResult<()>
where
    F: Fn(u8) -> bool,
    G: Fn(u8) -> bool,
{
    move |input: &[u8], c: &mut Cursor| {
        let mut seen_escape_char = false;
        while let Some(b) = c.peek(input) {
            if seen_escape_char {
                if is_escapable_char(b) {
                    c.advance(1);
                    seen_escape_char = false;
                } else {
                    return Err(ParseError);
                }
            } else if is_normal_char(b) {
                c.advance(1);
            } else if b == escape_char {
                c.advance(1);
                seen_escape_char = true;
            } else {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) fn token<'a>(input: &'a [u8], c: &mut Cursor) -> ParseResult<&'a [u8]> {
    let c0 = *c;
    match_one_or_more(is_tchar)(input, c)?;
    Ok(c0.slice(input, *c))
}

pub(crate) fn skip_token(input: &[u8], c: &mut Cursor) -> ParseResult<()> {
    match_one_or_more(is_tchar)(input, c)
}

#[inline]
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

pub(crate) fn quoted_string(input: &[u8], c: &mut Cursor) -> ParseResult<()> {
    byte(b'"')(input, c)?;
    escaped(is_qdtext, b'\\', is_quoted_pair_char)(input, c)?;
    byte(b'"')(input, c)
}

#[inline]
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

#[inline]
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

pub(crate) fn ows(input: &[u8], c: &mut Cursor) {
    match_zero_or_more(|b| matches!(b, b' ' | b'\t'))(input, c)
}

#[inline]
fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

pub(crate) fn q_value(input: &[u8], c: &mut Cursor) -> ParseResult<QValue> {
    let c1 = *c;
    alt(
        pair(byte(b'0'), opt(pair(byte(b'.'), match_m_n(is_digit, 0, 3)))),
        pair(
            byte(b'1'),
            opt(pair(byte(b'.'), match_m_n(|b| b == b'0', 0, 3))),
        ),
    )(input, c)?;
    Ok(QValue::try_from(str::from_utf8(c1.slice(input, *c)).unwrap()).unwrap())
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
            let mut c = Cursor(0);
            assert_eq!(Ok(&b"gzip"[..]), token(input, &mut c));
            assert_eq!(Cursor(input.len()), c);
        }
        {
            let input = b"gzip, ";
            let mut c = Cursor(0);
            assert_eq!(Ok(&b"gzip"[..]), token(input, &mut c));
            assert_eq!(Cursor(4), c);
        }
        {
            let input = b"";
            let mut c = Cursor(0);
            assert_eq!(Err(ParseError), token(input, &mut c));
            assert_eq!(Cursor(0), c);
        }
    }

    #[test]
    fn test_quoted_string() {
        {
            let input = br#""""#;
            let mut c = Cursor(0);
            assert_eq!(Ok(()), quoted_string(input, &mut c));
            assert_eq!(Cursor(input.len()), c);
        }
        {
            let input = br#""foo""#;
            let mut c = Cursor(0);
            assert_eq!(Ok(()), quoted_string(input, &mut c));
            assert_eq!(Cursor(input.len()), c);
        }
        {
            let input = br#""foo\\tbar""#;
            let mut c = Cursor(0);
            assert_eq!(Ok(()), quoted_string(input, &mut c));
            assert_eq!(Cursor(input.len()), c);
        }
        {
            let input = b"\"\\\"foo\\\"\"";
            let mut c = Cursor(0);
            assert_eq!(Ok(()), quoted_string(input, &mut c));
            assert_eq!(Cursor(input.len()), c);
        }
        {
            let input = b"\x00";
            let mut c = Cursor(0);
            assert_eq!(Err(ParseError), quoted_string(input, &mut c));
            assert_eq!(Cursor(0), c);
        }
        {
            let input = b"\"\\\x00";
            let mut c = Cursor(0);
            assert_eq!(Err(ParseError), quoted_string(input, &mut c));
            assert_eq!(Cursor(2), c);
        }
        {
            let input = b"";
            let mut c = Cursor(0);
            assert_eq!(Err(ParseError), quoted_string(input, &mut c));
            assert_eq!(Cursor(0), c);
        }
    }

    #[test]
    fn test_pair() {
        fn dot_followed_by_at_most_three_zeros(input: &[u8], c: &mut Cursor) -> ParseResult<()> {
            pair(byte(b'.'), match_m_n(|b| b == b'0', 0, 3))(input, c)
        }

        {
            let input = b".";
            let mut c = Cursor(0);
            assert_eq!(Ok(()), dot_followed_by_at_most_three_zeros(input, &mut c));
            assert_eq!(Cursor(1), c);
        }
        {
            let input = b".0";
            let mut c = Cursor(0);
            assert_eq!(Ok(()), dot_followed_by_at_most_three_zeros(input, &mut c));
            assert_eq!(Cursor(2), c);
        }
        {
            let input = b".000";
            let mut c = Cursor(0);
            assert_eq!(Ok(()), dot_followed_by_at_most_three_zeros(input, &mut c));
            assert_eq!(Cursor(4), c);
        }
        {
            let input = b".0000";
            let mut c = Cursor(0);
            assert_eq!(Ok(()), dot_followed_by_at_most_three_zeros(input, &mut c));
            assert_eq!(Cursor(4), c);
        }
    }

    #[test]
    fn test_q_value() {
        {
            let input = b"0";
            let mut c = Cursor(0);
            assert_eq!(Ok(QValue::from_millis(0).unwrap()), q_value(input, &mut c));
            assert_eq!(Cursor(1), c);
        }
        {
            let input = b"0.";
            let mut c = Cursor(0);
            assert_eq!(Ok(QValue::from_millis(0).unwrap()), q_value(input, &mut c));
            assert_eq!(Cursor(2), c);
        }
        {
            let input = b"0.,";
            let mut c = Cursor(0);
            assert_eq!(Ok(QValue::from_millis(0).unwrap()), q_value(input, &mut c));
            assert_eq!(Cursor(2), c);
        }
        {
            let input = b"0.8";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(800).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(3), c);
        }
        {
            let input = b"0.8,";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(800).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(3), c);
        }
        {
            let input = b"0.1239";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(123).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(5), c);
        }
        {
            let input = b"1";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(1), c);
        }
        {
            let input = b"1.";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(2), c);
        }
        {
            let input = b"1.0";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(3), c);
        }
        {
            let input = b"1.00";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(4), c);
        }
        {
            let input = b"1.000";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(5), c);
        }
        {
            let input = b"1.0000";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(5), c);
        }
        {
            let input = b"1.1";
            let mut c = Cursor(0);
            assert_eq!(
                Ok(QValue::from_millis(1000).unwrap()),
                q_value(input, &mut c)
            );
            assert_eq!(Cursor(2), c);
        }
    }
}
