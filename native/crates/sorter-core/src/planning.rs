//! Pure logical plans from supplied single-song facts. No storage/model adapter is accepted.
//!
//! Logical names and snapshot observations use exact supplied string equality.
//! No Windows case, reserved-name, identity, symlink or Android provider semantics
//! are inferred. Physical resolution and every apply operation remain unsupported.

use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::*;

/// Generate a stable, explicitly non-executable supplied-facts plan.
///
/// The interface cannot receive a file-opening capability or traversal adapter:
///
/// ```compile_fail,E0061
/// use sorter_core::{plan, PlanRequest};
/// fn with_file_opener(request: &PlanRequest, open: fn(&str) -> Vec<u8>) {
///     let _ = plan(request, open);
/// }
/// ```
pub fn plan(request: &PlanRequest) -> Result<Plan, EngineError> {
    validate_request(request)?;
    let request_hash = fingerprint(request)?;
    let settings = &request.context.settings;
    let mut occupied: BTreeSet<Vec<String>> = request
        .target
        .existing_files
        .iter()
        .chain(&request.target.existing_directories)
        .cloned()
        .collect();
    let mut items = Vec::with_capacity(request.songs.len());

    for (song_index, song) in request.songs.iter().enumerate() {
        let mut decision = classify(&ClassifyRequest {
            schema_version: request.schema_version,
            context: request.context.clone(),
            song: song.clone(),
        })?;
        let operation_id = fingerprint(&(
            SCHEMA_VERSION,
            "song",
            &request_hash,
            song_index,
            &song.source_id,
        ))?;
        let mut destinations = vec![];
        if let Some(artists) = &decision.artists {
            let artists = artists.clone();
            let intended = if settings.duet_mode {
                artists.as_slice()
            } else {
                &artists[..artists.len().min(1)]
            };
            let filename = sanitize(&song.filename).ok_or_else(|| EngineError {
                code: ErrorCode::InvalidInput,
                message: format!(
                    "Original filename for source {:?} sanitizes to no output name",
                    song.source_id
                ),
            })?;
            // Recognition blockers are path-dependent and come from classify.
            // A blocker for one unsafe duet destination must not rewrite other
            // valid raw intents or pretend that their recognition failed.
            let recognition_blocked = !decision.apply_blockers.is_empty();
            for (destination_index, artist) in intended.iter().enumerate() {
                let unsafe_artist = escapes_root(artist);
                if !unsafe_artist && !simple_component(artist) {
                    return Err(unsupported_shape(format!(
                        "Artist target {artist:?} requires physical/provider-dependent path resolution"
                    )));
                }
                let mut directory = vec![];
                if settings.abc_sort {
                    let first = artist.chars().next().ok_or_else(|| {
                        unsupported_shape("An empty artist has no alphabetic component".into())
                    })?;
                    directory.push(first.to_string());
                }
                directory.push(artist.clone());
                if settings.singles_folder {
                    directory.push("סינגלים".into());
                }
                let unsafe_destination = unsafe_artist || escapes_root(&filename);
                if unsafe_destination {
                    decision.apply_blockers.push(Diagnostic {
                        code: "UNSAFE_DESTINATION".into(),
                        message: format!(
                            "Raw intended destination for artist {artist:?} and filename {filename:?} contains a parent/absolute-root escape"
                        ),
                    });
                } else if !directory.iter().all(|part| simple_component(part)) {
                    return Err(unsupported_shape(format!(
                        "Derived directory {directory:?} contains an unsupported logical component"
                    )));
                } else if !simple_component(&filename) {
                    return Err(unsupported_shape(format!(
                        "Sanitized original filename {filename:?} requires physical path resolution"
                    )));
                }
                let disposition = if unsafe_destination {
                    DestinationDisposition::Blocked
                } else if settings.exist_only
                    && !request.target.existing_directories.contains(&directory)
                {
                    DestinationDisposition::SkippedExistingOnly
                } else if recognition_blocked {
                    DestinationDisposition::Blocked
                } else {
                    DestinationDisposition::Required
                };
                let mut components = directory;
                components.push(filename.clone());
                // Project the full selected sequence, including safe candidates
                // gated by inference. Their suffix evidence must remain intact.
                // Only unsafe or existing-only-skipped intents do not reserve.
                if !unsafe_destination && disposition != DestinationDisposition::SkippedExistingOnly
                {
                    components = reserve_destination(components, &mut occupied)?;
                }
                destinations.push(DestinationIntent {
                    destination_id: fingerprint(&(
                        SCHEMA_VERSION,
                        "destination",
                        &operation_id,
                        destination_index,
                    ))?,
                    components,
                    disposition,
                });
            }
        }
        items.push(PlannedSong {
            operation_id,
            source_id: song.source_id.clone(),
            decision,
            destinations,
            copy_mode: settings.copy_mode,
        });
    }
    let mut result = Plan {
        schema_version: SCHEMA_VERSION,
        provenance: Provenance::ProvidedFacts,
        plan_id: String::new(),
        request_hash,
        ruleset_id: request.context.ruleset_id.clone(),
        items,
        warnings: vec![],
        executable: false,
    };
    result.plan_id = fingerprint(&PlanContent::from(&result))?;
    Ok(result)
}

