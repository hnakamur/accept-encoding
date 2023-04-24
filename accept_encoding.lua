local ffi = require "ffi"
local S = ffi.load("accept_encoding")

ffi.cdef[[
    typedef struct {
        int32_t match_type;
        double q;
    } CEncodingMatch;

    CEncodingMatch c_match_encoding(
        const char *header_value, size_t header_value_len,
        const char *encoding, size_t encoding_len);
    
    int c_cmp_encoding_match(CEncodingMatch res1, CEncodingMatch res2);

    typedef struct {
        int32_t match_type;
        double q;
    } CMimeTypeMatch;

    CMimeTypeMatch c_match_mime_type(
        const char *header_value, size_t header_value_len,
        const char *mime_type, size_t mime_type_len);
    
    int c_cmp_mime_type_match(CMimeTypeMatch res1, CMimeTypeMatch res2);
]]

local function match_encoding(header_value, encoding)
    return S.c_match_encoding(header_value, #header_value, encoding, #encoding)
end

local function cmp_encoding_match(res1, res2)
    return S.c_cmp_encoding_match(res1, res2)
end

local function match_mime_type(header_value, mime_type)
    return S.c_match_mime_type(header_value, #header_value, mime_type, #mime_type)
end

local function cmp_mime_type_match(res1, res2)
    return S.c_cmp_mime_type_match(res1, res2)
end

return {
    match_encoding = match_encoding,
    cmp_encoding_match = cmp_encoding_match,
    match_mime_type = match_mime_type,
    cmp_mime_type_match = cmp_mime_type_match,
}
