//! Album execution uses the unchanged retained plan. Planning/metadata adapters
//! are dropped before the native driver acquires its exact directory-rename lease.
use super::*;
use sorter_executor::{
    results::JobStatus,
    windows_album::{
        AlbumScope, SelectedAlbumRecovery, SelectedAlbumResult, SelectedWindowsAlbumJob,
        validate_album_execution,
    },
    windows_copy::WindowsPlanningStorage,
};
use sorter_io::desktop::{WindowsDirectoryReadLimits, windows_mutation::WindowsSelectedRootGrant};

impl Host {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_album(
        &self,
        id: &str,
        plan: &JobPlanV2,
        source: &Path,
        target: &Path,
        directory: PathBuf,
        scope: &AlbumScope,
        recognition: &mut dyn RecognitionProvider,
        cancellation: &Cancellation,
        retained_journal: Option<sorter_core::discovery::ObjectIdentity>,
    ) -> Result<()> {
        let copied_album = retained_journal.is_some();
        let prepared = (|| -> Result<SelectedWindowsAlbumJob> {
            let journal = selected::journal_grant(directory.clone(), retained_journal)?;
            // No live source handle may escape this block: the selected source can
            // itself be the album that the unchanged planner intends to move.
            let validated = {
                let mut reads = WindowsPlanningStorage::for_album_preview(
                    source,
                    target,
                    WindowsDirectoryReadLimits::ALBUM_DEFAULT,
                    &cancellation.cancelled,
                )
                .map_err(storage_error)?;
                let mut metadata = StorageMetadataProvider {
                    storage: &mut reads,
                    reader: MetadataReader,
                };
                validate_album_execution(
                    plan,
                    &mut metadata,
                    recognition,
                    scope.resources(),
                    scope,
                    &cancellation.cancelled,
                )
                .map_err(storage_error)?
            };
            let grant = |path: PathBuf, identity| {
                WindowsSelectedRootGrant::new(path, identity).map_err(storage_error)
            };
            SelectedWindowsAlbumJob::prepare(
                grant(
                    source.to_owned(),
                    scope.execution_scope().source_root.clone(),
                )?,
                grant(
                    target.to_owned(),
                    scope.execution_scope().target_root.clone(),
                )?,
                journal,
                validated,
                &cancellation.cancelled,
            )
            .map_err(storage_error)
        })();
        // Validation/preparation never creates journals or destination effects.
        // Keep cancellation in that boundary distinct from an execution failure.
        if cancellation.is_cancelled() {
            self.execution_error(id, "cancelled", "הביצוע בוטל לפני תחילת ההעברה.");
            return Ok(());
        }
        let mut job = prepared?;
        self.update_album(id, |snapshot| {
            snapshot.status = "running";
            snapshot.journal_path = Some(directory.to_string_lossy().into_owned());
        });
        let result = job.run(&cancellation.cancelled, &mut |progress| {
            self.update_album(id, |snapshot| {
                snapshot.operation = Some(progress.operation);
                snapshot.bytes_written = progress.bytes_written;
                snapshot.source_length = progress.source_length;
            });
        });
        if !job.journal_locations().is_empty() {
            if cancellation.is_cancelled() {
                self.update_album(id, |snapshot| {
                    snapshot.recovery_error = Some("VERIFICATION_CANCELLED".into());
                });
            } else {
                let inspected = if copied_album {
                    let control = self.verification_control(id, cancellation);
                    job.inspect_file_recovery_controlled(&control)
                } else {
                    job.inspect_recovery(&cancellation.cancelled)
                };
                match inspected {
                    Ok(report) => self.update_album(id, |snapshot| {
                        let inspection = recovery(report);
                        if reconciliation_incomplete(&inspection) {
                            snapshot.recovery_error =
                                Some("ALBUM_RECONCILIATION_INCOMPLETE".into());
                        }
                        snapshot.recovery = Some(inspection);
                    }),
                    Err(error) => self.update_album(id, |snapshot| {
                        snapshot.recovery_error =
                            Some(if error == sorter_executor::ExecutionError::Cancelled {
                                "VERIFICATION_CANCELLED".into()
                            } else {
                                error.to_string()
                            });
                    }),
                }
            }
        }
        match result {
            Ok(result) => self.update_album(id, |snapshot| {
                snapshot.status = match result.status {
                    JobStatus::Complete => "complete",
                    JobStatus::Partial => "partial",
                    JobStatus::Cancelled => "cancelled",
                    JobStatus::NeedsRecovery => "needs_recovery",
                };
                let legacy_summary = sorter_executor::legacy_summary::LegacySummary::from_receipts(
                    &plan.operations,
                    result
                        .transfers
                        .as_ref()
                        .map(|transfers| transfers.committed_operations.as_slice())
                        .unwrap_or_default(),
                    &result
                        .created_directories
                        .iter()
                        .map(|entry| entry.reference.clone())
                        .collect::<Vec<_>>(),
                    result.albums_completed,
                    &result.artist_files,
                );
                let mut projected = project(
                    result,
                    scope.execution_scope().inspected_sources.len(),
                    &plan
                        .operations
                        .iter()
                        .filter_map(|operation| {
                            matches!(
                                operation.kind,
                                sorter_core::job_plan::OperationKind::AlbumTransfer { .. }
                            )
                            .then_some(operation.id)
                        })
                        .collect::<Vec<_>>(),
                );
                projected.legacy_summary = Some(legacy_summary);
                snapshot.result = Some(projected);
                snapshot.operation = None;
            }),
            Err(error) => self.execution_error(
                id,
                if job.journal_locations().is_empty() && cancellation.is_cancelled() {
                    "cancelled"
                } else {
                    "needs_recovery"
                },
                &format!("הביצוע נעצר: {error}. יש לבדוק את המקור והיעד לפני ניסיון נוסף."),
            ),
        }
        Ok(())
    }
    fn update_album(&self, id: &str, update: impl FnOnce(&mut ExecutionSnapshot)) {
        if let Ok(mut inner) = self.lock()
            && let Some(snapshot) = inner
                .job
                .as_mut()
                .filter(|job| job.snapshot.job_id == id)
                .and_then(|job| job.execution.as_mut())
        {
            update(snapshot);
        }
    }
}

