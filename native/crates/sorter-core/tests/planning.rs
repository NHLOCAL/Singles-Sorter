#[allow(dead_code)]
mod common;

use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};
use sorter_core::*;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn request() -> PlanRequest {
    PlanRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings::default(),
            aliases: vec![Alias {
                source: "אמן".into(),
                target: "אמן".into(),
            }],
            recognition: RecognitionFacts::Disabled,
        },
        songs: vec![SongFacts {
            source_id: "source-1".into(),
            filename: "אמן - שיר.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: "".into(),
                title: "".into(),
            },
        }],
        target: TargetSnapshot {
            root_id: "logical-target".into(),
            existing_directories: vec![],
            existing_files: vec![],
        },
    }
}

fn ner_request(artists: &[&str]) -> PlanRequest {
    let mut value = request();
    value.songs[0].filename = "song.mp3".into();
    value.context.aliases.clear();
    value.context.recognition = RecognitionFacts::Configured {
        bundle_id: "supplied-bundle".into(),
        health: RecognitionHealth::Healthy,
        classifier: BTreeMap::new(),
        ner: BTreeMap::from([("song".into(), NerFact::Artists(strings(artists)))]),
    };
    value
}

fn stale(value: &Plan, request: &PlanRequest) {
    assert_eq!(
        verify_plan_binding(value, request).unwrap_err().code,
        ErrorCode::StalePlan
    );
}

fn digest(value: &impl Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}

#[test]
fn all_88_frozen_target_layouts_keep_verbatim_artist_components() {
    let corpus = common::load_release_contract();
    let cases: Vec<_> = corpus.iter().filter(|case| case.op == "target").collect();
    assert_eq!(cases.len(), 88);
    for case in cases {
        let mut value = request();
        value.context.settings = serde_json::from_value(serde_json::json!({
            "copy_mode":case.input["copy_mode"], "abc_sort":case.input["abc_sort"],
            "exist_only":case.input["exist_only"], "singles_folder":case.input["singles_folder"],
            "main_folder_only":case.input["main_folder_only"], "duet_mode":case.input["duet_mode"]
        }))
        .unwrap();
        value.context.aliases[0].target = case.input["artist"].as_str().unwrap().into();
        let directory: Vec<String> = serde_json::from_value(case.expected.clone()).unwrap();
        value.target.existing_directories.push(directory.clone());
        let result = plan(&value).unwrap_or_else(|error| panic!("{}: {error}", case.id));
        let destination = &result.items[0].destinations[0];
        assert_eq!(
            &destination.components[..destination.components.len() - 1],
            directory,
            "{}",
            case.id
        );
        assert_eq!(
            destination.disposition,
            DestinationDisposition::Required,
            "{}",
            case.id
        );
        assert_eq!(result.items[0].copy_mode, value.context.settings.copy_mode);
        assert_eq!(result.provenance, Provenance::ProvidedFacts);
        assert!(!result.executable);
    }
}

#[test]
fn all_64_settings_directly_cover_layout_duet_copy_and_existing_only() {
    for mask in 0..64 {
        let mut value = ner_request(&["שני", "ראשון", "שני"]);
        let settings = Settings {
            copy_mode: mask & 1 != 0,
            main_folder_only: mask & 2 != 0,
            singles_folder: mask & 4 != 0,
            exist_only: mask & 8 != 0,
            abc_sort: mask & 16 != 0,
            duet_mode: mask & 32 != 0,
        };
        value.context.settings = settings.clone();
        let mut second = if settings.abc_sort {
            strings(&["ש", "שני"])
        } else {
            strings(&["שני"])
        };
        if settings.singles_folder {
            second.push("סינגלים".into());
        }
        // Only the exact complete directory exists. Parent presence is insufficient.
        value.target.existing_directories.push(second.clone());
        let before = value.clone();
        let result = plan(&value).unwrap();
        assert_eq!(
            value, before,
            "planner changed supplied settings/facts, mask {mask}"
        );
        assert_eq!(result.request_hash, digest(&value));
        assert_eq!(result.items[0].copy_mode, settings.copy_mode);
        assert_eq!(
            result.items[0].decision.artists,
            Some(strings(&["שני", "ראשון", "שני"]))
        );
        let destinations = &result.items[0].destinations;
        assert_eq!(
            destinations.len(),
            if settings.duet_mode { 3 } else { 1 },
            "mask {mask}"
        );
        let mut first_file = second.clone();
        first_file.push("song.mp3".into());
        assert_eq!(destinations[0].components, first_file, "mask {mask}");
        assert_eq!(
            destinations[0].disposition,
            DestinationDisposition::Required
        );
        if settings.duet_mode {
            let mut first = if settings.abc_sort {
                strings(&["ר", "ראשון"])
            } else {
                strings(&["ראשון"])
            };
            if settings.singles_folder {
                first.push("סינגלים".into());
            }
            first.push("song.mp3".into());
            assert_eq!(destinations[1].components, first, "mask {mask}");
            assert_eq!(
                destinations[1].disposition,
                if settings.exist_only {
                    DestinationDisposition::SkippedExistingOnly
                } else {
                    DestinationDisposition::Required
                }
            );
            second.push("song_1.mp3".into());
            assert_eq!(
                destinations[2].components, second,
                "duplicate reservation, mask {mask}"
            );
            assert_eq!(
                destinations[2].disposition,
                DestinationDisposition::Required
            );
        }
        assert!(!result.executable);
        verify_plan_binding(&result, &value).unwrap();
    }
}

