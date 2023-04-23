local accept_encoding = require "accept_encoding"

local header_value = "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8"
local res1 = accept_encoding.match_mime_type(header_value, "image/webp")
print(string.format("res1 type=%s, q=%s", res1.match_type, res1.q))

local res2 = accept_encoding.match_mime_type(header_value, "image/png")
print(string.format("res2 type=%s, q=%s", res2.match_type, res2.q))

local better = accept_encoding.is_better_mime_type_match(res1, res2)
print(string.format("better=%s", better))