fn project(
    result: SelectedAlbumResult,
    source_items: usize,
    album_operations: &[usize],
) -> ExecutionResult {
    let completed_albums = result
        .completed_album_operations
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let mut projected = result
        .transfers
        .map(selected::project)
        .unwrap_or(ExecutionResult {
            source_items,
            committed_destinations: 0,
            moved_sources: 0,
            failed_operations: 0,
            blocked_operations: 0,
            recovery_operations: 0,
            cancelled_operations: 0,
            items: vec![],
            repair: None,
            albums: None,
            legacy_summary: None,
        });
    // Copy's transfer report already includes every album file. Whole Move
    // receipts contain album descendants; its optional transfer report contains
    // deferred singles only. Never count the same file through both reports.
    projected.source_items = source_items;
    projected.albums = Some(AlbumSummary {
        completed: result.albums_completed,
        child_names_changed: result.child_renames.len(),
        created_directories: result.created_directories.len(),
        removed_directories: result.removed_directories.len(),
        retained_directories: result.retained_directories.len(),
    });
    for album in result.whole_albums {
        let renamed = !album.child_renames.is_empty();
        let status = match album.status.as_str() {
            "complete" => {
                let files = album
                    .descendants
                    .iter()
                    .filter(|(_, observed)| observed.size.is_some())
                    .count();
                projected.committed_destinations += files;
                projected.moved_sources += files;
                "complete"
            }
            "failed" => {
                projected.failed_operations += 1;
                if renamed { "partial" } else { "failed" }
            }
            "cancelled" => {
                projected.cancelled_operations += 1;
                if renamed { "partial" } else { "cancelled" }
            }
            _ => {
                projected.recovery_operations += 1;
                "needs_recovery"
            }
        };
        projected.items.push(ExecutionItem {
            operation: album.operation,
            status,
            detail: album.detail,
            repair: None,
        });
    }
    // Child renames can precede a stopped whole-directory handoff. They are real
    // name effects, but never evidence that any file reached its sorted target.
    let mut observed_operations = projected
        .items
        .iter()
        .map(|item| item.operation)
        .collect::<std::collections::HashSet<_>>();
    for renamed in result.child_renames {
        if observed_operations.insert(renamed.operation) {
            projected.items.push(ExecutionItem {
                operation: renamed.operation,
                status: "partial",
                detail: "Album child names changed before transfer stopped".into(),
                repair: None,
            });
        }
    }
    // A file receipt does not establish its album's metadata/cleanup completion.
    // Use synced original operation IDs, including earlier successful albums
    // when a later album stopped; never infer completion from a global count.
    let album_operations = album_operations
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    for item in &mut projected.items {
        if item.status == "complete"
            && album_operations.contains(&item.operation)
            && !completed_albums.contains(&item.operation)
        {
            item.status = "partial";
        }
    }
    projected
}