#[test]
fn original_filename_is_sanitized_without_recognition_cleanup() {
    let mut value = request();
    value.songs[0].filename = "  אמן__שיר - מייל מיוזיק <>?.mp3  ".into();
    value.songs[0].metadata = MetadataFacts::Present {
        artist: "אמן".into(),
        title: "".into(),
    };
    let result = plan(&value).unwrap();
    assert_eq!(
        result.items[0].destinations[0].components,
        strings(&["אמן", "סינגלים", "אמן__שיר - מייל מיוזיק .mp3"])
    );
}

#[test]
fn collisions_reserve_snapshot_files_directories_then_prior_song_intents() {
    let mut value = request();
    value
        .target
        .existing_files
        .push(strings(&["אמן", "סינגלים", "אמן - שיר.mp3"]));
    value
        .target
        .existing_directories
        .push(strings(&["אמן", "סינגלים", "אמן - שיר_1.mp3"]));
    let mut next = value.songs[0].clone();
    next.source_id = "source-2".into();
    value.songs.push(next);
    let result = plan(&value).unwrap();
    assert_eq!(
        result.items[0].destinations[0].components.last().unwrap(),
        "אמן - שיר_2.mp3"
    );
    assert_eq!(
        result.items[1].destinations[0].components.last().unwrap(),
        "אמן - שיר_3.mp3"
    );
    assert_ne!(result.items[0].operation_id, result.items[1].operation_id);
    assert_ne!(
        result.items[0].destinations[0].destination_id,
        result.items[1].destinations[0].destination_id
    );
}

#[test]
fn collision_suffix_preserves_python_last_suffix_and_codepoint_truncation() {
    for (filename, collision) in [
        ("אמן.tar.gz", "אמן.tar_1.gz"),
        ("אמן.", "אמן._1"),
        (".אמן", ".אמן_1"),
        ("אמן", "אמן_1"),
    ] {
        let mut value = request();
        value.songs[0].filename = filename.into();
        value
            .target
            .existing_files
            .push(strings(&["אמן", "סינגלים", filename]));
        assert_eq!(
            plan(&value).unwrap().items[0].destinations[0]
                .components
                .last()
                .unwrap(),
            collision
        );
    }
    let mut value = request();
    value.songs[0].filename = format!("אמן {}.mp3", "😀".repeat(255));
    let sanitized = sanitize(&value.songs[0].filename).unwrap();
    assert_eq!(sanitized.chars().count(), 255);
    value
        .target
        .existing_files
        .push(vec!["אמן".into(), "סינגלים".into(), sanitized.clone()]);
    assert_eq!(
        plan(&value).unwrap().items[0].destinations[0]
            .components
            .last()
            .unwrap(),
        &format!("{sanitized}_1")
    );
}

#[test]
fn duplicate_duet_artists_keep_order_and_distinct_reservations() {
    let mut value = ner_request(&["שני", "ראשון", "שני"]);
    value.context.settings.duet_mode = true;
    let result = plan(&value).unwrap();
    assert_eq!(
        result.items[0]
            .destinations
            .iter()
            .map(|d| d.components.clone())
            .collect::<Vec<_>>(),
        vec![
            strings(&["שני", "סינגלים", "song.mp3"]),
            strings(&["ראשון", "סינגלים", "song.mp3"]),
            strings(&["שני", "סינגלים", "song_1.mp3"])
        ]
    );
    assert_ne!(
        result.items[0].destinations[0].destination_id,
        result.items[0].destinations[2].destination_id
    );
    value.context.settings.duet_mode = false;
    let result = plan(&value).unwrap();
    assert_eq!(
        result.items[0].decision.artists,
        Some(strings(&["שני", "ראשון", "שני"]))
    );
    assert_eq!(result.items[0].destinations.len(), 1);
}

