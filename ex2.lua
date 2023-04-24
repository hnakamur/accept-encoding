local accept_encoding = require "accept_encoding"

local header_value = "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8"
local m1 = accept_encoding.match_mime_type(header_value, "image/webp")
print(string.format("m1 type=%s, q=%s", m1.match_type, m1.q))

local m2 = accept_encoding.match_mime_type(header_value, "image/png")
print(string.format("m2 type=%s, q=%s", m2.match_type, m2.q))

local cmp = accept_encoding.cmp_mime_type_match(m1, m2)
print(string.format("cmp=%s", cmp))
