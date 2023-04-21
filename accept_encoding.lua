local ffi = require "ffi"
local S = ffi.load("accept_encoding_normalizer")

ffi.cdef[[
    typedef struct {
        int32_t match_type;
        float q;
    } EncodingMatchResult;

    EncodingMatchResult ae_match(
        const char *header_value, size_t header_value_len,
        const char *encoding, size_t encoding_len);
    
    int ae_is_better_match_than(EncodingMatchResult res1, EncodingMatchResult res2);
]]

local function ae_match(header_value, encoding)
    return S.ae_match(header_value, #header_value, encoding, #encoding)
end

local function ae_is_better_match_than(res1, res2)
    return S.ae_is_better_match_than(res1, res2)
end

return {
    match = ae_match,
    is_better_match_than = ae_is_better_match_than,
}