#[test]
fn existing_only_requires_full_directory_and_zero_required_destinations_keeps_source() {
    let mut value = ner_request(&["שני", "ראשון", "שני"]);
    value.context.settings.duet_mode = true;
    value.context.settings.exist_only = true;
    value.target.existing_directories = vec![strings(&["שני"]), strings(&["ראשון"])];
    let result = plan(&value).unwrap();
    assert!(
        result.items[0]
            .destinations
            .iter()
            .all(|d| d.disposition == DestinationDisposition::SkippedExistingOnly)
    );
    // Skipped intents do not invent successful writes or reserve collision names.
    assert_eq!(
        result.items[0].destinations[0].components,
        result.items[0].destinations[2].components
    );
    let required = result.items[0]
        .destinations
        .iter()
        .filter(|d| d.disposition == DestinationDisposition::Required)
        .count();
    assert_eq!(required, 0);
    assert!(!source_delete_allowed(false, required, 0, false));
}

#[test]
fn source_deletion_guard_requires_move_nonzero_exact_verification_and_no_failure() {
    assert!(!source_delete_allowed(false, 0, 0, false));
    assert!(!source_delete_allowed(false, 2, 1, true));
    assert!(source_delete_allowed(false, 2, 2, false));
    assert!(!source_delete_allowed(true, 2, 2, false));
    assert!(!source_delete_allowed(false, 2, 1, false));
    assert!(!source_delete_allowed(false, 2, 3, false));
    assert!(!source_delete_allowed(false, 2, 2, true));
    assert!(source_delete_allowed(false, usize::MAX, usize::MAX, false));
}

#[test]
fn disabled_ai_is_not_a_blocker_and_unused_failed_bundle_does_not_block_aliases() {
    let mut value = request();
    for recognition in [
        RecognitionFacts::Disabled,
        RecognitionFacts::Configured {
            bundle_id: "failed-unused".into(),
            health: RecognitionHealth::Failed,
            classifier: BTreeMap::new(),
            ner: BTreeMap::new(),
        },
    ] {
        value.context.recognition = recognition;
        let result = plan(&value).unwrap();
        assert!(result.items[0].decision.apply_blockers.is_empty());
        assert_eq!(
            result.items[0].destinations[0].disposition,
            DestinationDisposition::Required
        );
        assert!(!result.executable);
    }
}

#[test]
fn consulted_configured_failures_keep_decision_evidence_and_block_intents() {
    let mut value = request();
    value.context.aliases.clear();
    value.songs[0].filename = "song.mp3".into();
    value.songs[0].metadata = MetadataFacts::Present {
        artist: "דוד".into(),
        title: "".into(),
    };
    for (health, fact) in [
        (RecognitionHealth::Failed, ClassifierFact::Accept),
        (RecognitionHealth::Healthy, ClassifierFact::Unavailable),
        (
            RecognitionHealth::Healthy,
            ClassifierFact::Error {
                code: "RUNTIME".into(),
            },
        ),
    ] {
        value.context.recognition = RecognitionFacts::Configured {
            bundle_id: "configured".into(),
            health,
            classifier: BTreeMap::from([("דוד".into(), fact.clone())]),
            ner: BTreeMap::new(),
        };
        let result = plan(&value).unwrap();
        let item = &result.items[0];
        assert_eq!(item.decision.artists, Some(strings(&["דוד"])));
        assert!(!item.decision.apply_blockers.is_empty());
        assert_eq!(
            item.destinations[0].components,
            strings(&["דוד", "סינגלים", "song.mp3"])
        );
        assert_eq!(
            item.destinations[0].disposition,
            DestinationDisposition::Blocked
        );
        if !matches!(fact, ClassifierFact::Accept) {
            assert!(
                item.decision
                    .warnings
                    .iter()
                    .any(|d| d.code == "CLASSIFIER_FAIL_OPEN")
            );
        }
    }
}

