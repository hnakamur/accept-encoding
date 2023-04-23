local ffi = require "ffi"
local S = ffi.load("accept_encoding")

ffi.cdef[[
    typedef struct {
        int32_t match_type;
        double q;
    } AeEncodingMatch;

    AeEncodingMatch ae_match(
        const char *header_value, size_t header_value_len,
        const char *encoding, size_t encoding_len);
    
    int ae_is_better_match_than(AeEncodingMatch res1, AeEncodingMatch res2);
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
