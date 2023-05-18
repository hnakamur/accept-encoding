use std::{cmp::Ordering, str};

use crate::{
    byte_slice::bytes_eq_ignore_case,
    lexer::{self, Cursor},
    q_value::QValue,
};

pub fn match_for_mime_type(input: &[u8], mime_type: &[u8]) -> Option<MimeTypeMatch> {
    let (want_main_type, want_subtype) = match split_mime_type(mime_type) {
        Some((main_type, subtype)) => (main_type, subtype),
        None => return None,
    };

    let mut c = Cursor(0);
    let mut state = State::SearchingMainType;
    let mut cur_result: Option<MimeTypeMatch> = None;
    let mut best_result: Option<MimeTypeMatch> = None;

    let mut cur_main_type = None;
    let mut is_q_param = false;
    while !c.eof(input) {
        match state {
            State::SearchingMainType => {
                let c1 = c;
                if lexer::token(input, &mut c).is_ok() {
                    let token = c1.slice(input, c);
                    cur_main_type = Some(token);
                    state = State::SeenMainType;
                } else {
                    return None;
                }
            }
            State::SeenMainType => {
                if lexer::byte(b'/')(input, &mut c).is_ok() {
                    state = State::SeenSlash;
                } else {
                    return None;
                }
            }
            State::SeenSlash => {
                let c1 = c;
                if lexer::token(input, &mut c).is_ok() {
                    let subtype = c1.slice(input, c);
                    let main_type = cur_main_type.unwrap();
                    if let Some(match_type) =
                        get_mime_type_match_type(main_type, subtype, want_main_type, want_subtype)
                    {
                        cur_result = Some(MimeTypeMatch {
                            match_type,
                            q: QValue::from_millis(1000).unwrap(),
                        })
                    }
                    state = State::SeenSubType;
                } else {
                    return None;
                }
            }
            State::SeenSubType => {
                lexer::ows(input, &mut c);
                if lexer::byte(b';')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    state = State::SeenSemicolon;
                } else if lexer::byte(b',')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingMainType;
                } else {
                    return None;
                }
            }
            State::SeenSemicolon => {
                let c1 = c;
                lexer::token(input, &mut c).ok()?;
                let param_name = c1.slice(input, c);
                is_q_param = bytes_eq_ignore_case(param_name, b"q");
                state = State::SeenParameterName;
            }
            State::SeenParameterName => {
                lexer::byte(b'=')(input, &mut c).ok()?;
                state = State::SeenEqual;
            }
            State::SeenEqual => {
                if is_q_param {
                    let c1 = c;
                    lexer::q_value(input, &mut c).ok()?;
                    if let Some(cur_result) = cur_result.as_mut() {
                        cur_result.q =
                            QValue::try_from(str::from_utf8(c1.slice(input, c)).unwrap()).unwrap();
                    }
                } else {
                    lexer::alt(lexer::token, lexer::quoted_string)(input, &mut c).ok()?;
                }
                state = State::SeenParameterValue;
            }
            State::SeenParameterValue => {
                lexer::ows(input, &mut c);
                if lexer::byte(b',')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    may_update_best_result(&mut cur_result, &mut best_result);
                    state = State::SearchingMainType;
                } else if lexer::byte(b';')(input, &mut c).is_ok() {
                    lexer::ows(input, &mut c);
                    state = State::SeenSemicolon;
                } else {
                    return None;
                }
            }
        }
    }
    may_update_best_result(&mut cur_result, &mut best_result);
    best_result.take()
}

fn may_update_best_result(
    cur_result: &mut Option<MimeTypeMatch>,
    best_result: &mut Option<MimeTypeMatch>,
) {
    if cur_result.gt(&best_result) {
        *best_result = cur_result.take();
    }
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