fn record_directories(
    inspection: &mut RecoveryInspection,
    role: &'static str,
    entries: Vec<sorter_executor::windows_album::AlbumDirectoryEvidence>,
    completed: &std::collections::HashSet<sorter_core::discovery::ObjectIdentity>,
) {
    use sorter_executor::windows_album::AlbumDirectoryEvidenceStatus as Status;
    for entry in entries {
        let moved = completed.contains(&entry.expected_identity);
        let rejected = match entry.status {
            Status::Replaced | Status::Unverified => true,
            Status::SameIdentity => {
                matches!(role, "source_album" | "removed_source_directory") && moved
            }
            Status::Absent => {
                role == "destination_directory" || (role == "destination_album" && moved)
            }
        };
        if rejected {
            inspection.rejected_directories += 1;
        } else if matches!(entry.status, Status::SameIdentity) {
            inspection.observed_directories += 1;
        }
        inspection.directory_evidence.push(RecoveryDirectory {
            reference: entry.reference,
            role,
            status: format!("{:?}", entry.status),
        });
    }
}
fn reconciliation_incomplete(inspection: &RecoveryInspection) -> bool {
    inspection.terminal_records_complete != Some(true)
        || inspection.append_blocked == Some(true)
        || inspection.incomplete_tail
        || inspection.pending_directories > 0
        || inspection.rejected_directories > 0
        || !inspection.pending_operations.is_empty()
        || inspection.metadata_issues.is_some_and(|count| count > 0)
        || inspection
            .transfers
            .iter()
            .any(|item| !matches!(item.status.as_str(), "Committed" | "verified"))
}
fn merge_terminal_evidence(
    inspection: &mut RecoveryInspection,
    finished: bool,
    append_blocked: bool,
) {
    inspection.terminal_records_complete =
        Some(inspection.terminal_records_complete.unwrap_or(true) && finished);
    inspection.append_blocked = Some(inspection.append_blocked.unwrap_or(false) || append_blocked);
}
fn record_file_directories(
    inspection: &mut RecoveryInspection,
    entries: Vec<sorter_executor::windows_album::AlbumDirectoryEvidence>,
    removed: Vec<(StorageRef, sorter_core::discovery::ObjectIdentity)>,
) {
    // Only validated CleanupObserved records qualify an exact original reference
    // and full identity as an expected absence. Never normalize path aliases here.
    let removed = removed
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    for entry in entries {
        let acknowledged_removed =
            removed.contains(&(entry.reference.clone(), entry.expected_identity.clone()));
        let identities = if acknowledged_removed {
            std::collections::HashSet::from([entry.expected_identity.clone()])
        } else {
            std::collections::HashSet::new()
        };
        record_directories(
            inspection,
            if acknowledged_removed {
                "removed_source_directory"
            } else {
                "destination_directory"
            },
            vec![entry],
            &identities,
        );
    }
}
fn recovery(report: SelectedAlbumRecovery) -> RecoveryInspection {
    let terminal_complete = report.manifest_finished
        && report
            .recorded_status
            .as_deref()
            .is_none_or(|status| status == "Complete");
    let mut pending = report
        .pending_directories
        .into_iter()
        .chain(report.pending_cleanup)
        .chain(report.pending_metadata)
        .collect::<std::collections::HashSet<_>>();
    let metadata_issues = report
        .directory_metadata_status
        .iter()
        .filter(|(_, status)| status != "verified")
        .count()
        + report.transfers.as_ref().map_or(0, |transfers| {
            transfers
                .operations
                .iter()
                .filter(|operation| {
                    matches!(
                        operation.copy_metadata,
                        sorter_executor::recovery::CopyMetadataRecoveryStatus::Unverified
                            | sorter_executor::recovery::CopyMetadataRecoveryStatus::Mismatch
                    )
                })
                .count()
        });
    let mut inspection = RecoveryInspection {
        observed_directories: 0,
        pending_directories: 0,
        rejected_directories: 0,
        incomplete_tail: report.incomplete_tail,
        pending_operations: vec![],
        directory_evidence: vec![],
        metadata_issues: Some(metadata_issues),
        terminal_records_complete: Some(terminal_complete),
        append_blocked: Some(
            report
                .transfers
                .as_ref()
                .is_some_and(|transfers| transfers.append_blocked),
        ),
        transfers: report
            .transfers
            .map(|transfers| {
                transfers
                    .operations
                    .into_iter()
                    .map(|operation| RecoveryItem {
                        operation: operation.operation,
                        status: format!("{:?}", operation.status),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    record_file_directories(
        &mut inspection,
        report.directories,
        report.removed_directory_references,
    );
    if let Some(whole) = report.whole {
        inspection.incomplete_tail |= whole.incomplete_tail;
        merge_terminal_evidence(&mut inspection, whole.recorded_finished, false);
        // The dispatcher mirrors these paths in the outer report. Count exact
        // references once, while retaining every pending original operation ID.
        pending.extend(whole.pending_directories);
        inspection.pending_operations.extend(whole.pending_albums);
        let completed = whole
            .recorded
            .iter()
            .filter(|item| item.status == "complete")
            .filter_map(|item| item.root.as_ref().map(|root| root.identity.clone()))
            .collect();
        record_directories(
            &mut inspection,
            "source_album",
            whole.source_evidence,
            &completed,
        );
        record_directories(
            &mut inspection,
            "destination_album",
            whole.destination_evidence,
            &completed,
        );
        inspection
            .transfers
            .extend(whole.files.into_iter().map(|entry| RecoveryItem {
                operation: entry.operation,
                status: entry.status,
            }));
        if let Some(singles) = whole.singles {
            let singles = recovery(*singles);
            inspection.observed_directories += singles.observed_directories;
            inspection.pending_directories += singles.pending_directories;
            inspection.rejected_directories += singles.rejected_directories;
            inspection.incomplete_tail |= singles.incomplete_tail;
            merge_terminal_evidence(
                &mut inspection,
                singles.terminal_records_complete.unwrap_or(false),
                singles.append_blocked.unwrap_or(false),
            );
            inspection.transfers.extend(singles.transfers);
            inspection
                .pending_operations
                .extend(singles.pending_operations);
            inspection
                .directory_evidence
                .extend(singles.directory_evidence);
            *inspection
                .metadata_issues
                .as_mut()
                .expect("album metadata count") += singles.metadata_issues.unwrap_or_default();
        }
    }
    inspection.pending_directories += pending.len();
    inspection
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sorter_executor::{results::JobResult, windows_album::WholeAlbumOutcome};
    fn result() -> SelectedAlbumResult {
        SelectedAlbumResult {
            effect_receipts: vec![],
            child_renames: vec![],
            artist_files: vec![],
            whole_albums: vec![],
            status: JobStatus::Complete,
            transfers: None,
            albums_completed: 0,
            completed_album_operations: vec![],
            created_directories: vec![],
            removed_directories: vec![],
            retained_directories: vec![],
            detail: String::new(),
        }
    }
    #[test]
    fn physical_projection_does_not_double_count_album_and_single_receipts() {
        let mut result = result();
        let observed = json!({"reference":{"TestOpaque":{"id":"file"}},"identity":{"Test":{"id":"file"}},"parent_identity":{"Test":{"id":"album"}},"actual_name":{"Windows":[102]},"size":12});
        let directory = json!({"reference":{"TestOpaque":{"id":"directory"}},"identity":{"Test":{"id":"directory"}},"parent_identity":{"Test":{"id":"album"}},"actual_name":{"Windows":[100]},"size":null});
        result.whole_albums.push(WholeAlbumOutcome {
            child_renames: vec![],
            operation: 0,
            status: "complete".into(),
            detail: String::new(),
            failure: None,
            root: None,
            descendants: vec![
                (vec![], serde_json::from_value(observed).unwrap()),
                (vec![], serde_json::from_value(directory).unwrap()),
            ],
        });
        result.albums_completed = 1;
        result.completed_album_operations.push(0);
        assert_eq!(result.completed_album_operations, vec![0]);
        result.transfers = Some(JobResult {
            committed_operations: vec![],
            effect_receipts: vec![],
            status: JobStatus::Complete,
            source_items: 1,
            committed_destinations: 1,
            moved_sources: 1,
            failed_operations: 0,
            blocked_operations: 0,
            recovery_operations: 0,
            cancelled_operations: 0,
            items: vec![],
        });
        result.albums_completed = 1;
        let result = project(result, 2, &[0]);
        assert_eq!(result.source_items, 2);
        assert_eq!(result.committed_destinations, 2);
        assert_eq!(result.moved_sources, 2);
        assert_eq!(result.albums.unwrap().completed, 1);
        assert_eq!(result.items[0].operation, 0);
    }
    #[test]
    fn committed_prefix_without_terminal_record_still_requires_verification() {
        let mut inspection = RecoveryInspection {
            observed_directories: 1,
            pending_directories: 0,
            rejected_directories: 0,
            incomplete_tail: false,
            transfers: vec![RecoveryItem {
                operation: 0,
                status: "Committed".into(),
            }],
            pending_operations: vec![],
            directory_evidence: vec![],
            metadata_issues: Some(0),
            terminal_records_complete: Some(true),
            append_blocked: Some(false),
        };
        assert!(!reconciliation_incomplete(&inspection));
        // A whole manifest or its nested singles may end cleanly at a newline
        // after Committed but before Finished. Physical effects remain observed.
        merge_terminal_evidence(&mut inspection, false, false);
        merge_terminal_evidence(&mut inspection, true, false);
        assert_eq!(inspection.terminal_records_complete, Some(false));
        assert!(reconciliation_incomplete(&inspection));
        assert_eq!(inspection.transfers[0].status, "Committed");
        inspection.terminal_records_complete = Some(true);
        merge_terminal_evidence(&mut inspection, true, true);
        merge_terminal_evidence(&mut inspection, true, false);
        assert_eq!(inspection.append_blocked, Some(true));
        assert!(reconciliation_incomplete(&inspection));
    }
    #[test]
    fn whole_recovery_keeps_replaced_roots_and_accepts_expected_source_absence() {
        use sorter_core::discovery::ObjectIdentity;
        use sorter_executor::windows_album::{
            AlbumDirectoryEvidence, AlbumDirectoryEvidenceStatus as Status,
        };
        let identity = ObjectIdentity::Test { id: "owned".into() };
        let completed = [identity.clone()].into_iter().collect();
        let mut inspection = RecoveryInspection {
            observed_directories: 0,
            pending_directories: 1,
            rejected_directories: 0,
            incomplete_tail: false,
            transfers: vec![],
            pending_operations: vec![7],
            directory_evidence: vec![],
            metadata_issues: Some(0),
            terminal_records_complete: Some(true),
            append_blocked: Some(false),
        };
        let entry = |status| AlbumDirectoryEvidence {
            reference: StorageRef::TestOpaque { id: "owned".into() },
            expected_identity: identity.clone(),
            observed_identity: None,
            status,
        };
        record_directories(
            &mut inspection,
            "source_album",
            vec![entry(Status::Absent)],
            &completed,
        );
        assert_eq!(inspection.rejected_directories, 0);
        record_directories(
            &mut inspection,
            "destination_album",
            vec![entry(Status::Replaced)],
            &completed,
        );
        assert_eq!(inspection.rejected_directories, 1);
        assert_eq!(inspection.directory_evidence.len(), 2);
        assert_eq!(inspection.pending_operations, vec![7]);
        assert_eq!(inspection.pending_directories, 1);
    }
    #[test]
    fn copied_cleanup_absence_requires_exact_original_reference_and_identity() {
        use sorter_core::discovery::ObjectIdentity;
        use sorter_executor::windows_album::{
            AlbumDirectoryEvidence, AlbumDirectoryEvidenceStatus as Status,
        };
        let reference = StorageRef::TestOpaque {
            id: "selected-spelling".into(),
        };
        let identity = ObjectIdentity::Test { id: "owned".into() };
        for case in [
            "exact",
            "other_reference",
            "other_identity",
            "unacknowledged",
            "reappeared",
            "foreign",
            "unknown",
        ] {
            let mut inspection = RecoveryInspection {
                observed_directories: 0,
                pending_directories: 0,
                rejected_directories: 0,
                incomplete_tail: false,
                transfers: vec![],
                pending_operations: vec![],
                directory_evidence: vec![],
                metadata_issues: Some(0),
                terminal_records_complete: Some(true),
                append_blocked: Some(false),
            };
            let removed = match case {
                "other_reference" => vec![(
                    StorageRef::TestOpaque {
                        id: "native-spelling".into(),
                    },
                    identity.clone(),
                )],
                "other_identity" => vec![(
                    reference.clone(),
                    ObjectIdentity::Test {
                        id: "foreign".into(),
                    },
                )],
                "unacknowledged" => vec![],
                _ => vec![(reference.clone(), identity.clone())],
            };
            let status = match case {
                "reappeared" => Status::SameIdentity,
                "foreign" => Status::Replaced,
                "unknown" => Status::Unverified,
                _ => Status::Absent,
            };
            record_file_directories(
                &mut inspection,
                vec![AlbumDirectoryEvidence {
                    reference: reference.clone(),
                    expected_identity: identity.clone(),
                    observed_identity: None,
                    status,
                }],
                removed,
            );
            assert_eq!(
                reconciliation_incomplete(&inspection),
                case != "exact",
                "{case}"
            );
            assert_eq!(
                inspection.rejected_directories,
                usize::from(case != "exact")
            );
        }
    }
    fn audio(track: usize) -> Vec<u8> {
        fn sync(size: usize) -> [u8; 4] {
            [
                ((size >> 21) & 127) as u8,
                ((size >> 14) & 127) as u8,
                ((size >> 7) & 127) as u8,
                (size & 127) as u8,
            ]
        }
        let mut body = vec![];
        for (id, value) in [
            (b"TPE1", "שלום עולם".to_owned()),
            (b"TPE2", "שלום עולם".to_owned()),
            (b"TALB", "אלבום".to_owned()),
            (b"TRCK", track.to_string()),
        ] {
            let mut payload = vec![3];
            payload.extend(value.as_bytes());
            body.extend(id);
            body.extend(sync(payload.len()));
            body.extend([0, 0]);
            body.extend(payload);
        }
        let mut file = b"ID3\x04\x00\x00".to_vec();
        file.extend(sync(body.len()));
        file.extend(body);
        file.extend(include_bytes!("../../../../tests/fixtures/media/blank.mp3"));
        file
    }
    #[test]
    fn retained_host_root_album_copy_and_move_use_real_metadata_and_fresh_grants() {
        for copy in [true, false] {
            let tree = tempfile::tempdir().unwrap();
            let source = tree.path().join("source");
            let target = tree.path().join("target");
            let journal = tree.path().join("journal");
            for path in [&source, &target, &journal] {
                std::fs::create_dir(path).unwrap();
            }
            for track in 1..=3 {
                std::fs::write(source.join(format!("{track}-שיר.mp3")), audio(track)).unwrap();
            }
            std::fs::write(source.join("cover.jpg"), b"owned sidecar").unwrap();
            let host = Host::default();
            host.set_journal_directory(journal).unwrap();
            let source_token = host
                .select_path(Role::Source, source.clone())
                .unwrap()
                .token;
            let target_token = host
                .select_path(Role::Target, target.clone())
                .unwrap()
                .token;
            let personal = "שלום עולם,שם ממופה\n".as_bytes().to_vec();
            let request = serde_json::from_value(json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
                "settings":{"copy_mode":copy,"main_folder_only":false,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
            let job = host
                .preview_with_personal(request, Some(personal.clone()))
                .unwrap();
            let snapshot = host.snapshot(&job.job_id).unwrap();
            assert_eq!(snapshot.status, "completed", "{:?}", snapshot.error);
            assert!(
                snapshot.apply.as_ref().is_some_and(|ready| ready.supported),
                "{:?}",
                snapshot.apply
            );
            let plan_id = snapshot.plan_id.unwrap();
            host.start_apply(
                &job.job_id,
                &plan_id,
                personal.clone(),
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let outcome = loop {
                let snapshot = host.execution_snapshot(&job.job_id).unwrap();
                if !snapshot.is_running() {
                    break snapshot;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "host album execution timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            assert_eq!(outcome.status, "complete", "{outcome:?}");
            assert!(outcome.recovery_error.is_none(), "{outcome:?}");
            let result = outcome.result.unwrap();
            assert_eq!(result.albums.unwrap().completed, 1);
            assert_eq!(result.committed_destinations, 4);
            assert_eq!(
                serde_json::to_value(result.legacy_summary.unwrap()).unwrap(),
                json!({"songs_sorted":0,"artist_folders_created":0,"albums_processed":1,"top_artists":[["שלום עולם",4]]})
            );
            assert_eq!(result.moved_sources, if copy { 0 } else { 4 });
            assert_eq!(source.exists(), copy);
            for track in 1..=3 {
                assert_eq!(
                    std::fs::read(target.join(format!("שם ממופה/אלבום/{track}-שיר.mp3"))).unwrap(),
                    audio(track)
                );
            }
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
                host.start_apply(&job.job_id, &plan_id, personal, guard)
                    .is_err()
            );
        }
    }
    #[test]
    fn copied_album_keeps_earlier_completed_row_when_later_cleanup_stops() {
        use sorter_executor::results::{ItemResult, ItemStatus};
        let mut actual = result();
        actual.status = JobStatus::Cancelled;
        actual.albums_completed = 1;
        actual.completed_album_operations = vec![0];
        actual.transfers = Some(JobResult {
            status: JobStatus::Complete,
            source_items: 2,
            committed_destinations: 2,
            committed_operations: vec![0, 1],
            effect_receipts: vec![],
            moved_sources: 2,
            failed_operations: 0,
            blocked_operations: 0,
            recovery_operations: 0,
            cancelled_operations: 0,
            items: [0, 1]
                .into_iter()
                .map(|operation| ItemResult {
                    operation,
                    status: ItemStatus::Complete,
                    detail: String::new(),
                    error: None,
                    observations: vec![],
                })
                .collect(),
        });
        let projected = project(actual, 2, &[0, 1]);
        assert_eq!(projected.committed_destinations, 2);
        assert_eq!(projected.moved_sources, 2);
        assert_eq!(projected.items[0].status, "complete");
        assert_eq!(projected.items[1].status, "partial");
    }

    #[test]
    fn album_preparation_cancellation_is_no_effect_and_stale_errors_stay_errors() {
        use sorter_core::{ClassifierFact, job_plan::RecognitionError};
        struct CancelAtProfile {
            host: Host,
            id: String,
            called: Arc<AtomicBool>,
        }
        impl RecognitionProvider for CancelAtProfile {
            fn profile(&self) -> AiProfile {
                self.called.store(true, Ordering::Release);
                self.host.cancel_apply(&self.id).unwrap();
                AiProfile::Disabled
            }
            fn classify(
                &mut self,
                _: &str,
            ) -> std::result::Result<ClassifierFact, RecognitionError> {
                panic!("Disabled fixture cannot infer")
            }
            fn ner(&mut self, _: &str) -> std::result::Result<Vec<String>, RecognitionError> {
                panic!("Disabled fixture cannot infer")
            }
        }
        for action in ["cancel_before_read", "cancel_during_validation", "stale"] {
            let owned = tempfile::tempdir().unwrap();
            let source = owned.path().join("source");
            let target = owned.path().join("target");
            let journal = owned.path().join("journal");
            for path in [&source, &target, &journal] {
                std::fs::create_dir(path).unwrap();
            }
            for track in 1..=3 {
                std::fs::write(source.join(format!("{track}.mp3")), audio(track)).unwrap();
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
            let job = host.preview_with_personal(serde_json::from_value(json!({"mode":"Sort","source_token":source_token,
                "target_token":target_token,"settings":{"copy_mode":false,"main_folder_only":false,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap(),
                Some("שלום עולם,שם ממופה\n".as_bytes().to_vec())).unwrap();
            let (plan, scope, cancellation) = {
                let mut inner = host.lock().unwrap();
                let retained = inner.job.as_mut().unwrap();
                let plan = retained._plan.as_ref().unwrap().clone();
                let scope = retained
                    .selected
                    .as_ref()
                    .unwrap()
                    .test_album_scope()
                    .unwrap()
                    .clone();
                retained.execution = Some(ExecutionSnapshot {
                    host_schema: 1,
                    job_id: job.job_id.clone(),
                    plan_id: plan.binding_sha256.clone(),
                    status: "preparing",
                    operation_count: plan.operations.len(),
                    operation: None,
                    bytes_written: 0,
                    source_length: 0,
                    journal_path: None,
                    error: None,
                    result: None,
                    recovery: None,
                    recovery_error: None,
                    verification: None,
                });
                (plan, scope, retained.cancellation.clone())
            };
            if action == "cancel_before_read" {
                host.cancel_apply(&job.job_id).unwrap();
            }
            if action == "stale" {
                std::fs::write(source.join("late.txt"), b"owned late member").unwrap();
            }
            let before = album_inspection_tree(owned.path());
            let called = Arc::new(AtomicBool::new(false));
            let mut recognition: Box<dyn RecognitionProvider> =
                if action == "cancel_during_validation" {
                    Box::new(CancelAtProfile {
                        host: host.clone(),
                        id: job.job_id.clone(),
                        called: called.clone(),
                    })
                } else {
                    Box::new(DisabledRecognition)
                };
            let guard = host.claim().unwrap();
            let outcome = host.execute_album(
                &job.job_id,
                &plan,
                &source,
                &target,
                journal,
                &scope,
                recognition.as_mut(),
                &cancellation,
                None,
            );
            if action == "stale" {
                assert!(outcome.is_err());
                assert!(!cancellation.is_cancelled());
            } else {
                outcome.unwrap();
                let snapshot = host.execution_snapshot(&job.job_id).unwrap();
                assert_eq!(snapshot.status, "cancelled");
                assert!(snapshot.result.is_none() && snapshot.recovery.is_none());
                assert_eq!(
                    called.load(Ordering::Acquire),
                    action == "cancel_during_validation"
                );
            }
            assert_eq!(album_inspection_tree(owned.path()), before);
            drop(guard);
            assert!(host.claim().is_ok());
        }
    }

    type AlbumInspectionEntry = (
        PathBuf,
        bool,
        u64,
        std::time::SystemTime,
        bool,
        Vec<u8>,
        Option<Vec<u8>>,
    );
    fn album_inspection_tree(root: &Path) -> Option<Vec<AlbumInspectionEntry>> {
        fn visit(root: &Path, path: &Path, output: &mut Vec<AlbumInspectionEntry>) {
            let data = std::fs::symlink_metadata(path).unwrap();
            assert!(!data.file_type().is_symlink());
            let auxiliary = PathBuf::from(format!("{}:notes", path.display()));
            output.push((
                path.strip_prefix(root).unwrap().to_owned(),
                data.is_dir(),
                data.len(),
                data.modified().unwrap(),
                data.permissions().readonly(),
                if data.is_file() {
                    std::fs::read(path).unwrap()
                } else {
                    vec![]
                },
                if data.is_file() && auxiliary.exists() {
                    Some(std::fs::read(auxiliary).unwrap())
                } else {
                    None
                },
            ));
            if data.is_dir() {
                let mut children = std::fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect::<Vec<_>>();
                children.sort();
                for child in children {
                    visit(root, &child, output);
                }
            }
        }
        if !root.exists() {
            return None;
        }
        let mut entries = vec![];
        visit(root, root, &mut entries);
        Some(entries)
    }

    #[test]
    fn cross_volume_album_host_retains_streams_journal_and_cancellable_verification() {
        use std::{
            fs,
            sync::{
                Arc, Mutex,
                atomic::{AtomicBool, Ordering},
            },
        };
        for action in [
            "apply",
            "stale_stream",
            "stale_journal",
            "cancel_verification",
        ] {
            let local = tempfile::tempdir().unwrap();
            let other = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
            let source = local.path().canonicalize().unwrap().join("source");
            let journal = local.path().canonicalize().unwrap().join("journal");
            let target = other.path().canonicalize().unwrap().join("target");
            for path in [&source, &journal, &target] {
                fs::create_dir(path).unwrap();
            }
            for track in 1..=3 {
                fs::write(source.join(format!("{track}-שיר.mp3")), audio(track)).unwrap();
            }
            fs::write(source.join("cover.jpg"), b"owned sidecar").unwrap();
            fs::create_dir_all(source.join("nested/deep")).unwrap();
            fs::write(source.join("nested/deep/readme.txt"), b"owned nested file").unwrap();
            let relative_files = [
                "1-שיר.mp3",
                "2-שיר.mp3",
                "3-שיר.mp3",
                "cover.jpg",
                "nested/deep/readme.txt",
            ];
            for name in relative_files {
                fs::write(source.join(format!("{name}:notes")), b"owned auxiliary").unwrap();
            }
            match (
                inspect_root(&source).unwrap().identity,
                inspect_root(&target).unwrap().identity,
            ) {
                (
                    Some(sorter_core::discovery::ObjectIdentity::Windows { volume: a, .. }),
                    Some(sorter_core::discovery::ObjectIdentity::Windows { volume: b, .. }),
                ) => assert_ne!(a, b, "Two real volumes required"),
                _ => panic!("Actual Windows identities required"),
            }
            let mut host = Host::default();
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
            let request = serde_json::from_value(json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
                "settings":{"copy_mode":false,"main_folder_only":false,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
            let job = host
                .preview_with_personal(request, Some(personal.clone()))
                .unwrap();
            let preview = host.snapshot(&job.job_id).unwrap();
            assert!(
                preview
                    .apply
                    .as_ref()
                    .is_some_and(|ready| ready.supported && ready.notice.is_some()),
                "{preview:?}"
            );
            assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
            assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
            let before_verification = Arc::new(Mutex::new(None));
            let cancelled = Arc::new(AtomicBool::new(false));
            if action == "stale_stream" {
                let file = source.join("1-שיר.mp3");
                let modified = fs::metadata(&file).unwrap().modified().unwrap();
                fs::write(source.join("1-שיר.mp3:notes"), b"changed stream!").unwrap();
                fs::File::options()
                    .write(true)
                    .open(file)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_modified(modified))
                    .unwrap();
            } else if action == "stale_journal" {
                fs::rename(&journal, local.path().join("old-journal")).unwrap();
                fs::create_dir(&journal).unwrap();
            } else if action == "cancel_verification" {
                let before = before_verification.clone();
                let flag = cancelled.clone();
                let paths = [source.clone(), target.clone(), journal.clone()];
                let cancel_host = host.clone();
                let cancel_id = job.job_id.clone();
                host.verification_observer = Some(Arc::new(move |progress| {
                    if progress.stage == "album_journals" && progress.bytes_read == 0 {
                        *before.lock().unwrap() = Some(
                            paths
                                .iter()
                                .map(|path| album_inspection_tree(path))
                                .collect::<Vec<_>>(),
                        );
                    }
                    if progress.bytes_read > 0 && !flag.swap(true, Ordering::AcqRel) {
                        cancel_host.cancel_apply(&cancel_id).unwrap();
                    }
                }));
            }
            let source_before = album_inspection_tree(&source);
            host.start_apply(
                &job.job_id,
                &preview.plan_id.unwrap(),
                personal,
                host.claim().unwrap(),
            )
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            let outcome = loop {
                let value = host.execution_snapshot(&job.job_id).unwrap();
                if !value.is_running() {
                    break value;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "Cross-album host timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            if action.starts_with("stale_") {
                assert_eq!(outcome.status, "failed", "{outcome:?}");
                assert!(outcome.result.is_none());
                assert_eq!(album_inspection_tree(&source), source_before);
                assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
                assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
            } else {
                assert_eq!(outcome.status, "complete", "{outcome:?}");
                assert!(!source.exists());
                let destination = target.join("שם ממופה/אלבום");
                for name in relative_files {
                    assert_eq!(
                        fs::read(destination.join(format!("{name}:notes"))).unwrap(),
                        b"owned auxiliary"
                    );
                }
                let result = outcome.result.as_ref().unwrap();
                assert_eq!(
                    (
                        result.source_items,
                        result.committed_destinations,
                        result.moved_sources
                    ),
                    (5, 5, 5)
                );
                assert_eq!(result.albums.as_ref().unwrap().completed, 1);
                assert!(result.items.iter().all(|item| item.status == "complete"));
                assert_eq!(
                    serde_json::to_value(result.legacy_summary.as_ref().unwrap()).unwrap(),
                    json!({"songs_sorted":0,"artist_folders_created":0,"albums_processed":1,"top_artists":[["שלום עולם",4]]})
                );
                if action == "cancel_verification" {
                    assert!(cancelled.load(Ordering::Acquire));
                    assert!(outcome.recovery.is_none());
                    assert_eq!(
                        outcome.recovery_error.as_deref(),
                        Some("VERIFICATION_CANCELLED")
                    );
                    assert_eq!(
                        *before_verification.lock().unwrap(),
                        Some(
                            [&source, &target, &journal]
                                .into_iter()
                                .map(|path| album_inspection_tree(path))
                                .collect()
                        )
                    );
                } else {
                    assert!(outcome.recovery_error.is_none(), "{outcome:?}");
                }
            }
            loop {
                if let Ok(_guard) = host.claim() {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "Host Busy ownership retained"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
