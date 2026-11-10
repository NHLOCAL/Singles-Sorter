// Actual native writer journals feed the fresh host; projection mutations below
// exercise contradiction handling without granting authority from saved data.
mod whole_restart_tests {
    use super::tests::{owned_tree_state, wait_for_recovery};
    use super::*;
    use sorter_executor::windows_album::{AlbumFailure, RecordedWholeAlbumInspection};

    fn no_clear(value: RecordedWholeAlbumInspection) {
        if let Ok(report) = project_whole_inspection(value) {
            assert_eq!(report.state, "needs_attention", "{report:?}");
        }
    }

    fn require_contradictions_visible(actual: &RecordedWholeAlbumInspection) {
        let mut changed = actual.clone();
        changed.recovery.recorded_finished = false;
        no_clear(changed);
        let mut changed = actual.clone();
        changed.recovery.recorded[0].status = "Complete".into();
        no_clear(changed); // Whole outcomes have the exact lowercase vocabulary.
        let mut changed = actual.clone();
        changed.files.pop();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.files[0].main = None;
        no_clear(changed);
        let mut changed = actual.clone();
        changed.membership.pop();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.membership[0].status = "changed".into();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.recovery.destination_evidence.clear();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.directories.clear();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.recovery.child_renames.clear();
        no_clear(changed);
        let mut changed = actual.clone();
        changed.recovery.recorded[0].child_renames.clear();
        no_clear(changed);
        let mut changed = actual.clone();
        changed
            .recovery
            .child_renames
            .push(changed.recovery.child_renames[0].clone());
        no_clear(changed);
        let mut changed = actual.clone();
        changed.recovery.child_renames[0].observation.identity =
            changed.recorded.target.identity.clone().unwrap();
        no_clear(changed);
        let mut changed = actual.clone();
        let mut extra = changed.recovery.recorded[0]
            .descendants
            .iter()
            .find(|(_, item)| item.size.is_some())
            .unwrap()
            .clone();
        extra.0 = vec![sorter_io::mutation::MutationName::Windows(
            "unexpected-file.txt".encode_utf16().collect(),
        )];
        changed.recovery.recorded[0].descendants.push(extra);
        no_clear(changed);
        assert_eq!(actual.recovery.child_locations.len(), 4);
        for kind in 0..5 {
            let mut changed = actual.clone();
            match kind {
                0 => {
                    changed.recovery.child_locations.pop();
                }
                1 => {
                    changed.recovery.child_locations[0].error = Some(AlbumFailure {
                        kind: "owned_error".into(),
                        detail: "owned projection regression".into(),
                        observed: None,
                    })
                }
                2 => {
                    changed.recovery.child_locations[0].observed =
                        changed.recovery.recorded[0].root.clone()
                }
                3 => changed
                    .recovery
                    .child_locations
                    .push(changed.recovery.child_locations[0].clone()),
                _ => changed.recovery.child_locations[0].planner_index = usize::MAX,
            }
            no_clear(changed);
        }
        if let Some(nested) = &actual.recovery.singles {
            assert!(nested.manifest_finished);
            let mut changed = actual.clone();
            changed.recovery.singles.as_mut().unwrap().manifest_finished = false;
            no_clear(changed);
            let mut changed = actual.clone();
            changed
                .recovery
                .singles
                .as_mut()
                .unwrap()
                .transfers
                .as_mut()
                .unwrap()
                .append_blocked = true;
            no_clear(changed);
            let mut changed = actual.clone();
            changed
                .files
                .last_mut()
                .unwrap()
                .transfer
                .as_mut()
                .unwrap()
                .copy_metadata = sorter_executor::recovery::CopyMetadataRecoveryStatus::Mismatch;
            no_clear(changed);
        }
    }