#[test]
fn unknown_artist_has_no_destinations_or_deletion_permission() {
    let mut value = request();
    value.context.aliases.clear();
    let result = plan(&value).unwrap();
    assert_eq!(result.items[0].decision.artists, None);
    assert!(result.items[0].destinations.is_empty());
    assert!(!source_delete_allowed(false, 0, 0, false));
}

#[test]
fn request_hash_is_canonical_versioned_struct_bytes_and_plan_hash_omits_only_id() {
    let value = request();
    let result = plan(&value).unwrap();
    assert_eq!(result.request_hash, digest(&value));
    #[derive(Serialize)]
    struct Content<'a> {
        schema_version: u16,
        provenance: Provenance,
        request_hash: &'a str,
        ruleset_id: &'a str,
        items: &'a [PlannedSong],
        warnings: &'a [Diagnostic],
        executable: bool,
    }
    assert_eq!(
        result.plan_id,
        digest(&Content {
            schema_version: result.schema_version,
            provenance: result.provenance,
            request_hash: &result.request_hash,
            ruleset_id: &result.ruleset_id,
            items: &result.items,
            warnings: &result.warnings,
            executable: result.executable,
        })
    );
    assert_eq!(result.plan_id.len(), 64);
    assert_eq!(result.request_hash.len(), 64);
    assert_eq!(result, plan(&value).unwrap());
    assert_eq!(
        result,
        plan(&serde_json::from_slice::<PlanRequest>(&serde_json::to_vec(&value).unwrap()).unwrap())
            .unwrap()
    );
    assert!(!result.executable);
}

#[test]
fn binding_covers_all_facts_aliases_settings_model_ids_and_ordered_snapshot() {
    let value = request();
    let result = plan(&value).unwrap();
    let mut variants = vec![];
    macro_rules! change {
        ($edit:expr) => {{
            let mut v = value.clone();
            ($edit)(&mut v);
            variants.push(v);
        }};
    }
    change!(|v: &mut PlanRequest| v.songs[0].source_id.push('x'));
    change!(|v: &mut PlanRequest| v.songs[0].filename.push('x'));
    change!(
        |v: &mut PlanRequest| v.songs[0].metadata = MetadataFacts::ReadError {
            code: "unreadable".into()
        }
    );
    change!(|v: &mut PlanRequest| v.context.aliases.push(v.context.aliases[0].clone()));
    change!(|v: &mut PlanRequest| v.context.aliases[0].source.push('x'));
    change!(|v: &mut PlanRequest| v.context.aliases[0].target.push('x'));
    for bit in 0..6 {
        change!(|v: &mut PlanRequest| match bit {
            0 => v.context.settings.copy_mode = true,
            1 => v.context.settings.main_folder_only = true,
            2 => v.context.settings.singles_folder = false,
            3 => v.context.settings.exist_only = true,
            4 => v.context.settings.abc_sort = true,
            _ => v.context.settings.duet_mode = true,
        });
    }
    change!(|v: &mut PlanRequest| v.target.root_id.push('x'));
    change!(|v: &mut PlanRequest| v.target.existing_directories.push(strings(&["unrelated"])));
    change!(|v: &mut PlanRequest| v.target.existing_files.push(strings(&["unrelated.mp3"])));
    change!(
        |v: &mut PlanRequest| v.context.recognition = RecognitionFacts::Configured {
            bundle_id: "new-model".into(),
            health: RecognitionHealth::Healthy,
            classifier: BTreeMap::new(),
            ner: BTreeMap::new(),
        }
    );
    for changed in variants {
        stale(&result, &changed);
        assert_ne!(result.request_hash, digest(&changed));
    }
    let mut value = ner_request(&["שני", "ראשון"]);
    value.context.settings.duet_mode = true;
    value.target.existing_files = vec![strings(&["x"]), strings(&["y"])];
    value.target.existing_directories = vec![strings(&["x-dir"]), strings(&["y-dir"])];
    let result = plan(&value).unwrap();
    let mut changed = value.clone();
    changed.target.existing_files.reverse();
    stale(&result, &changed);
    let mut changed = value.clone();
    changed.target.existing_directories.reverse();
    stale(&result, &changed);
    let mut changed = value.clone();
    if let RecognitionFacts::Configured { bundle_id, .. } = &mut changed.context.recognition {
        bundle_id.push('x');
    }
    stale(&result, &changed);
    let mut changed = value.clone();
    if let RecognitionFacts::Configured { ner, .. } = &mut changed.context.recognition {
        ner.insert("song".into(), NerFact::Artists(strings(&["ראשון", "שני"])));
    }
    stale(&result, &changed);
    let mut changed = value.clone();
    if let RecognitionFacts::Configured { classifier, .. } = &mut changed.context.recognition {
        classifier.insert("unused".into(), ClassifierFact::Reject);
    }
    stale(&result, &changed);
    let mut changed = value.clone();
    if let RecognitionFacts::Configured { health, .. } = &mut changed.context.recognition {
        *health = RecognitionHealth::Failed;
    }
    stale(&result, &changed);
    let mut value = request();
    let mut next = value.songs[0].clone();
    next.source_id.push('2');
    value.songs.push(next);
    value.context.aliases.push(Alias {
        source: "unused".into(),
        target: "unused".into(),
    });
    let result = plan(&value).unwrap();
    let mut changed = value.clone();
    changed.songs.reverse();
    stale(&result, &changed);
    let mut changed = value.clone();
    changed.context.aliases.reverse();
    stale(&result, &changed);
}

