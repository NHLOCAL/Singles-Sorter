use sorter_core::{
    Settings,
    discovery::{Capabilities, EntryKind, EntryObservation, ObjectIdentity, StorageRef},
    job_plan::{AiProfile, DecisionV2, JobMode, JobPlanV2, JobRequestV2, PredictedEffect, ReadPoint},
    metadata::{FieldError, LegacyField, LegacyItem, MetadataError, MetadataObservation},
};
use sorter_executor::{digest, no_op::is_clean_sort_noop};

fn entry() -> EntryObservation {
    EntryObservation {
        reference: StorageRef::TestOpaque {
            id: "skipped".into(),
        },
        identity: Some(ObjectIdentity::Test {
            id: "skipped".into(),
        }),
        parent_identity: None,
        name: "unknown.wav".into(),
        kind: EntryKind::File,
        size: Some(1),
        modified: Some("unchanged".into()),
        capabilities: Capabilities::default(),
    }
}
fn read() -> ReadPoint {
    ReadPoint {
        source: entry(),
        after_operations: 0,
        depends_on: vec![],
        virtually_present: true,
    }
}
fn seal(plan: &mut JobPlanV2) {
    let mut value = serde_json::to_value(&*plan).unwrap();
    value["binding_sha256"] = serde_json::json!("");
    plan.binding_sha256 = digest(&serde_json::to_vec(&value).unwrap());
}
fn skipped() -> JobPlanV2 {
    let mut plan = JobPlanV2 {
        schema_version: 2,
        ruleset_id: sorter_core::RULESET_ID.into(),
        binding_sha256: String::new(),
        content_binding: "synthetic-noop".into(),
        executable: false,
        request: JobRequestV2 {
            mode: JobMode::Sort,
            source: StorageRef::TestOpaque {
                id: "source".into(),
            },
            target: Some(StorageRef::TestOpaque {
                id: "target".into(),
            }),
            settings: Settings::default(),
            alias_rows: vec![],
            resource_ids: vec![],
            adapter_versions: vec![],
            ai_profile: AiProfile::Disabled,
        },
        observations: vec![entry()],
        metadata_observations: vec![],
        precollected_visits: vec![],
        decisions: vec![DecisionV2::Single {
            read: read(),
            artists: vec![],
        }],
        operations: vec![],
        predicted_effects: vec![],
        predicted_legacy_counters: Default::default(),
        warnings: vec!["AI intentionally disabled; no native inference bundle is used".into()],
        blockers: vec!["PHYSICAL_APPLY_UNAVAILABLE".into()],
    };
    seal(&mut plan);
    plan
}
#[test]
fn skipped_clean_sort_is_eligible_but_unverified_changed_plan_is_not() {
    let mut plan = skipped();
    assert!(is_clean_sort_noop(&plan));
    plan.request.settings.copy_mode = !plan.request.settings.copy_mode;
    assert!(!is_clean_sort_noop(&plan));
    seal(&mut plan);
    assert!(is_clean_sort_noop(&plan));
    plan.request.mode = JobMode::FixNames;
    seal(&mut plan);
    assert!(!is_clean_sort_noop(&plan));
}
#[test]
fn no_op_never_hides_planning_errors_vanished_items_or_recognition_diagnostics() {
    for decision in [
        DecisionV2::ItemError {
            read: read(),
            error: "read failed".into(),
        },
        DecisionV2::Vanished { read: read() },
    ] {
        let mut plan = skipped();
        plan.decisions.push(decision);
        seal(&mut plan);
        assert!(!is_clean_sort_noop(&plan));
    }
    for blocker in [
        "MALFORMED_ALIAS_ROWS",
        "LINK_OR_REPARSE",
        "SOURCE_TARGET_OVERLAP",
        "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS",
    ] {
        let mut plan = skipped();
        plan.blockers.push(blocker.into());
        seal(&mut plan);
        assert!(!is_clean_sort_noop(&plan));
    }
    let mut plan = skipped();
    plan.warnings.push("MODEL_RUNTIME_FAILURE".into());
    seal(&mut plan);
    assert!(!is_clean_sort_noop(&plan));
}
#[test]
fn swallowed_legacy_metadata_open_get_and_value_failures_are_not_successful_noops() {
    let failures = [
        MetadataObservation::Open {
            source: entry(),
            after_operations: 0,
            result: Err(MetadataError::LegacyOpen {
                kind: "Parser".into(),
                detail: "unreadable".into(),
            }),
        },
        MetadataObservation::Get {
            source: entry(),
            after_operations: 0,
            field: LegacyField::Title,
            result: Err(FieldError::get("Read", "unreadable")),
        },
        MetadataObservation::Get {
            source: entry(),
            after_operations: 0,
            field: LegacyField::Title,
            result: Ok(LegacyItem {
                values: vec![],
                truthy: false,
                value: Err(FieldError::get("Value", "unreadable")),
            }),
        },
    ];
    for observation in failures {
        let mut plan = skipped();
        plan.metadata_observations.push(observation);
        seal(&mut plan);
        assert!(!is_clean_sort_noop(&plan));
    }
}