    #[test]
    fn fresh_host_reopens_whole_albums_without_fabricating_child_transfers() {
        for root_album in [true, false] {
            let owned = tempfile::tempdir().unwrap();
            let root = owned.path().canonicalize().unwrap();
            let source = root.join("source");
            let target = root.join("target");
            let journal = root.join("journal");
            for path in [&source, &target, &journal] {
                std::fs::create_dir(path).unwrap();
            }
            let album = if root_album {
                source.clone()
            } else {
                source.join("album")
            };
            if !root_album {
                std::fs::create_dir(&album).unwrap();
            }
            for track in 1..=3 {
                let name = if track == 1 {
                    "1-שיר  ארוך.mp3".into()
                } else {
                    format!("{track}-שיר.mp3")
                };
                std::fs::write(album.join(name), album_restart_tests::audio(track)).unwrap();
            }
            std::fs::write(album.join("cover.jpg"), b"owned artwork").unwrap();
            std::fs::write(album.join("cover.jpg:notes"), b"owned historical ADS").unwrap();
            std::fs::create_dir_all(album.join("booklet  notes/deep")).unwrap();
            std::fs::write(
                album.join("booklet  notes/deep/readme.txt"),
                b"owned nested text",
            )
            .unwrap();
            if !root_album {
                std::fs::write(
                    source.join("שלום עולם-single.mp3"),
                    include_bytes!("../../../../tests/fixtures/media/blank.mp3"),
                )
                .unwrap();
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
            let personal = "שלום עולם,שם ממופה\n".as_bytes().to_vec();
            let request = serde_json::from_value(serde_json::json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
                "settings":{"copy_mode":false,"main_folder_only":false,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
            let job = host
                .preview_with_personal(request, Some(personal.clone()))
                .unwrap();
            let preview = host.snapshot(&job.job_id).unwrap();
            assert!(
                preview.apply.as_ref().is_some_and(|a| a.supported),
                "{preview:?}"
            );
            host.start_apply(
                &job.job_id,
                preview.plan_id.as_ref().unwrap(),
                personal,
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let execution = host.execution_snapshot(&job.job_id).unwrap();
                if !execution.is_running() && !host.shared.busy.load(Ordering::Acquire) {
                    assert_eq!(execution.status, "complete", "{execution:?}");
                    assert_eq!(
                        execution.result.as_ref().unwrap().committed_destinations,
                        if root_album { 5 } else { 6 }
                    );
                    break;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            drop(host);
            assert!(!album.exists());
            let before = owned_tree_state(&root);
            let destination = target.join("שם ממופה/אלבום");
            let stream = destination.join("cover.jpg:notes");
            let before_stream = std::fs::read(&stream).unwrap();
            let reopened = Host::default();
            reopened.set_journal_directory(journal.clone()).unwrap();
            let page = reopened.list_recovery(0, 10).unwrap();
            let record = page
                .records
                .iter()
                .find(|r| r.verification_scope == Some(WHOLE_VERIFICATION_SCOPE))
                .unwrap();
            assert_eq!(record.state, "inspection_needed");
            assert_eq!(record.operation_count, if root_album { 5 } else { 6 });
            assert_eq!(record.original_parent.is_some(), root_album);
            let token = record.token.clone();
            assert!(
                reopened
                    .select_recovery_parent_path(&token, target.clone())
                    .is_err()
            );
            let selected_source = if root_album {
                SelectedSource::Missing({
                    let selection = reopened
                        .select_recovery_parent_path(&token, root.clone())
                        .unwrap();
                    reopened
                        .lock()
                        .unwrap()
                        .recovery
                        .parents
                        .iter()
                        .find(|p| p.selection.token == selection.token)
                        .unwrap()
                        .clone()
                })
            } else {
                assert!(
                    reopened
                        .select_recovery_parent_path(&token, root.clone())
                        .is_err()
                );
                let selection = reopened
                    .select_recovery_path(Role::Source, source.clone())
                    .unwrap();
                SelectedSource::Present(
                    reopened
                        .lock()
                        .unwrap()
                        .recovery
                        .roots
                        .iter()
                        .find(|r| r.selection.token == selection.token)
                        .unwrap()
                        .clone(),
                )
            };
            let source_token = match &selected_source {
                SelectedSource::Present(root) => root.selection.token.clone(),
                SelectedSource::Missing(parent) => parent.selection.token.clone(),
            };
            let selection = reopened
                .select_recovery_path(Role::Target, target.clone())
                .unwrap();
            let target_token = selection.token.clone();
            let selected_target = reopened
                .lock()
                .unwrap()
                .recovery
                .roots
                .iter()
                .find(|r| r.selection.token == selection.token)
                .unwrap()
                .clone();
            let retained = reopened
                .lock()
                .unwrap()
                .recovery
                .records
                .iter()
                .find(|r| r.token == token)
                .unwrap()
                .clone();
            let inspected =
                read_whole_inspection(retained, selected_source, selected_target, None, 0).unwrap();
            assert_eq!(inspected.recovery.recorded.len(), 1);
            assert_eq!(inspected.recorded.child_locations.len(), 2);
            assert_eq!(inspected.recovery.child_renames.len(), 2);
            assert_eq!(
                inspected
                    .files
                    .iter()
                    .filter(|r| r.main.is_some() && r.transfer.is_none())
                    .count(),
                5
            );
            assert_eq!(
                inspected
                    .files
                    .iter()
                    .filter(|r| r.transfer.is_some() && r.main.is_none())
                    .count(),
                usize::from(!root_album)
            );
            let report = project_whole_inspection(inspected.clone()).unwrap();
            assert_eq!(report.state, "consistent", "{report:?}");
            assert_eq!(report.verification_scope, Some(WHOLE_VERIFICATION_SCOPE));
            assert!(report.operations.iter().all(|r| r.status == "Verified"));
            require_contradictions_visible(&inspected);
            assert_eq!(owned_tree_state(&root), before);
            assert_eq!(std::fs::read(&stream).unwrap(), before_stream);

            let fired = Arc::new(AtomicBool::new(false));
            let callback_fired = fired.clone();
            let inspection_token = token.clone();
            let source_choice = source_token.clone();
            let target_choice = target_token.clone();
            let run = reopened
                .start_recovery_run(
                    "inspect",
                    Some((
                        token.clone(),
                        source_token.clone(),
                        Some(target_token.clone()),
                    )),
                    None,
                    reopened.claim().unwrap(),
                    move |cancel, host, id, cursor| {
                        let close_host = host.clone();
                        let progress_host = host.clone();
                        let progress_id = id.to_owned();
                        let baseline = AtomicU64::new(u64::MAX);
                        let control = InspectionControl::with_progress_interval(
                            cancel.cancelled.clone(),
                            std::time::Duration::ZERO,
                            move |p| {
                                if p.operation.is_some() {
                                    let _ = baseline.compare_exchange(
                                        u64::MAX,
                                        p.bytes_read,
                                        Ordering::AcqRel,
                                        Ordering::Acquire,
                                    );
                                    if p.bytes_read > baseline.load(Ordering::Acquire) {
                                        callback_fired.store(true, Ordering::Release);
                                        close_host.cancel_active();
                                    }
                                }
                                progress_host.update_recovery_progress(&progress_id, p.into());
                            },
                        );
                        host.inspect_recovery_with_control(
                            &inspection_token,
                            &source_choice,
                            Some(&target_choice),
                            Some(&control),
                            cursor,
                        )
                        .map(RecoveryOutput::Report)
                    },
                )
                .unwrap();
            let stopped = wait_for_recovery(&reopened, &run.run_id);
            assert_eq!(stopped.status, "cancelled", "{stopped:?}");
            assert!(fired.load(Ordering::Acquire));
            assert!(stopped.report.is_none());
            assert!(!reopened.shared.busy.load(Ordering::Acquire));
            assert_eq!(owned_tree_state(&root), before);
            assert_eq!(std::fs::read(&stream).unwrap(), before_stream);
            let resumed = reopened
                .start_recovery_inspection(
                    token.clone(),
                    source_token.clone(),
                    Some(target_token.clone()),
                    Some(run.run_id),
                    reopened.claim().unwrap(),
                )
                .unwrap();
            let completed = wait_for_recovery(&reopened, &resumed.run_id);
            assert_eq!(completed.status, "completed", "{completed:?}");
            assert_eq!(completed.report.unwrap().state, "consistent");
            assert_eq!(
                completed.progress.completed_operations,
                if root_album { 5 } else { 6 }
            );
            assert_eq!(owned_tree_state(&root), before);

            // Historical Whole journals do not contain ADS digests. Keep this
            // limitation explicit rather than manufacturing a verification claim.
            std::fs::write(&stream, b"changed historical ADS").unwrap();
            let after_ads = owned_tree_state(&root);
            let report = reopened
                .inspect_recovery(&token, &source_token, &target_token)
                .unwrap();
            assert_eq!(report.state, "consistent");
            assert_eq!(report.verification_scope, Some(WHOLE_VERIFICATION_SCOPE));
            assert_eq!(owned_tree_state(&root), after_ads);
            assert_eq!(std::fs::read(&stream).unwrap(), b"changed historical ADS");
            if root_album {
                std::fs::write(
                    destination.join("cover.jpg"),
                    b"externally changed main data",
                )
                .unwrap();
            } else {
                std::fs::create_dir(destination.join("unexpected-empty-directory")).unwrap();
            }
            let changed_tree = owned_tree_state(&root);
            let report = reopened
                .inspect_recovery(&token, &source_token, &target_token)
                .unwrap();
            assert_eq!(report.state, "needs_attention", "{report:?}");
            assert_eq!(owned_tree_state(&root), changed_tree);
            assert_eq!(std::fs::read(&stream).unwrap(), b"changed historical ADS");
            reopened.list_recovery(0, 10).unwrap();
            assert!(
                reopened
                    .inspect_recovery(&token, &source_token, &target_token)
                    .is_err()
            );
            assert_eq!(owned_tree_state(&root), changed_tree);
        }
    }
}