#[test]
fn inference_maps_have_key_order_canonicalization_but_arrays_retain_order() {
    let mut value = ner_request(&["שני", "ראשון"]);
    if let RecognitionFacts::Configured { ner, .. } = &mut value.context.recognition {
        ner.insert("a".into(), NerFact::Artists(vec![]));
        ner.insert("z".into(), NerFact::Artists(vec![]));
    }
    let mut reordered = value.clone();
    if let RecognitionFacts::Configured { ner, .. } = &mut reordered.context.recognition {
        let reverse: Vec<_> = ner.clone().into_iter().rev().collect();
        ner.clear();
        ner.extend(reverse);
    }
    assert_eq!(plan(&value).unwrap(), plan(&reordered).unwrap());
}

#[test]
fn tampered_complete_plan_is_stale_even_when_old_request_hash_remains() {
    let value = ner_request(&["שני", "ראשון"]);
    let result = plan(&value).unwrap();
    let mut variants = vec![];
    macro_rules! change {
        ($edit:expr) => {{
            let mut p = result.clone();
            ($edit)(&mut p);
            variants.push(p);
        }};
    }
    change!(|p: &mut Plan| p.items[0].destinations[0].components[0] = "replacement".into());
    change!(|p: &mut Plan| p.items[0].destinations[0].disposition = DestinationDisposition::Blocked);
    change!(|p: &mut Plan| p.items[0].destinations[0].destination_id.push('x'));
    change!(|p: &mut Plan| p.items[0].operation_id.push('x'));
    change!(|p: &mut Plan| p.items[0].source_id.push('x'));
    change!(|p: &mut Plan| p.items[0].copy_mode = !p.items[0].copy_mode);
    change!(|p: &mut Plan| p.items[0].decision.artists.as_mut().unwrap().reverse());
    change!(|p: &mut Plan| p.items[0].decision.trace.clear());
    change!(
        |p: &mut Plan| p.items[0].decision.apply_blockers.push(Diagnostic {
            code: "FORGED".into(),
            message: "forged".into()
        })
    );
    change!(|p: &mut Plan| p.warnings.push(Diagnostic {
        code: "FORGED".into(),
        message: "forged".into()
    }));
    change!(|p: &mut Plan| p.executable = true);
    change!(|p: &mut Plan| p.schema_version = 2);
    change!(|p: &mut Plan| p.ruleset_id.push('x'));
    change!(|p: &mut Plan| p.plan_id.push('x'));
    for tampered in variants {
        assert_eq!(tampered.request_hash, result.request_hash);
        stale(&tampered, &value);
    }
    let mut tampered = result;
    tampered.request_hash.push('x');
    stale(&tampered, &value);
}

#[test]
fn unsafe_parent_and_absolute_intents_are_preserved_raw_and_blocked() {
    for artist in [
        "..",
        "../outside",
        "nested/../outside",
        "/outside",
        "\\outside",
        "C:\\outside",
        "C:/outside",
        "\\\\server\\share",
    ] {
        let mut value = request();
        value.context.aliases[0].target = artist.into();
        let result = plan(&value).unwrap();
        let item = &result.items[0];
        assert_eq!(item.decision.artists, Some(vec![artist.into()]));
        assert_eq!(
            item.destinations[0].components,
            strings(&[artist, "סינגלים", "אמן - שיר.mp3"])
        );
        assert_eq!(
            item.destinations[0].disposition,
            DestinationDisposition::Blocked
        );
        assert!(
            item.decision
                .apply_blockers
                .iter()
                .any(|d| d.code == "UNSAFE_DESTINATION")
        );
        assert!(!result.executable);
    }
}

