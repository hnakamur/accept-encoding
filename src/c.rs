use std::{
    ffi::{c_char, c_int},
    slice,
};

use crate::{
    encoding_matcher::{match_for_encoding, EncodingMatchType},
    mime_type_matcher::{match_for_mime_type, MimeTypeMatchType},
};

pub const C_ENCODING_MATCH_TYPE_NO_MATCH: i32 = 0;
pub const C_ENCODING_MATCH_TYPE_WILDCARD: i32 = 1;
pub const C_ENCODING_MATCH_TYPE_EXACT: i32 = 2;

#[repr(C)]
pub struct CEncodingMatch {
    match_type: i32,
    q: f64,
}

#[no_mangle]
pub extern "C" fn c_match_encoding(
    header_value: *const c_char,
    header_value_len: usize,
    encoding: *const c_char,
    encoding_len: usize,
) -> CEncodingMatch {
    let header_value =
        unsafe { slice::from_raw_parts(header_value as *const u8, header_value_len) };
    let encoding = unsafe { slice::from_raw_parts(encoding as *const u8, encoding_len) };
    match match_for_encoding(header_value, encoding) {
        Some(r) => CEncodingMatch {
            match_type: match r.match_type {
                EncodingMatchType::Wildcard => C_ENCODING_MATCH_TYPE_WILDCARD,
                EncodingMatchType::Exact => C_ENCODING_MATCH_TYPE_EXACT,
            },
            q: r.q.into(),
        },
        None => CEncodingMatch {
            match_type: C_ENCODING_MATCH_TYPE_NO_MATCH,
            q: 0.0,
        },
    }
}

#[no_mangle]
pub extern "C" fn c_cmp_encoding_match(m1: CEncodingMatch, m2: CEncodingMatch) -> c_int {
    if m1.match_type < m2.match_type {
        -1
    } else if m1.match_type > m2.match_type {
        1
    } else if m1.match_type != C_ENCODING_MATCH_TYPE_NO_MATCH {
        if m1.q < m2.q {
            -1
        } else if m1.q > m2.q {
            1
        } else {
            0
        }
    } else {
        0
    }
}

pub const C_MIME_TYPE_MATCH_TYPE_NO_MATCH: i32 = 0;
pub const C_MIME_TYPE_MATCH_TYPE_MAIN_TYPE_WILDCARD: i32 = 1;
pub const C_MIME_TYPE_MATCH_TYPE_SUB_TYPE_WILDCARD: i32 = 2;
pub const C_MIME_TYPE_MATCH_TYPE_EXACT: i32 = 3;

#[repr(C)]
pub struct CMimeTypeMatch {
    match_type: i32,
    q: f64,
}

#[no_mangle]
pub extern "C" fn c_match_mime_type(
    header_value: *const c_char,
    header_value_len: usize,
    mime_type: *const c_char,
    mime_type_len: usize,
) -> CMimeTypeMatch {
    let header_value =
        unsafe { slice::from_raw_parts(header_value as *const u8, header_value_len) };
    let mime_type = unsafe { slice::from_raw_parts(mime_type as *const u8, mime_type_len) };
    match match_for_mime_type(header_value, mime_type) {
        Some(r) => CMimeTypeMatch {
            match_type: match r.match_type {
                MimeTypeMatchType::MainTypeWildcard => C_MIME_TYPE_MATCH_TYPE_MAIN_TYPE_WILDCARD,
                MimeTypeMatchType::SubTypeWildcard => C_MIME_TYPE_MATCH_TYPE_SUB_TYPE_WILDCARD,
                MimeTypeMatchType::Exact => C_MIME_TYPE_MATCH_TYPE_EXACT,
            },
            q: r.q.into(),
        },
        None => CMimeTypeMatch {
            match_type: C_MIME_TYPE_MATCH_TYPE_NO_MATCH,
            q: 0.0,
        },
    }
}

