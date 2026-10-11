pub mod matching;
pub mod singles;
pub mod text;

pub use matching::exact_match;
pub use singles::classify;
pub use text::{check_artist, clean, fix_jibrish, sanitize};