#[test]
fn complex_relative_and_provider_shapes_return_unsupported_without_substitution() {
    for artist in [
        "",
        ".",
        "nested/artist",
        "nested\\artist",
        "C:artist",
        "content://provider/document/42",
        "artist\0suffix",
    ] {
        let mut value = request();
        value.context.aliases[0].target = artist.into();
        assert_eq!(
            plan(&value).unwrap_err().code,
            ErrorCode::UnsupportedTargetShape,
            "{artist:?}"
        );
    }
    for path in [
        vec![],
        strings(&[""]),
        strings(&[".."]),
        strings(&["nested/artist"]),
        strings(&["C:artist"]),
    ] {
        let mut value = request();
        value.target.existing_files.push(path);
        assert_eq!(
            plan(&value).unwrap_err().code,
            ErrorCode::UnsupportedTargetShape
        );
    }
}

#[test]
fn snapshot_case_and_unicode_identity_are_exact_supplied_strings_only() {
    let mut value = request();
    value.context.aliases[0].target = "David".into();
    value.context.settings.exist_only = true;
    value
        .target
        .existing_directories
        .push(strings(&["david", "סינגלים"]));
    assert_eq!(
        plan(&value).unwrap().items[0].destinations[0].disposition,
        DestinationDisposition::SkippedExistingOnly
    );
    value
        .target
        .existing_directories
        .push(strings(&["David", "סינגלים"]));
    value
        .target
        .existing_files
        .push(strings(&["David", "סינגלים", "אמן - שיר.MP3"]));
    assert_eq!(
        plan(&value).unwrap().items[0].destinations[0]
            .components
            .last()
            .unwrap(),
        "אמן - שיר.mp3"
    );
    value.context.aliases[0].target = "é".into();
    value.target.existing_directories = vec![strings(&["e\u{301}", "סינגלים"])];
    assert_eq!(
        plan(&value).unwrap().items[0].destinations[0].disposition,
        DestinationDisposition::SkippedExistingOnly
    );
}

#[test]
fn invalid_envelope_duplicate_sources_and_empty_target_are_rejected_even_without_songs() {
    let mut value = request();
    value.songs.push(value.songs[0].clone());
    assert_eq!(plan(&value).unwrap_err().code, ErrorCode::InvalidInput);
    let mut value = request();
    value.target.root_id.clear();
    assert_eq!(plan(&value).unwrap_err().code, ErrorCode::InvalidInput);
    let mut value = request();
    value.songs.clear();
    value.schema_version = 2;
    assert_eq!(plan(&value).unwrap_err().code, ErrorCode::UnsupportedSchema);
    let mut value = request();
    value.songs.clear();
    value.context.ruleset_id = "unsupported".into();
    assert_eq!(
        plan(&value).unwrap_err().code,
        ErrorCode::UnsupportedRuleset
    );
    let mut value = ner_request(&[]);
    if let RecognitionFacts::Configured { bundle_id, .. } = &mut value.context.recognition {
        bundle_id.clear();
    }
    value.songs.clear();
    assert_eq!(plan(&value).unwrap_err().code, ErrorCode::InvalidInput);
    let mut value = request();
    value.songs.clear();
    let result = plan(&value).unwrap();
    assert!(result.items.is_empty());
    assert!(!result.executable);
}

#[test]
fn regeneration_errors_on_changed_request_are_stale_binding_results() {
    let value = request();
    let result = plan(&value).unwrap();
    let mut changed = value.clone();
    changed.schema_version = 2;
    stale(&result, &changed);
    let mut changed = value.clone();
    changed.context.ruleset_id = "other".into();
    stale(&result, &changed);
    let mut changed = value;
    changed.context.aliases[0].target = "complex/shape".into();
    stale(&result, &changed);
}

