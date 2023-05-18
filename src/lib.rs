pub use encoding_matcher::match_for_encoding;
pub use mime_type_matcher::match_for_mime_type;

mod byte_slice;
pub mod c;
mod encoding_matcher;
mod lexer;
mod mime_type_matcher;
mod q_value;
