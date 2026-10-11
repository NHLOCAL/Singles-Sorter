//! Pure release-compatible album policy over ordered observations. No filesystem access.
use crate::discovery::{EntryKind, EntryObservation, StorageRef};
use crate::metadata::{
    FieldError, LegacyField, LegacyScalar, MetadataError, MetadataFields, MetadataProvider,
};
use crate::unicode15::decimal_digit;
use crate::{RepairMode, fix_jibrish};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderObservation {
    pub entry: EntryObservation,
    pub direct_children: Vec<EntryObservation>,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AlbumTraceEvent {
    Open {
        source: StorageRef,
        result: Result<(), MetadataError>,
    },
    Get {
        source: StorageRef,
        field: LegacyField,
        result: Result<(), FieldError>,
    },
    Truthy {
        source: StorageRef,
        field: LegacyField,
        result: bool,
    },
    Value {
        source: StorageRef,
        field: LegacyField,
        values: Vec<LegacyScalar>,
        result: Result<LegacyScalar, FieldError>,
    },
    FileError {
        source: StorageRef,
        kind: String,
    },
    ProtectedError {
        kind: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AlbumDecision {
    pub is_album: bool,
    pub should_process: bool,
    pub album_name: Option<String>,
    pub artist_name: Option<String>,
    pub trace: Vec<AlbumTraceEvent>,
}
impl AlbumDecision {
    fn singles() -> Self {
        Self {
            is_album: false,
            should_process: false,
            album_name: None,
            artist_name: None,
            trace: vec![],
        }
    }
}
/// Like Python 3.12 Path.suffix: dotfiles without a further dot have no suffix.
pub fn is_audio(name: &str) -> bool {
    let Some((stem, suffix)) = name.rsplit_once('.') else {
        return false;
    };
    !stem.is_empty()
        && matches!(
            suffix.to_lowercase().as_str(),
            "m4a"
                | "wma"
                | "wav"
                | "aiff"
                | "flac"
                | "aac"
                | "alac"
                | "wv"
                | "ogg"
                | "dsf"
                | "opus"
                | "mp3"
        )
}
fn read_field(
    fields: &mut dyn MetadataFields,
    source: &StorageRef,
    field: LegacyField,
    default: LegacyScalar,
    trace: &mut Vec<AlbumTraceEvent>,
) -> Result<LegacyScalar, String> {
    let first = fields.get(field);
    trace.push(AlbumTraceEvent::Get {
        source: source.clone(),
        field,
        result: first.as_ref().map(|_| ()).map_err(Clone::clone),
    });
    let first = first.map_err(|e| e.kind)?;
    trace.push(AlbumTraceEvent::Truthy {
        source: source.clone(),
        field,
        result: first.truthy,
    });
    if !first.truthy {
        return Ok(default);
    }
    // The release calls get twice; stateful providers must observe both calls.
    let second = fields.get(field);
    trace.push(AlbumTraceEvent::Get {
        source: source.clone(),
        field,
        result: second.as_ref().map(|_| ()).map_err(Clone::clone),
    });
    let second = second.map_err(|e| e.kind)?;
    trace.push(AlbumTraceEvent::Value {
        source: source.clone(),
        field,
        values: second.values,
        result: second.value.clone(),
    });
    second.value.map_err(|e| e.kind)
}
fn text(value: LegacyScalar) -> Result<String, String> {
    match value {
        LegacyScalar::Text(s) => Ok(s),
        _ => Err("AttributeError".into()),
    }
}
fn strip(s: &str) -> &str {
    s.trim_matches(crate::unicode15::is_space)
}
fn increment(counts: &mut Vec<(String, usize)>, value: String) {
    if let Some((_, count)) = counts.iter_mut().find(|(name, _)| *name == value) {
        *count += 1;
    } else {
        counts.push((value, 1));
    }
}
const EMPTY: &str = "___EMPTY___";
pub fn analyze_album(
    folder: &FolderObservation,
    provider: &mut dyn MetadataProvider,
) -> AlbumDecision {
    analyze_album_controlled(folder, provider, &crate::job_plan::UncontrolledPlanning)
        .expect("uncontrolled album analysis cannot abort")
}
/// Same released policy and read order, with job-control checks between files.
pub fn analyze_album_controlled(
    folder: &FolderObservation,
    provider: &mut dyn MetadataProvider,
    control: &dyn crate::job_plan::PlanningControl,
) -> Result<AlbumDecision, crate::job_plan::PlanningAbort> {
    control.checkpoint(crate::job_plan::PlanningBoundary::Visit)?;
    let mut decision = AlbumDecision::singles();
    if folder.entry.kind != EntryKind::Directory {
        return Ok(decision);
    }
    // Intentionally suffix-only, including directories with audio suffixes.
    let audio: Vec<_> = folder
        .direct_children
        .iter()
        .filter(|e| is_audio(&e.name))
        .collect();
    let total = audio.len();
    if total < 3
        || folder
            .direct_children
            .iter()
            .filter(|e| e.kind == EntryKind::Directory)
            .count()
            > 1
    {
        return Ok(decision);
    }
    let mut albums = Vec::new();
    let mut artists = Vec::new();
    let mut tracks = Vec::new();
    let mut filenames = Vec::new();
    let mut empty = 0;
    for entry in audio {
        control.checkpoint(crate::job_plan::PlanningBoundary::File)?;
        let result: Result<(), String> = (|| {
            let opened = provider.open(entry);
            decision.trace.push(AlbumTraceEvent::Open {
                source: entry.reference.clone(),
                result: opened.as_ref().map(|_| ()).map_err(Clone::clone),
            });
            let mut fields = opened.map_err(|e| match e {
                MetadataError::LegacyOpen { kind, .. } => kind,
                other => format!("{other:?}"),
            })?;
            let mut read = |field, default| {
                read_field(
                    fields.as_mut(),
                    &entry.reference,
                    field,
                    default,
                    &mut decision.trace,
                )
            };
            let mut album = text(read(LegacyField::Album, LegacyScalar::Text(String::new()))?)?;
            if strip(&album).is_empty() {
                album = EMPTY.into();
            }
            // Keep this accumulation even if a later field fails.
            increment(&mut albums, album.clone());
            let artist = read(LegacyField::Artist, LegacyScalar::Text(String::new()))?;
            let album_artist = text(read(
                LegacyField::AlbumArtist,
                LegacyScalar::Text(String::new()),
            )?)?;
            let chosen = if !strip(&album_artist).is_empty() {
                strip(&album_artist).to_owned()
            } else {
                strip(&text(artist)?).to_owned()
            };
            let chosen = fix_jibrish(&chosen, RepairMode::Hebrew);
            let track = read(LegacyField::TrackNumber, LegacyScalar::Null)?;
            let track = match track {
                LegacyScalar::Text(s) => parse_decimal(s.split('/').next().unwrap_or("")),
                LegacyScalar::Integer(s) => parse_decimal(&s).filter(|s| s != "0"),
                LegacyScalar::Null => None,
            };
            let has_track = track.is_some();
            if let Some(track) = track {
                tracks.push(track);
            }
            let prefix: String = entry
                .name
                .chars()
                .take_while(|c| decimal_digit(*c).is_some())
                .collect();
            if !prefix.is_empty() {
                filenames.push(parse_decimal(&prefix).ok_or_else(|| "ValueError".to_string())?);
            }
            if !chosen.is_empty() {
                increment(&mut artists, chosen.clone());
            }
            if strip(&chosen).is_empty() && album == EMPTY && !has_track && prefix.is_empty() {
                empty += 1;
            }
            Ok(())
        })();
        control.checkpoint(crate::job_plan::PlanningBoundary::File)?;
        if let Err(kind) = result {
            decision.trace.push(AlbumTraceEvent::FileError {
                source: entry.reference.clone(),
                kind,
            });
        }
    }
    if empty == total {
        return Ok(decision);
    }
    if albums.iter().chain(artists.iter()).any(|(s, _)| {
        s != EMPTY
            && ["סינגל", "single"]
                .iter()
                .any(|k| s.to_lowercase().contains(k))
    }) {
        return Ok(decision);
    }
    // Counter.most_common retains the first encountered name on a tie.
    let Some((album, count)) = albums
        .iter()
        .enumerate()
        .max_by_key(|(index, (_, count))| (*count, std::cmp::Reverse(*index)))
        .map(|(_, v)| v)
    else {
        decision.is_album = true;
        decision.trace.push(AlbumTraceEvent::ProtectedError {
            kind: "IndexError".into(),
        });
        return Ok(decision);
    };
    decision.is_album = (album != EMPTY && *count as f64 / total as f64 >= 0.7)
        || consistent(&tracks, total)
        || consistent(&filenames, total);
    if !decision.is_album {
        return Ok(decision);
    }
    let artist_total: usize = artists.iter().map(|(_, n)| n).sum();
    decision.artist_name = artists
        .iter()
        .find(|(_, n)| *n as f64 / artist_total as f64 >= 0.7)
        .map(|(s, _)| s.clone());
    decision.should_process = decision.artist_name.is_some();
    decision.album_name = Some(fix_jibrish(
        if album == EMPTY {
            &folder.entry.name
        } else {
            album
        },
        RepairMode::Hebrew,
    ));
    Ok(decision)
}
// Arbitrary-width signed numbers retained as canonical decimal strings. No machine-int
// truncation; only range comparisons against 1 and list length are needed by policy.
fn parse_decimal(s: &str) -> Option<String> {
    // Python int accepts Unicode whitespace except ASCII information separators.
    let s = s.trim_matches(|c| crate::unicode15::is_space(c) && !matches!(c, '\u{1c}'..='\u{1f}'));
    let (negative, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let mut digits = String::new();
    let mut previous_digit = false;
    for c in s.chars() {
        if c == '_' {
            if !previous_digit {
                return None;
            }
            previous_digit = false;
        } else {
            digits.push(char::from(b'0' + decimal_digit(c)?));
            previous_digit = true;
        }
    }
    if !previous_digit || digits.len() > 4300 {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        Some("0".into())
    } else {
        Some(format!("{}{digits}", if negative { "-" } else { "" }))
    }
}
fn consistent(nums: &[String], total: usize) -> bool {
    let n = nums.len();
    let limit = n.to_string();
    n > 0
        && n as f64 >= 0.7 * total as f64
        && nums.iter().any(|s| s == "1")
        && nums.iter().any(|s| s == &limit)
        && nums.iter().all(|s| {
            !s.starts_with('-')
                && s != "0"
                && (s.len() < limit.len() || (s.len() == limit.len() && s <= &limit))
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_grammar_and_digit_limit_are_python_312_compatible() {
        assert_eq!(
            parse_decimal(&format!("{}3", "0".repeat(4299))),
            Some("3".into())
        );
        assert_eq!(parse_decimal(&format!("{}3", "0".repeat(4300))), None);
        assert_eq!(parse_decimal(" -٠٠ "), Some("0".into()));
        assert_eq!(parse_decimal(" +١_٢\u{a0}"), Some("12".into()));
        for invalid in ["", "_1", "1_", "1__2", "1.0", "\u{1c}1", "¹", "\u{10d41}"] {
            assert_eq!(parse_decimal(invalid), None, "{invalid:?}");
        }
    }
    #[test]
    fn numbering_preserves_release_duplicate_quirk() {
        assert!(consistent(&["1".into(), "1".into(), "3".into()], 3));
        assert!(!consistent(&["1".into(), "1".into(), "3".into()], 5));
        assert!(!consistent(
            &[
                "1".into(),
                "2".into(),
                "9999999999999999999999999999".into()
            ],
            3
        ));
    }
}