#[test]
fn safe_inference_blocked_intent_still_reserves_before_later_same_filename() {
    let mut value = request();
    value.songs[0].filename = "song.mp3".into();
    value.songs[0].metadata = MetadataFacts::Present {
        artist: "דוד".into(),
        title: "".into(),
    };
    value.context.aliases = vec![Alias {
        source: "קבוע".into(),
        target: "דוד".into(),
    }];
    value.context.recognition = RecognitionFacts::Configured {
        bundle_id: "configured".into(),
        health: RecognitionHealth::Healthy,
        classifier: BTreeMap::from([("דוד".into(), ClassifierFact::Unavailable)]),
        ner: BTreeMap::new(),
    };
    let mut later = value.songs[0].clone();
    later.source_id = "source-2".into();
    later.metadata = MetadataFacts::Present {
        artist: "קבוע".into(),
        title: "".into(),
    };
    value.songs.push(later);
    value
        .target
        .existing_files
        .push(strings(&["דוד", "סינגלים", "song.mp3"]));
    let result = plan(&value).unwrap();
    assert_eq!(
        result.items[0].destinations[0].disposition,
        DestinationDisposition::Blocked
    );
    assert!(
        result.items[0]
            .decision
            .warnings
            .iter()
            .any(|d| d.code == "CLASSIFIER_FAIL_OPEN")
    );
    assert_eq!(
        result.items[0].destinations[0].components,
        strings(&["דוד", "סינגלים", "song_1.mp3"])
    );
    assert_eq!(
        result.items[1].destinations[0].disposition,
        DestinationDisposition::Required
    );
    assert!(result.items[1].decision.apply_blockers.is_empty());
    assert_eq!(
        result.items[1].destinations[0].components,
        strings(&["דוד", "סינגלים", "song_2.mp3"])
    );
    verify_plan_binding(&result, &value).unwrap();
    // Dropping the blocked song cannot silently reinterpret remaining reservations.
    let mut subset = value;
    subset.songs.remove(0);
    stale(&result, &subset);
    assert_eq!(
        plan(&subset).unwrap().items[0].destinations[0]
            .components
            .last()
            .unwrap(),
        "song_1.mp3"
    );
}

#[test]
fn inference_blocked_existing_only_skips_missing_directory_without_reserving() {
    let mut value = request();
    value.context.settings.exist_only = true;
    value.context.aliases.clear();
    value.songs[0].filename = "song.mp3".into();
    value.songs[0].metadata = MetadataFacts::Present {
        artist: "דוד".into(),
        title: "".into(),
    };
    value.context.recognition = RecognitionFacts::Configured {
        bundle_id: "configured".into(),
        health: RecognitionHealth::Healthy,
        classifier: BTreeMap::from([("דוד".into(), ClassifierFact::Unavailable)]),
        ner: BTreeMap::new(),
    };
    let mut later = value.songs[0].clone();
    later.source_id = "source-2".into();
    value.songs.push(later);
    let result = plan(&value).unwrap();
    for item in &result.items {
        assert!(!item.decision.apply_blockers.is_empty());
        assert_eq!(
            item.destinations[0].disposition,
            DestinationDisposition::SkippedExistingOnly
        );
        assert_eq!(
            item.destinations[0].components,
            strings(&["דוד", "סינגלים", "song.mp3"])
        );
    }
}

#[test]
fn self_consistent_forged_plan_hash_is_not_authority_for_changed_destinations() {
    let value = request();
    let mut forged = plan(&value).unwrap();
    let original_request_hash = forged.request_hash.clone();
    forged.items[0].destinations[0].components[0] = "forged-safe-artist".into();
    #[derive(Serialize)]
    struct Content<'a> {
        schema_version: u16,
        provenance: Provenance,
        request_hash: &'a str,
        ruleset_id: &'a str,
        items: &'a [PlannedSong],
        warnings: &'a [Diagnostic],
        executable: bool,
    }
    forged.plan_id = digest(&Content {
        schema_version: forged.schema_version,
        provenance: forged.provenance,
        request_hash: &forged.request_hash,
        ruleset_id: &forged.ruleset_id,
        items: &forged.items,
        warnings: &forged.warnings,
        executable: forged.executable,
    });
    assert_eq!(forged.request_hash, original_request_hash);
    stale(&forged, &value);
}

