//! Actual metadata-driven mixed-library effects on owned Windows trees.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*};
use sorter_executor::{
    binding::Resources, digest, results::JobStatus, windows_album::*,
    windows_copy::WindowsPlanningStorage,
};
use sorter_io::{
    desktop::{
        DesktopStorage, WindowsDirectoryReadLimits, windows_mutation::WindowsSelectedRootGrant,
    },
    metadata::{MetadataReader, StorageMetadataProvider},
};
use std::{path::Path, sync::atomic::AtomicBool};
#[path = "../../../tests/metadata_read_compatibility/fixture_support.rs"]
mod fixture_support;
fn files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = vec![];
    for e in std::fs::read_dir(root).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(files(&p));
        } else {
            let bytes = std::fs::read(&p).unwrap();
            out.push((p, bytes));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
fn exercise(copy: bool, changed: bool) {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    for p in [&source, &target, &journal] {
        std::fs::create_dir(p).unwrap();
    }
    fixture_support::mixed_library(&source);
    let before = files(&source);
    assert_eq!(before.len(), 103);
    let cancel = AtomicBool::new(false);
    let resources = Resources {
        resource_ids: vec!["fixture".into()],
        adapter_versions: vec!["native".into()],
        ai_profile: AiProfile::Disabled,
        resource_hashes: vec![digest(b"fixture")],
        adapter_hashes: vec![digest(b"native")],
        model_hash: None,
    };
    let mut expected_outputs = Vec::new();
    let mut admitted_sources = Vec::new();
    let validated = {
        let mut reads = WindowsPlanningStorage::for_album_preview(
            &source,
            &target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &cancel,
        )
        .unwrap();
        let mut metadata_reads = WindowsPlanningStorage::for_album_preview(
            &source,
            &target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &cancel,
        )
        .unwrap();
        let mut metadata = StorageMetadataProvider {
            storage: &mut metadata_reads,
            reader: MetadataReader,
        };
        let request = JobRequestV2 {
            mode: JobMode::Sort,
            source: reads.reference(&source).unwrap(),
            target: Some(reads.reference(&target).unwrap()),
            settings: Settings {
                copy_mode: copy,
                main_folder_only: false,
                singles_folder: false,
                ..Settings::default()
            },
            alias_rows: vec![
                vec!["זמר אלבום".into(), "Mapped Artist".into()],
                vec!["שלום עולם".into(), "Mapped Artist".into()],
                vec!["Fallback Artist".into(), "Mapped Artist".into()],
                vec!["Artist".into(), "Mapped Artist".into()],
            ],
            resource_ids: resources.resource_ids.clone(),
            adapter_versions: resources.adapter_versions.clone(),
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(
            &request,
            &mut reads,
            &mut metadata,
            &mut DisabledRecognition,
        )
        .unwrap();
        assert_eq!(plan.blockers, vec!["PHYSICAL_APPLY_UNAVAILABLE"]);
        assert_eq!(plan.operations.len(), 24);
        use std::os::windows::ffi::OsStringExt;
        for operation in &plan.operations {
            let (original, destination) = match &operation.kind {
                OperationKind::AlbumTransfer {
                    source,
                    destination,
                    ..
                } => (source, destination),
                OperationKind::SingleTransfer {
                    source, transfer, ..
                } => (source, &transfer.destination),
                _ => panic!("unexpected operation"),
            };
            let StorageRef::WindowsPath { utf16 } = &original.reference else {
                panic!("Windows source expected")
            };
            let original_path = std::path::PathBuf::from(std::ffi::OsString::from_wide(utf16));
            assert_eq!(Some(&destination.root), request.target.as_ref());
            let destination_path = destination
                .components
                .iter()
                .fold(target.clone(), |p, c| p.join(c));
            for (p, bytes) in &before {
                if p == &original_path {
                    expected_outputs.push((destination_path.clone(), bytes.clone()));
                } else if let Ok(relative) = p.strip_prefix(&original_path) {
                    expected_outputs.push((destination_path.join(relative), bytes.clone()));
                }
            }
            admitted_sources.push(original_path);
        }
        expected_outputs.sort_by(|a, b| a.0.cmp(&b.0));
        let scope = prepare_album_scope(&plan, &mut reads, &resources, &cancel).unwrap();
        if changed {
            assert!(source.join("metadata-only-v1.mp3").is_file());
            std::fs::write(
                source.join("metadata-only-v1.mp3"),
                b"changed after preview",
            )
            .unwrap();
        }
        let result = validate_album_execution(
            &plan,
            &mut metadata,
            &mut DisabledRecognition,
            &resources,
            &scope,
            &cancel,
        );
        if changed {
            assert!(matches!(
                result,
                Err(sorter_executor::ExecutionError::Stale)
            ));
            assert!(files(&target).is_empty());
            assert!(files(&journal).is_empty());
            return;
        }
        result.unwrap()
    };
    let grants = {
        let mut reads =
            DesktopStorage::new(&[source.clone(), target.clone(), journal.clone()]).unwrap();
        let mut grant = |p: &std::path::PathBuf| {
            let r = reads.reference(p).unwrap();
            WindowsSelectedRootGrant::new(p.clone(), reads.stat(&r).unwrap().identity.unwrap())
                .unwrap()
        };
        (grant(&source), grant(&target), grant(&journal))
    };
    let mut job =
        SelectedWindowsAlbumJob::prepare(grants.0, grants.1, grants.2, validated, &cancel).unwrap();
    let result = job.run(&cancel, &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.albums_completed, 1);
    drop(job);
    assert_eq!(expected_outputs.len(), 63);
    assert_eq!(files(&target), expected_outputs);
    if copy {
        assert_eq!(files(&source), before);
    } else {
        let expected_remaining = before
            .into_iter()
            .filter(|(p, _)| {
                !admitted_sources
                    .iter()
                    .any(|admitted| p == admitted || p.starts_with(admitted))
            })
            .collect::<Vec<_>>();
        assert_eq!(expected_remaining.len(), 40);
        assert_eq!(files(&source), expected_remaining);
        for admitted in admitted_sources {
            assert!(
                !admitted.exists(),
                "acknowledged source must be retired: {admitted:?}"
            );
        }
    }
}
#[test]
fn mixed_103_real_metadata_copy_preserves_original_decisions_and_all_sources() {
    exercise(true, false);
}
#[test]
fn mixed_103_real_metadata_move_retains_policy_skipped_album() {
    exercise(false, false);
}
#[test]
fn changed_real_metadata_source_refuses_before_any_effect() {
    exercise(true, true);
}
