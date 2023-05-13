use std::hint::black_box;

use accept_encoding::encoding_matcher2::match_for_encoding;

fn main() {
    for _ in 0..10_000_000 {
        black_box(match_for_encoding(b"gzip, deflate, br", b"br"));
    }
}