#[test]
fn planner_propagates_exact_missing_or_failed_ner_without_hidden_fallback() {
    let mut value = ner_request(&["אמן"]);
    if let RecognitionFacts::Configured { ner, .. } = &mut value.context.recognition {
        ner.clear();
    }
    assert_eq!(
        plan(&value).unwrap_err().code,
        ErrorCode::MissingInferenceFact
    );
    if let RecognitionFacts::Configured { ner, .. } = &mut value.context.recognition {
        ner.insert(
            "song".into(),
            NerFact::Error {
                code: "RUNTIME".into(),
            },
        );
    }
    assert_eq!(plan(&value).unwrap_err().code, ErrorCode::InferenceError);
    if let RecognitionFacts::Configured { ner, .. } = &mut value.context.recognition {
        ner.insert("song".into(), NerFact::Unavailable);
    }
    let result = plan(&value).unwrap();
    assert!(result.items[0].destinations.is_empty());
    assert!(!result.items[0].decision.apply_blockers.is_empty());
    assert!(!result.executable);
}

#[test]
fn dot_prefixed_artists_preserve_non_alphabetic_names_but_reject_derived_dot_component() {
    for artist in [".artist", "..artist"] {
        let mut value = request();
        value.context.aliases[0].target = artist.into();
        value
            .target
            .existing_directories
            .push(strings(&[artist, "סינגלים"]));
        value
            .target
            .existing_files
            .push(strings(&[artist, "סינגלים", "אמן - שיר.mp3"]));
        let result = plan(&value).unwrap();
        assert_eq!(result.items[0].decision.artists, Some(strings(&[artist])));
        assert_eq!(
            result.items[0].destinations[0].components,
            strings(&[artist, "סינגלים", "אמן - שיר_1.mp3"])
        );
        assert_eq!(
            result.items[0].destinations[0].disposition,
            DestinationDisposition::Required
        );
        verify_plan_binding(&result, &value).unwrap();

        value.context.settings.abc_sort = true;
        value.target.existing_directories.clear();
        value.target.existing_files.clear();
        assert_eq!(
            plan(&value).unwrap_err().code,
            ErrorCode::UnsupportedTargetShape,
            "derived dot component for {artist:?}"
        );
        // The same intended directory supplied as a snapshot is unsupported too.
        value
            .target
            .existing_directories
            .push(strings(&[".", artist, "סינגלים"]));
        assert_eq!(
            plan(&value).unwrap_err().code,
            ErrorCode::UnsupportedTargetShape
        );
    }
}

#[test]
fn alphabetic_unsafe_intents_keep_raw_derived_components_instead_of_shape_substitution() {
    for artist in ["..", "../outside", "/outside", "\\outside", "C:\\outside"] {
        let mut value = request();
        value.context.settings.abc_sort = true;
        value.context.aliases[0].target = artist.into();
        let result = plan(&value).unwrap();
        let first = artist.chars().next().unwrap().to_string();
        let destination = &result.items[0].destinations[0];
        assert_eq!(
            destination.components,
            strings(&[&first, artist, "סינגלים", "אמן - שיר.mp3"])
        );
        assert_eq!(destination.disposition, DestinationDisposition::Blocked);
        assert!(
            result.items[0]
                .decision
                .apply_blockers
                .iter()
                .any(|d| d.code == "UNSAFE_DESTINATION")
        );
        assert!(!result.executable);
    }
}

#[test]
fn duplicate_unsafe_duets_do_not_reserve_or_turn_safe_siblings_into_inference_blockers() {
    let mut value = ner_request(&["../outside", "safe", "../outside", "safe"]);
    value.context.settings.duet_mode = true;
    value.context.settings.abc_sort = true;
    let result = plan(&value).unwrap();
    let destinations = &result.items[0].destinations;
    assert_eq!(destinations.len(), 4);
    assert_eq!(
        destinations[0].components,
        strings(&[".", "../outside", "סינגלים", "song.mp3"])
    );
    assert_eq!(destinations[0].components, destinations[2].components);
    assert_eq!(destinations[0].disposition, DestinationDisposition::Blocked);
    assert_eq!(destinations[2].disposition, DestinationDisposition::Blocked);
    assert_eq!(
        destinations[1].components,
        strings(&["s", "safe", "סינגלים", "song.mp3"])
    );
    assert_eq!(
        destinations[3].components,
        strings(&["s", "safe", "סינגלים", "song_1.mp3"])
    );
    assert_eq!(
        destinations[1].disposition,
        DestinationDisposition::Required
    );
    assert_eq!(
        destinations[3].disposition,
        DestinationDisposition::Required
    );
    assert!(
        result.items[0]
            .decision
            .apply_blockers
            .iter()
            .all(|d| d.code == "UNSAFE_DESTINATION")
    );
    verify_plan_binding(&result, &value).unwrap();
    assert!(!result.executable);
}
