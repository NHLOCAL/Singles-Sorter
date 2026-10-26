//! Pure fix-mode compatibility preview. All references bind the original object;
//! a future executor must stop after a failed rename instead of opening its target.
use crate::discovery::{ApplyBlocker, EntryObservation};
use crate::metadata::{
    ErrorStage, FieldError, LegacyField, LegacyScalar, MetadataError, MetadataProvider,
};
use crate::{RepairMode, clean, fix_jibrish, sanitize};
use serde::Serialize;

pub const FIX_FIELDS: [LegacyField; 5] = [
    LegacyField::Artist,
    LegacyField::AlbumArtist,
    LegacyField::Title,
    LegacyField::Album,
    LegacyField::Genre,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldRepair {
    pub field: LegacyField,
    pub original: String,
    pub replacement: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FixFailure {
    Open(MetadataError),
    Field {
        field: LegacyField,
        error: FieldError,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FixBlocker {
    Observation(ApplyBlocker),
    UnsafeIntendedName,
    EmptyIntendedName,
    CaseCollisionRequiresProviderResolution,
    /// Metadata intentions refer only to source identity. No fallback to the
    /// intended destination is permitted if rename/rebinding fails.
    RenameMustSucceedAndRebindOriginal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixFilePlan {
    pub source: EntryObservation,
    pub intended_name: Option<String>,
    pub rename_intended: bool,
    /// Successful assignments up to the first exception, in released field order.
    pub repairs: Vec<FieldRepair>,
    /// Includes unchanged tags: the release saves unconditionally after all fields.
    pub save_intended: bool,
    pub failure: Option<FixFailure>,
    pub blockers: Vec<FixBlocker>,
    pub executable: bool,
}

/// `occupied_sibling_names` represents the logical view at this file's turn,
/// including reservations from earlier predicted successful renames. Collection
/// and virtual mutations belong to the job planner, never this read-only policy.
pub fn fix_file(
    entry: &EntryObservation,
    occupied_sibling_names: &[String],
    metadata: &mut dyn MetadataProvider,
) -> FixFilePlan {
    let mut plan = FixFilePlan {
        source: entry.clone(),
        intended_name: sanitize(&clean(&entry.name)),
        rename_intended: false,
        repairs: vec![],
        save_intended: false,
        failure: None,
        blockers: entry
            .apply_blockers()
            .into_iter()
            .map(FixBlocker::Observation)
            .collect(),
        executable: false,
    };
    let Some(mut name) = plan.intended_name.take() else {
        plan.blockers.push(FixBlocker::EmptyIntendedName);
        return plan;
    };
    if name != entry.name {
        let (stem, suffix) = split_suffix(&name);
        let mut counter = 1usize;
        let mut candidate = name.clone();
        while occupied_sibling_names.contains(&candidate) {
            candidate = format!("{stem}_{counter}{suffix}");
            counter += 1;
        }
        name = candidate;
        plan.rename_intended = true;
        plan.blockers
            .push(FixBlocker::RenameMustSucceedAndRebindOriginal);
    }
    if unsafe_name(&name) {
        plan.blockers.push(FixBlocker::UnsafeIntendedName);
    }
    plan.intended_name = Some(name);
    // Never manufacture a reference from the proposed display name.
    let mut fields = match metadata.open(entry) {
        Ok(fields) => fields,
        Err(error) => {
            plan.failure = Some(FixFailure::Open(error));
            return plan;
        }
    };
    for field in FIX_FIELDS {
        let value = match fields.get(field).and_then(|item| item.value) {
            Ok(value) => value,
            Err(error) => {
                plan.failure = Some(FixFailure::Field { field, error });
                return plan;
            }
        };
        match value {
            LegacyScalar::Text(original) => {
                let replacement = fix_jibrish(&original, RepairMode::Hebrew);
                if replacement != original {
                    plan.repairs.push(FieldRepair {
                        field,
                        original,
                        replacement,
                    });
                }
            }
            LegacyScalar::Null => {}
            LegacyScalar::Integer(number) if number == "0" => {}
            LegacyScalar::Integer(_) => {
                // Python `value and check_jibrish(value)` attempts to iterate a
                // nonzero integer and raises; it does not stringify it.
                plan.failure = Some(FixFailure::Field {
                    field,
                    error: FieldError {
                        stage: ErrorStage::Value,
                        kind: "TypeError".into(),
                        detail: "'int' object is not iterable".into(),
                    },
                });
                return plan;
            }
        }
    }
    plan.save_intended = true;
    plan
}

// pathlib on the frozen Python 3.12 runtime treats leading-only dots and a final
// dot as part of the stem; only a non-leading, non-final dot starts a suffix.
fn split_suffix(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}
pub(crate) fn unsafe_name(name: &str) -> bool {
    let basename = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    let reserved = matches!(
        basename.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        basename.strip_prefix(prefix).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    name.is_empty()
        || matches!(name, "." | "..")
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c < '\u{20}' || "<>:\"/\\|?*".contains(c))
        || name.encode_utf16().count() > 255
        || reserved
}
