//! Real Windows host acceptance for effect-free completion, in owned temp trees.
use super::*;
use serde_json::json;

#[test]
fn retained_clean_noop_revalidates_without_writing_any_journal_or_destination() {
    for (changed, existing_only) in [(false, false), (true, false), (false, true)] {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("source");
        let target = tree.path().join("target");
        let journal = tree.path().join("journal");
        for path in [&source, &target, &journal] {
            std::fs::create_dir(path).unwrap();
        }
        if existing_only {
            std::fs::write(
                source.join("אברהם פריד - owned.mp3"),
                include_bytes!("../../../../tests/fixtures/media/blank.mp3"),
            )
            .unwrap();
        } else {
            std::fs::write(source.join("notes.txt"), b"owned non-audio").unwrap();
        }
        let host = Host::default();
        host.set_journal_directory(journal.clone()).unwrap();
        let source_token = host
            .select_path(Role::Source, source.clone())
            .unwrap()
            .token;
        let target_token = host
            .select_path(Role::Target, target.clone())
            .unwrap()
            .token;
        let request = serde_json::from_value(json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
            "settings":{"copy_mode":true,"main_folder_only":false,"singles_folder":true,"abc_sort":false,"exist_only":existing_only,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
        let job = host.preview_with_personal(request, Some(vec![])).unwrap();
        let snapshot = host.snapshot(&job.job_id).unwrap();
        assert_eq!(snapshot.status, "completed", "{:?}", snapshot.error);
        assert!(
            snapshot
                .apply
                .as_ref()
                .is_some_and(|ready| ready.supported && ready.operation_count == 0),
            "{:?}",
            snapshot.apply
        );
        let plan_id = snapshot.plan_id.unwrap();
        if changed {
            std::fs::write(source.join("new-note.txt"), b"changed since review").unwrap();
        }
        host.start_apply(&job.job_id, &plan_id, vec![], host.claim().unwrap())
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let outcome = loop {
            let snapshot = host.execution_snapshot(&job.job_id).unwrap();
            if !snapshot.is_running() {
                break snapshot;
            }
            assert!(std::time::Instant::now() < deadline, "host no-op timed out");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        if changed {
            assert_eq!(outcome.status, "failed");
            assert!(outcome.result.is_none());
        } else {
            assert_eq!(outcome.status, "complete", "{outcome:?}");
            assert!(outcome.journal_path.is_none());
            let result = outcome.result.unwrap();
            assert_eq!(result.committed_destinations, 0);
            assert!(result.items.is_empty());
            assert_eq!(
                serde_json::to_value(result.legacy_summary.unwrap()).unwrap(),
                json!({"songs_sorted":0,"artist_folders_created":0,"albums_processed":0,"top_artists":[]})
            );
        }
        assert_eq!(std::fs::read_dir(&journal).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
        assert!(source.is_dir());
        let guard = loop {
            if let Ok(guard) = host.claim() {
                break guard;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "host ownership was not released"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(
            host.start_apply(&job.job_id, &plan_id, vec![], guard)
                .is_err()
        );
    }
}
