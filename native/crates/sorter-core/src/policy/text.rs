//! Text-only release rules. No normalization, path validation or tag writes.
use crate::{RepairMode, unicode15};

const SPAM: &[&str] = &[
    "מייל מיוזיק",
    "ציצו במייל",
    "חדשות המוזיקה",
    "חדשות המוזיקה",
    "ציצו",
    "מוזיקה מכל הלב",
    "מייל מיוזיק",
    "המחדש",
];
const UNUSUAL: &[&str] = &[
    "סינגלים",
    "סינגל",
    "אבגדהוזחטיכלמנסעפצקרשתךםןץ",
    "אמן לא ידוע",
    "טוב",
    "לא ידוע",
    "תודה לך ה",
];
const HEBREW: &str = "אבגדהוזחטיכלמנסעפצקרשתךםןףץ";
const JIBRISH: &str = "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ";

pub fn clean(text: &str) -> String {
    let underscored = if text.contains('_') {
        if text.contains(' ') {
            replace_between_words(text, '_', "")
        } else {
            text.replace('_', " ")
        }
    } else {
        text.into()
    };
    let mut cleaned = replace_between_words(&underscored, '-', " ");
    for spam in SPAM {
        cleaned = cleaned.replace(spam, "");
    }
    collapse_space(&cleaned)
}

fn replace_between_words(text: &str, separator: char, replacement: &str) -> String {
    let characters: Vec<_> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    for (index, &character) in characters.iter().enumerate() {
        if character == separator
            && index > 0
            && unicode15::is_word(characters[index - 1])
            && characters
                .get(index + 1)
                .copied()
                .is_some_and(unicode15::is_word)
        {
            output.push_str(replacement);
        } else {
            output.push(character);
        }
    }
    output
}

fn collapse_space(text: &str) -> String {
    text.split(unicode15::is_space)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The release's sanitation only; safe destination checks belong to planning.
pub fn sanitize(text: &str) -> Option<String> {
    let stripped: String = text
        .chars()
        .filter(|character| !"<>:\"/\\|?*".contains(*character))
        .collect();
    let sanitized: String = collapse_space(&stripped).chars().take(255).collect();
    (!sanitized.is_empty()).then_some(sanitized)
}

pub fn check_artist(text: &str) -> bool {
    if text.is_empty() || UNUSUAL.contains(&text) {
        return false;
    }
    let word_count = text
        .split(unicode15::is_space)
        .filter(|word| !word.is_empty())
        .count();
    (1..=3).contains(&word_count)
        && text
            .chars()
            .all(|character| HEBREW.contains(character) || "'׳- ".contains(character))
}

/// Convert the 27 pinned letters only. Auto chooses at the first convertible character.
pub fn fix_jibrish(text: &str, mode: RepairMode) -> String {
    let hebrew: Vec<_> = HEBREW.chars().collect();
    let jibrish: Vec<_> = JIBRISH.chars().collect();
    let mode = if mode == RepairMode::Auto {
        text.chars()
            .find_map(|character| {
                if HEBREW.contains(character) {
                    Some(RepairMode::Jibrish)
                } else if JIBRISH.contains(character) {
                    Some(RepairMode::Hebrew)
                } else {
                    None
                }
            })
            .unwrap_or(RepairMode::Auto)
    } else {
        mode
    };
    let (from, to) = match mode {
        RepairMode::Hebrew => (&jibrish, &hebrew),
        RepairMode::Jibrish => (&hebrew, &jibrish),
        RepairMode::Auto => return text.into(),
    };
    text.chars()
        .map(|character| {
            from.iter()
                .position(|&candidate| candidate == character)
                .map_or(character, |index| to[index])
        })
        .collect()
}
