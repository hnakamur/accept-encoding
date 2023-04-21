local accept_encoding = require "accept_encoding"

local header_value = "br; q=0.9 , *"
local res1 = accept_encoding.match(header_value, "gzip")
print(string.format("res1 type=%s, q=%s", res1.match_type, res1.q))

local res2 = accept_encoding.match(header_value, "br")
print(string.format("res2 type=%s, q=%s", res2.match_type, res2.q))

local better = accept_encoding.is_better_match_than(res1, res2)
print(string.format("better=%s", better))

local res3 = accept_encoding.match(header_value, "deflate")
print(string.format("res3 type=%s, q=%s", res3.match_type, res3.q))

better = accept_encoding.is_better_match_than(res2, res3)
print(string.format("better=%s", better))