#[no_mangle]
pub extern "C" fn c_cmp_mime_type_match(m1: CMimeTypeMatch, m2: CMimeTypeMatch) -> c_int {
    if m1.match_type < m2.match_type {
        -1
    } else if m1.match_type > m2.match_type {
        1
    } else if m1.match_type != C_MIME_TYPE_MATCH_TYPE_NO_MATCH {
        if m1.q < m2.q {
            -1
        } else if m1.q > m2.q {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    #[test]
    fn test_c_match_encoding() {
        {
            let header_value = CString::new("br, gzip").unwrap();
            let encoding = CString::new("br").unwrap();
            let m = c_match_encoding(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_ENCODING_MATCH_TYPE_EXACT, m.match_type);
            assert_eq!(1.0, m.q);
        }
        {
            let header_value = CString::new("*").unwrap();
            let encoding = CString::new("br").unwrap();
            let m = c_match_encoding(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_ENCODING_MATCH_TYPE_WILDCARD, m.match_type);
            assert_eq!(1.0, m.q);
        }
        {
            let header_value = CString::new("gzip").unwrap();
            let encoding = CString::new("br").unwrap();
            let m = c_match_encoding(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_ENCODING_MATCH_TYPE_NO_MATCH, m.match_type);
            assert_eq!(0.0, m.q);
        }
    }

    #[test]
    fn test_c_cmp_encoding_match() {
        assert_eq!(
            -1,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_WILDCARD,
                    q: 0.0,
                }
            )
        );

        assert_eq!(
            1,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.0,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                }
            )
        );

        assert_eq!(
            -1,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.0,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.1,
                }
            )
        );

        assert_eq!(
            0,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.8,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.8,
                }
            )
        );

        assert_eq!(
            1,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 1.0,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_EXACT,
                    q: 0.9,
                }
            )
        );

        assert_eq!(
            0,
            c_cmp_encoding_match(
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                },
                CEncodingMatch {
                    match_type: C_ENCODING_MATCH_TYPE_NO_MATCH,
                    q: 1.0,
                }
            )
        );
    }

    #[test]
    fn test_c_match_mime_type() {
        {
            let header_value = CString::new("image/webp").unwrap();
            let encoding = CString::new("image/webp").unwrap();
            let m = c_match_mime_type(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_MIME_TYPE_MATCH_TYPE_EXACT, m.match_type);
            assert_eq!(1.0, m.q);
        }
        {
            let header_value = CString::new("image/*").unwrap();
            let encoding = CString::new("image/webp").unwrap();
            let m = c_match_mime_type(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_MIME_TYPE_MATCH_TYPE_SUB_TYPE_WILDCARD, m.match_type);
            assert_eq!(1.0, m.q);
        }
        {
            let header_value = CString::new("*/*").unwrap();
            let encoding = CString::new("image/webp").unwrap();
            let m = c_match_mime_type(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_MIME_TYPE_MATCH_TYPE_MAIN_TYPE_WILDCARD, m.match_type);
            assert_eq!(1.0, m.q);
        }
        {
            let header_value = CString::new("image/png").unwrap();
            let encoding = CString::new("image/webp").unwrap();
            let m = c_match_mime_type(
                header_value.as_ptr(),
                header_value.as_bytes().len(),
                encoding.as_ptr(),
                encoding.as_bytes().len(),
            );
            assert_eq!(C_ENCODING_MATCH_TYPE_NO_MATCH, m.match_type);
            assert_eq!(0.0, m.q);
        }
    }
    #[test]
    fn test_c_cmp_mime_type_match() {
        assert_eq!(
            -1,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_MAIN_TYPE_WILDCARD,
                    q: 0.0,
                }
            )
        );

        assert_eq!(
            1,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.0,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                }
            )
        );

        assert_eq!(
            -1,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.0,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.1,
                }
            )
        );

        assert_eq!(
            0,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.8,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.8,
                }
            )
        );

        assert_eq!(
            1,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 1.0,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_EXACT,
                    q: 0.9,
                }
            )
        );

        assert_eq!(
            0,
            c_cmp_mime_type_match(
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_NO_MATCH,
                    q: 0.0,
                },
                CMimeTypeMatch {
                    match_type: C_MIME_TYPE_MATCH_TYPE_NO_MATCH,
                    q: 1.0,
                }
            )
        );
    }
}
