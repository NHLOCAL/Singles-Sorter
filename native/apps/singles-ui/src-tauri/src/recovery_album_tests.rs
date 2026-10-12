// These owned fixtures exercise the actual host/reader boundary on Windows.
mod album_restart_tests {
    use super::*;
    use super::tests::{owned_tree_state, wait_for_recovery};
    use sorter_core::discovery::ObjectIdentity;
    use sorter_executor::recovery::CopyMetadataRecoveryStatus;
    use sorter_executor::windows_album::AlbumDirectoryEvidenceStatus;

    pub(super) fn audio(track: usize) -> Vec<u8> {
        fn sync(size: usize) -> [u8; 4] {
            [((size >> 21) & 127) as u8, ((size >> 14) & 127) as u8,
                ((size >> 7) & 127) as u8, (size & 127) as u8]
        }
        let mut body = vec![];
        for (id, value) in [(b"TPE1", "שלום עולם".to_owned()), (b"TPE2", "שלום עולם".to_owned()),
            (b"TALB", "אלבום".to_owned()), (b"TRCK", track.to_string())] {
            let mut payload = vec![3];
            payload.extend(value.as_bytes());
            body.extend(id);
            body.extend(sync(payload.len()));
            body.extend([0, 0]);
            body.extend(payload);
        }
        let mut file = b"ID3\x04\0\0".to_vec();
        file.extend(sync(body.len()));
        file.extend(body);
        file.extend(include_bytes!("../../../../tests/fixtures/media/blank.mp3"));
        file
    }