/// Recompute the entire expected plan. A matching stored request_hash or plan_id
/// alone is never sufficient, and a successful binding check grants no authority.
/// A request that cannot regenerate this plan is stale, not a request to replan.
/// A later executor must not silently drop blocked operations and reinterpret
/// the remaining reservations. A selected subset requires a fresh plan.
pub fn verify_plan_binding(candidate: &Plan, request: &PlanRequest) -> Result<(), EngineError> {
    let expected = plan(request).map_err(|error| EngineError {
        code: ErrorCode::StalePlan,
        message: format!("Supplied request cannot regenerate the bound plan: {error}"),
    })?;
    if *candidate != expected {
        return Err(EngineError {
            code: ErrorCode::StalePlan,
            message: "Plan content differs from the regenerated supplied-facts plan".into(),
        });
    }
    Ok(())
}

// Struct order is the v1 canonical DTO order with exactly plan_id omitted.
// Do not round-trip through a generic JSON object (which changes field order).
#[derive(Serialize)]
struct PlanContent<'a> {
    schema_version: u16,
    provenance: Provenance,
    request_hash: &'a str,
    ruleset_id: &'a str,
    items: &'a [PlannedSong],
    warnings: &'a [Diagnostic],
    executable: bool,
}

impl<'a> From<&'a Plan> for PlanContent<'a> {
    fn from(plan: &'a Plan) -> Self {
        Self {
            schema_version: plan.schema_version,
            provenance: plan.provenance,
            request_hash: &plan.request_hash,
            ruleset_id: &plan.ruleset_id,
            items: &plan.items,
            warnings: &plan.warnings,
            executable: plan.executable,
        }
    }
}

fn fingerprint(value: &impl Serialize) -> Result<String, EngineError> {
    let bytes = serde_json::to_vec(value).map_err(|error| EngineError {
        code: ErrorCode::InvalidInput,
        message: format!("Canonical DTO serialization failed: {error}"),
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_request(request: &PlanRequest) -> Result<(), EngineError> {
    // Validate even an empty song list with the existing envelope rules. The
    // placeholder is validation-only and never becomes a song or a decision.
    validate_classify_request(&ClassifyRequest {
        schema_version: request.schema_version,
        context: request.context.clone(),
        song: SongFacts {
            source_id: "envelope-validation".into(),
            filename: "envelope-validation".into(),
            metadata: MetadataFacts::Present {
                artist: String::new(),
                title: String::new(),
            },
        },
    })?;
    if request.target.root_id.is_empty() {
        return Err(EngineError {
            code: ErrorCode::InvalidInput,
            message: "Target root_id must be a non-empty supplied identity".into(),
        });
    }
    let mut source_ids = BTreeSet::new();
    for song in &request.songs {
        validate_classify_request(&ClassifyRequest {
            schema_version: request.schema_version,
            context: request.context.clone(),
            song: song.clone(),
        })?;
        if !source_ids.insert(&song.source_id) {
            return Err(EngineError {
                code: ErrorCode::InvalidInput,
                message: format!("Duplicate supplied source_id {:?}", song.source_id),
            });
        }
    }
    for components in request
        .target
        .existing_directories
        .iter()
        .chain(&request.target.existing_files)
    {
        if components.is_empty() || !components.iter().all(|part| simple_component(part)) {
            return Err(unsupported_shape(format!(
                "Snapshot path {components:?} must contain simple, nonempty logical components"
            )));
        }
    }
    Ok(())
}

fn unsupported_shape(message: String) -> EngineError {
    EngineError {
        code: ErrorCode::UnsupportedTargetShape,
        message,
    }
}

fn simple_component(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && !value.contains(['/', '\\', ':', '\0'])
}

fn escapes_root(value: &str) -> bool {
    value.starts_with(['/', '\\'])
        || value.split(['/', '\\']).any(|part| part == "..")
        || (value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && value.as_bytes().get(1) == Some(&b':')
            && matches!(value.as_bytes().get(2), Some(b'/' | b'\\')))
}

fn reserve_destination(
    mut components: Vec<String>,
    occupied: &mut BTreeSet<Vec<String>>,
) -> Result<Vec<String>, EngineError> {
    let last = components.len() - 1;
    let original = components[last].clone();
    // Python 3.12 last suffix rules; a leading/trailing dot is not a suffix.
    let (stem, suffix) = match original.rfind('.') {
        Some(index) if index > 0 && index + 1 < original.len() => {
            (&original[..index], &original[index..])
        }
        _ => (original.as_str(), ""),
    };
    let mut counter = 0usize;
    while occupied.contains(&components) {
        counter = counter.checked_add(1).ok_or_else(|| EngineError {
            code: ErrorCode::InputTooLarge,
            message: "Logical collision suffix counter is exhausted".into(),
        })?;
        components[last] = format!("{stem}_{counter}{suffix}");
    }
    occupied.insert(components.clone());
    Ok(components)
}
