//! Released regex semantics without a dependency on a newer Unicode database.
use crate::{Alias, unicode15};

/// Python 3.12 `(^|[^א-ת])ו?<escaped artist>\b` after Python lstrip.
/// Matching is case-sensitive and deliberately does not normalize Unicode.
pub fn exact_match(text: &str, artist: &str) -> bool {
    let text = text.trim_start_matches(unicode15::is_space);
    if !text.contains(artist) {
        return false;
    }
    // Include the final position for an empty alias, and do not skip overlapping
    // occurrences. A trailing regex boundary can follow a non-word alias too.
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .any(|start| {
            if !text[start..].starts_with(artist) {
                return false;
            }
            let end = start + artist.len();
            let left_word = text[..end]
                .chars()
                .next_back()
                .is_some_and(unicode15::is_word);
            let right_word = text[end..].chars().next().is_some_and(unicode15::is_word);
            if left_word == right_word {
                return false;
            }
            let preceding = text[..start].chars().next_back();
            let plain_prefix = preceding.is_none_or(|character| !is_hebrew(character));
            let vav_prefix = preceding == Some('ו')
                && text[..start - 'ו'.len_utf8()]
                    .chars()
                    .next_back()
                    .is_none_or(|character| !is_hebrew(character));
            plain_prefix || vav_prefix
        })
}

fn is_hebrew(character: char) -> bool {
    ('א'..='ת').contains(&character)
}

pub(crate) fn first_alias<'a>(text: &str, aliases: &'a [Alias]) -> Option<(usize, &'a str)> {
    aliases.iter().enumerate().find_map(|(index, alias)| {
        exact_match(text, &alias.source).then_some((index, alias.target.as_str()))
    })
}
