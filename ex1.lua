local accept_encoding = require "accept_encoding"

local header_value = "br; q=0.9 , *"
local m1 = accept_encoding.match_encoding(header_value, "gzip")
print(string.format("m1 type=%s, q=%s", m1.match_type, m1.q))

local m2 = accept_encoding.match_encoding(header_value, "br")
print(string.format("m2 type=%s, q=%s", m2.match_type, m2.q))

local cmp = accept_encoding.cmp_encoding_match(m1, m2)
print(string.format("cmp=%s", cmp))

local res3 = accept_encoding.match_encoding(header_value, "deflate")
print(string.format("res3 type=%s, q=%s", res3.match_type, res3.q))

cmp = accept_encoding.cmp_encoding_match(m2, res3)
print(string.format("cmp=%s", cmp))
