use std::cmp::Ordering;

use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer::{comma, equal, ows, parameter_value, q_value, semicolon, slash, token, LexerToken},
    q_value::QValue,
};

pub fn match_for_mime_type(header_value: &[u8], mime_type: &[u8]) -> Option<MimeTypeMatch> {
    MimeTypeMatcher::new(header_value).match_mime_type(mime_type)
}

pub(crate) struct MimeTypeMatcher<'a> {
    value: &'a [u8],
    pos: usize,
    state: State,
    cur_result: Option<MimeTypeMatch>,
    best_result: Option<MimeTypeMatch>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum MimeTypeMatchType {
    MainTypeWildcard,
    SubTypeWildcard,
    Exact,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct MimeTypeMatch {
    pub match_type: MimeTypeMatchType,
    pub q: QValue,
}

impl Ord for MimeTypeMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.match_type, &self.q).cmp(&(other.match_type, &other.q))
    }
}

impl PartialOrd for MimeTypeMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
            value,
            pos: 0,
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
        while self.pos < self.value.len() {
            match self.state {
                State::SearchingMainType => {
                    if let Some(LexerToken::Token(token)) = token(self.value, &mut self.pos) {
                        cur_main_type = Some(token);
                        self.state = State::SeenMainType;
                    } else {
                        return None;
                    }
                }
                State::SeenMainType => {
                    if let Some(LexerToken::Slash) = slash(self.value, &mut self.pos) {
                        self.state = State::SeenSlash;
                    } else {
                        return None;
                    }
                }
                State::SeenSlash => {
                    if let Some(LexerToken::Token(subtype)) = token(self.value, &mut self.pos) {
                        let main_type = cur_main_type.unwrap();
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
                    ows(self.value, &mut self.pos);
                    if let Some(LexerToken::Semicolon) = semicolon(self.value, &mut self.pos) {
                        ows(self.value, &mut self.pos);
                        self.state = State::SeenSemicolon;
                    } else if let Some(LexerToken::Comma) = comma(self.value, &mut self.pos) {
                        ows(self.value, &mut self.pos);
                        self.may_update_best_result();
                        self.state = State::SearchingMainType;
                    } else {
                        return None;
                    }
                }
                State::SeenSemicolon => {
                    if let Some(LexerToken::Token(param_name)) = token(self.value, &mut self.pos) {
                        is_q_param = bytes_eq_ignore_case(param_name, b"q");
                        self.state = State::SeenParameterName;
                    } else {
                        return None;
                    }
                }
                State::SeenParameterName => {
                    if Some(LexerToken::Equal) == equal(self.value, &mut self.pos) {
                        self.state = State::SeenEqual;
                    } else {
                        return None;
                    }
                }
                State::SeenEqual => {
                    if is_q_param {
                        if let Some(LexerToken::QValue(q)) = q_value(self.value, &mut self.pos) {
                            if let Some(cur_result) = self.cur_result.as_mut() {
                                cur_result.q = q;
                            }
                        } else {
                            return None;
                        }
                    } else if parameter_value(self.value, &mut self.pos).is_none() {
                        return None;
                    }
                    self.state = State::SeenParameterValue;
                }
                State::SeenParameterValue => {
                    ows(self.value, &mut self.pos);
                    if let Some(LexerToken::Comma) = comma(self.value, &mut self.pos) {
                        ows(self.value, &mut self.pos);
                        self.may_update_best_result();
                        self.state = State::SearchingMainType;
                    } else if let Some(LexerToken::Semicolon) = semicolon(self.value, &mut self.pos)
                    {
                        ows(self.value, &mut self.pos);
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
    } else if bytes_eq_ignore_case(main_type, want_main_type) {
        if bytes_eq_ignore_case(subtype, want_subtype) {
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
    fn test_match_for_mime_type() {
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::MainTypeWildcard,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_mime_type(b"*/*", b"image/webp"),
        );

        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::SubTypeWildcard,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_mime_type(b"image/*", b"image/webp"),
        );

        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            match_for_mime_type(b"image/webp", b"image/webp"),
        );

        let chrome_accept_html = b"text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";

        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::MainTypeWildcard,
                q: QValue::try_from(0.8).unwrap(),
            }),
            match_for_mime_type(chrome_accept_html, b"image/png"),
        );

        let chrome_accept_img_tag =
            b"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8";
        let chrome_webp_match = match_for_mime_type(chrome_accept_img_tag, b"image/webp");
        let chrome_png_match = match_for_mime_type(chrome_accept_img_tag, b"image/png");
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            chrome_webp_match,
        );
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::SubTypeWildcard,
                q: QValue::try_from(1.0).unwrap(),
            }),
            chrome_png_match,
        );
        assert!(chrome_webp_match.gt(&chrome_png_match));

        let safari_accept_img_tag =
            b"image/webp,image/avif,video/*;q=0.8,image/png,image/svg+xml,image/*;q=0.8,*/*;q=0.5";
        let safari_webp_match = match_for_mime_type(safari_accept_img_tag, b"image/webp");
        let safari_png_match = match_for_mime_type(safari_accept_img_tag, b"image/png");
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            safari_webp_match,
        );
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            safari_png_match,
        );
        assert!(safari_webp_match.eq(&safari_png_match));

        let firefox_accept_img_tag = b"image/avif,image/webp,*/*";
        let firefox_webp_match = match_for_mime_type(firefox_accept_img_tag, b"image/webp");
        let firefox_png_match = match_for_mime_type(firefox_accept_img_tag, b"image/png");
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::Exact,
                q: QValue::try_from(1.0).unwrap(),
            }),
            firefox_webp_match,
        );
        assert_eq!(
            Some(MimeTypeMatch {
                match_type: MimeTypeMatchType::MainTypeWildcard,
                q: QValue::try_from(1.0).unwrap(),
            }),
            firefox_png_match,
        );
        assert!(firefox_webp_match.gt(&firefox_png_match));
    }
}