    #[test]
    fn fresh_host_reopens_file_albums_with_exact_locations_and_all_evidence() {
        for copy in [true, false] {
            let local = tempfile::tempdir().unwrap();
            let other = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
            let local_root = local.path().canonicalize().unwrap();
            let target_root = other.path().canonicalize().unwrap();
            let source = local_root.join("source");
            let journal = local_root.join("journal");
            let target = target_root.join("target");
            for path in [&source, &journal, &target] { std::fs::create_dir(path).unwrap(); }
            for track in 1..=3 { std::fs::write(source.join(format!("{track}-שיר.mp3")), audio(track)).unwrap(); }
            std::fs::write(source.join("cover.jpg"), b"owned artwork").unwrap();
            std::fs::create_dir_all(source.join("nested/deep")).unwrap();
            std::fs::write(source.join("nested/deep/readme.txt"), b"owned nested file").unwrap();
            let files = ["1-שיר.mp3", "2-שיר.mp3", "3-שיר.mp3", "cover.jpg", "nested/deep/readme.txt"];
            if !copy {
                for name in files { std::fs::write(source.join(format!("{name}:notes")), b"owned stream").unwrap(); }
            }
            if !copy {
                match (inspect_root(&source).unwrap().identity, inspect_root(&target).unwrap().identity) {
                    (Some(ObjectIdentity::Windows { volume: left, .. }), Some(ObjectIdentity::Windows { volume: right, .. })) => assert_ne!(left, right, "Actual cross-volume Move required"),
                    _ => panic!("Actual Windows identities required"),
                }
            }
            let host = Host::default();
            host.set_journal_directory(journal.clone()).unwrap();
            let source_token = host.select_path(Role::Source, source.clone()).unwrap().token;
            let target_token = host.select_path(Role::Target, target.clone()).unwrap().token;
            let personal = "שלום עולם,שם ממופה\n".as_bytes().to_vec();
            let request = serde_json::from_value(serde_json::json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
                "settings":{"copy_mode":copy,"main_folder_only":false,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
            let job = host.preview_with_personal(request, Some(personal.clone())).unwrap();
            let preview = host.snapshot(&job.job_id).unwrap();
            assert!(preview.apply.as_ref().is_some_and(|value| value.supported), "{preview:?}");
            host.start_apply(&job.job_id, preview.plan_id.as_ref().unwrap(), personal, host.claim().unwrap()).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let snapshot = host.execution_snapshot(&job.job_id).unwrap();
                if !snapshot.is_running() && !host.shared.busy.load(Ordering::Acquire) {
                    assert_eq!(snapshot.status, "complete", "{snapshot:?}");
                    assert_eq!(snapshot.result.as_ref().unwrap().committed_destinations, files.len());
                    break;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert_eq!(source.exists(), copy);
            drop(host);
            let before = (owned_tree_state(&local_root), owned_tree_state(&target_root));
            let destination = target.join("שם ממופה/אלבום");
            let streams = || if copy { vec![] } else { files.iter().map(|name| std::fs::read(destination.join(format!("{name}:notes"))).unwrap()).collect::<Vec<_>>() };
            let streams_before = streams();
            let reopened = Host::default();
            reopened.set_journal_directory(journal.clone()).unwrap();
            let listed = reopened.start_recovery_list(0, 10, reopened.claim().unwrap()).unwrap();
            let page = wait_for_recovery(&reopened, &listed.run_id).page.unwrap();
            assert_eq!(page.records.len(), 1);
            let record = &page.records[0];
            assert_eq!(record.family, "album");
            assert_eq!(record.state, "inspection_needed");
            assert_eq!(record.operation_count, files.len());
            assert_eq!(record.original_parent.is_some(), !copy);
            if copy { assert!(reopened.select_recovery_parent_path(&record.token, local_root.clone()).is_err()); }
            let token = record.token.clone();
            assert!(reopened.select_recovery_parent_path(&token, target_root.clone()).is_err());
            let source_token = if copy {
                reopened.select_recovery_path(Role::Source, source.clone()).unwrap().token
            } else {
                reopened.select_recovery_parent_path(&token, local_root.clone()).unwrap().token
            };
            let target_token = reopened.select_recovery_path(Role::Target, target.clone()).unwrap().token;
            assert!(reopened.inspect_recovery(&token, "old execution token", &target_token).is_err());
            let inspected = reopened.inspect_recovery(&token, &source_token, &target_token).unwrap();
            assert_eq!(inspected.state, "consistent", "{inspected:?}");
            assert_eq!(inspected.operations.len(), files.len());
            assert!(inspected.operations.iter().all(|item| item.status == "Committed"));
            assert!(inspected.evidence.iter().any(|entry| entry.role == "source_root" && entry.status == if copy { "Verified" } else { "AbsentExpected" }));

            // The same live façade output must not yield an all-clear after any
            // independent terminal, physical-file, cleanup or metadata condition is lost.
            let (retained, selected, target_selection) = {
                let state = reopened.lock().unwrap();
                let retained = state.recovery.records.iter().find(|entry| entry.token == token).unwrap().clone();
                let selected = if copy {
                    SelectedSource::Present(state.recovery.roots.iter().find(|entry| entry.selection.token == source_token).unwrap().clone())
                } else {
                    SelectedSource::Missing(state.recovery.parents.iter().find(|entry| entry.selection.token == source_token).unwrap().clone())
                };
                let selected_target = state.recovery.roots.iter().find(|entry| entry.selection.token == target_token).unwrap().clone();
                (retained, selected, selected_target)
            };
            let actual = read_album_inspection(retained, selected, target_selection, None, 0).unwrap();
            for condition in ["finished", "status", "album", "append", "file", "metadata", "directory", "cleanup", "mapping"] {
                let mut incomplete = actual.clone();
                match condition {
                    "finished" => incomplete.recovery.manifest_finished = false,
                    "status" => incomplete.recovery.recorded_status = Some("Partial".into()),
                    "album" => incomplete.recovery.completed_albums.clear(),
                    "append" => incomplete.recovery.transfers.as_mut().unwrap().append_blocked = true,
                    "file" => incomplete.files[0].recovery = None,
                    "metadata" => incomplete.files[0].recovery.as_mut().unwrap().copy_metadata = CopyMetadataRecoveryStatus::Mismatch,
                    "directory" => incomplete.recovery.directories[0].status = AlbumDirectoryEvidenceStatus::Unverified,
                    "cleanup" => incomplete.recovery.pending_cleanup.push(actual.recorded.source.reference.clone()),
                    "mapping" => incomplete.files[0].file.physical_operation += 1,
                    _ => unreachable!(),
                }
                match project_album_inspection(incomplete) {
                    Ok(report) => assert_eq!(report.state, "needs_attention", "{condition}: {report:?}"),
                    Err(_) => assert_eq!(condition, "mapping"),
                }
            }
            assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), before);
            assert_eq!(streams(), streams_before);

            // Cancel through the actual host owner at an observed media chunk,
            // then resume with the same read-only selection. No sleep race.
            let saw_chunk = Arc::new(AtomicBool::new(false));
            let observed_chunk = saw_chunk.clone();
            let inspection_token = token.clone();
            let source_choice = source_token.clone();
            let target_choice = target_token.clone();
            let run = reopened.start_recovery_run("inspect", Some((token.clone(), source_token.clone(), Some(target_token.clone()))), None,
                reopened.claim().unwrap(), move |cancel, host, id, cursor| {
                    let close_host = host.clone();
                    let progress_host = host.clone();
                    let progress_id = id.to_owned();
                    let media_start = AtomicU64::new(u64::MAX);
                    let control = InspectionControl::with_progress_interval(cancel.cancelled.clone(), std::time::Duration::ZERO, move |progress| {
                        if progress.stage == "files" {
                            let _ = media_start.compare_exchange(u64::MAX, progress.bytes_read, Ordering::AcqRel, Ordering::Acquire);
                            if progress.bytes_read > media_start.load(Ordering::Acquire) {
                                observed_chunk.store(true, Ordering::Release);
                                close_host.cancel_active();
                            }
                        }
                        progress_host.update_recovery_progress(&progress_id, progress.into());
                    });
                    host.inspect_recovery_with_control(&inspection_token, &source_choice, Some(&target_choice), Some(&control), cursor).map(RecoveryOutput::Report)
                }).unwrap();
            let stopped = wait_for_recovery(&reopened, &run.run_id);
            assert_eq!(stopped.status, "cancelled", "{stopped:?}");
            assert!(saw_chunk.load(Ordering::Acquire));
            assert!(stopped.report.is_none());
            assert!(!reopened.shared.busy.load(Ordering::Acquire));
            assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), before);
            assert_eq!(streams(), streams_before);
            let resumed = reopened.start_recovery_inspection(token.clone(), source_token.clone(), Some(target_token.clone()), Some(run.run_id), reopened.claim().unwrap()).unwrap();
            let completed = wait_for_recovery(&reopened, &resumed.run_id);
            assert_eq!(completed.status, "completed", "{completed:?}");
            assert_eq!(completed.report.unwrap().state, "consistent");
            assert_eq!(completed.progress.completed_operations, files.len());
            assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), before);
            assert_eq!(streams(), streams_before);

            // A changed stream remains a current warning even with an intact
            // complete manifest. Inspection itself must not repair or rewrite it.
            if !copy {
                std::fs::write(destination.join("cover.jpg:notes"), b"changed data").unwrap();
                let modified = (owned_tree_state(&local_root), owned_tree_state(&target_root));
                let changed_streams = streams();
                let report = reopened.inspect_recovery(&token, &source_token, &target_token).unwrap();
                assert_eq!(report.state, "needs_attention");
                assert!(report.operations.iter().any(|entry| entry.status != "Committed"));
                assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), modified);
                assert_eq!(streams(), changed_streams);
                // A foreign directory at the old root must never be treated as
                // expected absence or adopted by the missing-source selection.
                std::fs::create_dir(&source).unwrap();
                let replaced = (owned_tree_state(&local_root), owned_tree_state(&target_root));
                assert!(reopened.inspect_recovery(&token, &source_token, &target_token).is_err());
                assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), replaced);
            }
            if copy {
                // Discovery's digest remains binding even when identity, length
                // and modified time are preserved by an external journal edit.
                let manifest = journal.join(&record.journal_name);
                let modified = std::fs::metadata(&manifest).unwrap().modified().unwrap();
                let mut bytes = std::fs::read(&manifest).unwrap();
                bytes[0] ^= 1;
                std::fs::write(&manifest, &bytes).unwrap();
                std::fs::File::options().write(true).open(&manifest).unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(modified)).unwrap();
                let changed = (owned_tree_state(&local_root), owned_tree_state(&target_root));
                let error = reopened.inspect_recovery(&token, &source_token, &target_token).unwrap_err();
                assert_eq!(error.code, "JOURNAL_CHANGED");
                assert_eq!((owned_tree_state(&local_root), owned_tree_state(&target_root)), changed);
            }
            // Refresh revokes parent selections as well as source/target tokens.
            reopened.list_recovery(0, 10).unwrap();
            assert!(reopened.inspect_recovery(&token, &source_token, &target_token).is_err());
        }
    }
}