fn empty_duet() -> JobPlanV2 {
    let mut plan = skipped();
    plan.request.settings.copy_mode = false;
    plan.request.settings.duet_mode = true;
    plan.request.settings.exist_only = true;
    let DecisionV2::Single { artists, .. } = &mut plan.decisions[0] else { panic!("fixture") };
    *artists = vec!["אמן".into(), "זמר".into()];
    plan.predicted_effects.push(PredictedEffect::DuetDeleteIntention {
        source: entry(), after_operations: vec![], blocked: true,
    });
    plan.blockers.push("HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS".into());
    seal(&mut plan);
    plan
}

#[test]
fn zero_eligible_historical_duet_is_only_a_source_retaining_noop() {
    let plan = empty_duet();
    assert!(is_clean_sort_noop(&plan));
    assert!(plan.operations.is_empty());
    assert_eq!(plan.predicted_legacy_counters, Default::default());
    // Admission neither consumes the original evidence nor changes its blocker.
    assert!(matches!(&plan.predicted_effects[0], PredictedEffect::DuetDeleteIntention {
        after_operations, blocked: true, ..
    } if after_operations.is_empty()));
}

#[test]
fn historical_noop_rejects_unmatched_nonempty_unblocked_and_failed_evidence() {
    for variant in 0..12 {
        let mut plan = empty_duet();
        match variant {
            0 => plan.request.settings.copy_mode = true,
            1 => plan.request.settings.duet_mode = false,
            2 => plan.request.settings.exist_only = false,
            3 => {
                let PredictedEffect::DuetDeleteIntention { after_operations, .. } = &mut plan.predicted_effects[0] else { panic!("fixture") };
                after_operations.push(0);
            }
            4 => {
                let PredictedEffect::DuetDeleteIntention { blocked, .. } = &mut plan.predicted_effects[0] else { panic!("fixture") };
                *blocked = false;
            }
            5 => {
                let PredictedEffect::DuetDeleteIntention { source, .. } = &mut plan.predicted_effects[0] else { panic!("fixture") };
                source.identity = Some(ObjectIdentity::Test { id: "replacement".into() });
            }
            6 => plan.predicted_effects.push(plan.predicted_effects[0].clone()),
            7 => plan.blockers.push("READ_FAILED".into()),
            8 => plan.decisions.push(DecisionV2::ItemError { read: read(), error: "read failed".into() }),
            9 => plan.predicted_legacy_counters.songs_sorted = 1,
            10 => plan.predicted_effects.push(PredictedEffect::SourceUnavailable { operation: 0, source: entry().reference }),
            11 => plan.blockers.retain(|blocker| blocker == "PHYSICAL_APPLY_UNAVAILABLE"),
            _ => unreachable!(),
        }
        seal(&mut plan);
        assert!(!is_clean_sort_noop(&plan), "variant {variant}");
    }
}

#[test]
fn second_duet_with_invalid_read_point_cannot_disappear_from_noop_bijection() {
    for dependency in [false, true] {
        let mut plan = empty_duet();
        let mut second = read();
        second.source.reference = StorageRef::TestOpaque { id: "second".into() };
        second.source.identity = Some(ObjectIdentity::Test { id: "second".into() });
        if dependency {
            second.depends_on.push(99);
        } else {
            second.after_operations = 1;
        }
        plan.observations.push(second.source.clone());
        plan.decisions.push(DecisionV2::Single {
            read: second,
            artists: vec!["אמן אחר".into(), "זמר נוסף".into()],
        });
        seal(&mut plan);
        assert!(!is_clean_sort_noop(&plan), "dependency variant {dependency}");
    }
}
