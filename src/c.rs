use std::{
    ffi::{c_char, c_int},
    slice,
};

use crate::{match_for_encoding, EncodingMatchType};

pub const AE_ENCODING_MATCH_TYPE_NO_MATCH: i32 = 0;
pub const AE_ENCODING_MATCH_TYPE_WILDCARD: i32 = 1;
pub const AE_ENCODING_MATCH_TYPE_EXACT: i32 = 2;

#[repr(C)]
pub struct AeEncodingMatch {
    match_type: i32,
    q: f64,
}

#[no_mangle]
pub extern "C" fn ae_match(
    header_value: *const c_char,
    header_value_len: usize,
    encoding: *const c_char,
    encoding_len: usize,
) -> AeEncodingMatch {
    let header_value =
        unsafe { slice::from_raw_parts(header_value as *const u8, header_value_len) };
    let encoding = unsafe { slice::from_raw_parts(encoding as *const u8, encoding_len) };
    match match_for_encoding(header_value, encoding) {
        Some(r) => AeEncodingMatch {
            match_type: match r.match_type {
                EncodingMatchType::Wildcard => AE_ENCODING_MATCH_TYPE_WILDCARD,
                EncodingMatchType::Exact => AE_ENCODING_MATCH_TYPE_EXACT,
            },
            q: r.q.into(),
        },
        None => AeEncodingMatch {
            match_type: AE_ENCODING_MATCH_TYPE_NO_MATCH,
            q: 0.0,
        },
    }
}

#[no_mangle]
pub extern "C" fn ae_is_better_match_than(res1: AeEncodingMatch, res2: AeEncodingMatch) -> c_int {
    if res1.match_type > res2.match_type
        || (res1.match_type == res2.match_type
            && res1.match_type != AE_ENCODING_MATCH_TYPE_NO_MATCH
            && res1.q > res2.q)
    {
        1
    } else {
        0
    }
}
