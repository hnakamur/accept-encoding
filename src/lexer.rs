use crate::Token;

pub(crate) struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    pub(crate) fn next_token(&mut self) -> Option<Token> {
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
