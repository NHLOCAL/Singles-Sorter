//! Source-only FixNames host projection. Only retained in-process repair evidence
//! can reach the native mutation driver; renderer fields never grant authority.
use super::*;
use sorter_executor::{
    windows_copy::WindowsPlanningStorage,
    windows_repair::{
        RepairItem, RepairJobStatus, RepairResult, RepairScope, RepairStatus, SelectedWindowsRepairJob,
    },
};
use sorter_io::{
    desktop::windows_mutation::{WindowsMutationStorage, WindowsSelectedRootGrant},
    mutation::MutationName,
};

impl Host {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_repair(
        &self,
        id: &str,
        plan: &JobPlanV2,
        source: &Path,
        directory: PathBuf,
        scope: &RepairScope,
        recognition: &mut dyn RecognitionProvider,
        cancellation: &Cancellation,
    ) -> Result<()> {
        let journal = inspect_root(&directory)?;
        let roots = WindowsMutationStorage::for_selected_repair_roots(
            WindowsSelectedRootGrant::new(source.to_owned(), scope.source_root().clone())
                .map_err(storage_error)?,
            WindowsSelectedRootGrant::new(
                directory.clone(),
                journal.identity.ok_or_else(|| {
                    HostError::new("JOURNAL_UNAVAILABLE", "Journal root identity missing")
                })?,
            )
            .map_err(storage_error)?,
        )
        .map_err(storage_error)?;
        let mut reads =
            WindowsPlanningStorage::for_repair_preview(source).map_err(storage_error)?;
        let mut metadata = StorageMetadataProvider {
            storage: &mut reads,
            reader: MetadataReader,
        };
        let mut job = SelectedWindowsRepairJob::prepare(
            roots,
            plan,
            &mut metadata,
            recognition,
            scope.resources(),
            scope,
            &cancellation.cancelled,
        )
        .map_err(storage_error)?;
        self.update_repair(id, |snapshot| {
            snapshot.status = "running";
            snapshot.journal_path = Some(directory.to_string_lossy().into_owned());
        });
        let result = job.run(&cancellation.cancelled, &mut |progress| {
            self.update_repair(id, |snapshot| {
                snapshot.operation = Some(progress.operation);
            });
        });
        if !job.journal_locations().is_empty() && !cancellation.is_cancelled() {
            match job.inspect_recovery(&cancellation.cancelled) {
                Ok(report) => self.update_repair(id, |snapshot| {
                    snapshot.recovery = Some(RecoveryInspection {
                        pending_operations: vec![],
                        directory_evidence: vec![],
                        metadata_issues: None,
                        terminal_records_complete: None,
                        append_blocked: None,
                        observed_directories: 0,
                        pending_directories: 0,
                        rejected_directories: 0,
                        incomplete_tail: report.incomplete_tail,
                        transfers: report
                            .operations
                            .into_iter()
                            .map(|operation| RecoveryItem {
                                operation: operation.operation,
                                status: operation
                                    .evidence
                                    .iter()
                                    .map(|evidence| {
                                        format!("{}:{:?}", evidence.role, evidence.status)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            })
                            .collect(),
                    });
                }),
                Err(error) => self.update_repair(id, |snapshot| {
                    // This reports failed reconciliation, not a fictional failed
                    // mutation. Observed repair counters remain unchanged.
                    snapshot.recovery_error = Some(error.to_string());
                }),
            }
        }
        match result {
            Ok(result) => self.update_repair(id, |snapshot| {
                snapshot.status = match result.status {
                    RepairJobStatus::Complete => "complete",
                    RepairJobStatus::Partial => "partial",
                    RepairJobStatus::Cancelled => "cancelled",
                    RepairJobStatus::NeedsRecovery => "needs_recovery",
                };
                snapshot.result = Some(project(result));
                snapshot.operation = None;
            }),
            Err(error) => self.execution_error(
                id,
                if job.journal_locations().is_empty() && cancellation.is_cancelled() {
                    "cancelled"
                } else {
                    "needs_recovery"
                },
                &format!(
                    "תיקון הקבצים נעצר: {error}. יש לבדוק את הקבצים והגיבויים לפני ניסיון נוסף."
                ),
            ),
        }
        Ok(())
    }
    fn update_repair(&self, id: &str, update: impl FnOnce(&mut ExecutionSnapshot)) {
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

fn project(result: RepairResult) -> ExecutionResult {
    let summary = RepairSummary {
        total_files: result.total_files,
        processed_files: result.processed_files,
        renamed_files: result.renamed_files,
        repaired_files: result.repaired_files,
        unchanged_files: result.unchanged_files,
        unsupported_files: result.unsupported_files,
        failed_files: result.failed_files,
        retained_backup_bytes: result.retained_backup_bytes,
    };
    ExecutionResult {
        source_items: result.total_files,
        // FixNames neither sorts destinations nor moves a source out of its tree.
        committed_destinations: 0,
        moved_sources: 0,
        failed_operations: result.failed_files,
        blocked_operations: result.total_files.saturating_sub(result.items.len()),
        recovery_operations: result
            .items
            .iter()
            .filter(|item| item.status == RepairStatus::NeedsRecovery)
            .count(),
        cancelled_operations: result
            .items
            .iter()
            .filter(|item| item.status == RepairStatus::Cancelled)
            .count(),
        repair: Some(summary),
        albums: None,
        items: result
            .items
            .into_iter()
            .map(|item| {
                let observed_filename = observed_filename(&item);
                ExecutionItem {
                    operation: item.operation,
                    status: match item.status {
                        RepairStatus::Complete => "complete",
                        RepairStatus::UnsupportedTags => "unsupported_tags",
                        RepairStatus::MetadataFailure => "metadata_failure",
                        RepairStatus::Failed => "failed",
                        RepairStatus::Cancelled => "cancelled",
                        RepairStatus::NeedsRecovery => "needs_recovery",
                    },
                    detail: item.detail,
                    repair: Some(RepairOutcome {
                        renamed: item.renamed,
                        repaired: item.repaired,
                        unchanged: item.unchanged,
                        observed_filename,
                    }),
                }
            })
            .collect(),
    }
}

// A current observation alone is insufficient: it can precede publication
// verification, or survive a later failed/unknown mutation. These terminal paths
// either verified publication/retained source, or stopped before any tag staging
// after a verified rename. Job-level partial/cancelled status does not erase
// completed per-file evidence. Never use the intended, backup or staged name.
fn observed_filename(item: &RepairItem) -> Option<String> {
    if !matches!(item.status, RepairStatus::Complete)
        && !(item.renamed
            && matches!(
                item.status,
                RepairStatus::UnsupportedTags | RepairStatus::MetadataFailure
            ))
    {
        return None;
    }
    let current = item.current.as_ref()?;
    if [item.backup.as_ref(), item.staged.as_ref()]
        .into_iter()
        .flatten()
        .any(|other| other.identity == current.identity || other.reference == current.reference)
    {
        return None;
    }
    let MutationName::Windows(units) = &current.actual_name else {
        return None;
    };
    let StorageRef::WindowsPath { utf16 } = &current.reference else {
        return None;
    };
    if utf16.rsplit(|unit| *unit == 47 || *unit == 92).next()? != units.as_slice() {
        return None;
    }
    let name = String::from_utf16(units).ok()?;
    if name.is_empty()
        || matches!(name.as_str(), "." | "..")
        || name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
        || name.to_ascii_lowercase().ends_with(".backup")
        || name.to_ascii_lowercase().ends_with(".tmp")
    {
        return None;
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sorter_core::discovery::ObjectIdentity;
    use sorter_io::mutation::MutationObservation;

    fn observation(name: &str, identity: u8) -> MutationObservation {
        MutationObservation {
            reference: StorageRef::WindowsPath {
                utf16: format!("C:\\owned\\{name}").encode_utf16().collect(),
            },
            identity: ObjectIdentity::Windows {
                volume: 1,
                file_id: [identity; 16],
            },
            parent_identity: ObjectIdentity::Windows {
                volume: 1,
                file_id: [0; 16],
            },
            actual_name: MutationName::Windows(name.encode_utf16().collect()),
            size: Some(128),
        }
    }

    fn repaired(status: RepairStatus) -> RepairItem {
        RepairItem {
            operation: 0,
            original: observation("old_name.mp3", 1).reference,
            status,
            detail: "observed result".into(),
            renamed: true,
            repaired: true,
            unchanged: false,
            processed: true,
            current: Some(observation("שם מתוקן.mp3", 2)),
            backup: Some(observation(".sorter-owned-0.backup", 1)),
            staged: None,
        }
    }

    #[test]
    fn current_filename_comes_from_the_observation_not_the_original_or_backup() {
        let item = repaired(RepairStatus::Complete);
        assert_eq!(observed_filename(&item).as_deref(), Some("שם מתוקן.mp3"));
        let mut renamed_only = repaired(RepairStatus::UnsupportedTags);
        renamed_only.backup = None;
        renamed_only.repaired = false;
        assert_eq!(
            observed_filename(&renamed_only).as_deref(),
            Some("שם מתוקן.mp3")
        );
        renamed_only.status = RepairStatus::MetadataFailure;
        assert_eq!(
            observed_filename(&renamed_only).as_deref(),
            Some("שם מתוקן.mp3")
        );
        renamed_only.renamed = false;
        assert_eq!(observed_filename(&renamed_only), None);
    }

    #[test]
    fn uncertain_or_missing_current_evidence_never_projects_a_repaired_name() {
        for status in [
            RepairStatus::Failed,
            RepairStatus::Cancelled,
            RepairStatus::NeedsRecovery,
        ] {
            assert_eq!(observed_filename(&repaired(status)), None);
        }
        let mut item = repaired(RepairStatus::Complete);
        item.current = None;
        assert_eq!(observed_filename(&item), None);
        assert!(item.backup.is_some());
        for name in [
            ".sorter-owned-0.backup",
            ".sorter-owned-0.tmp",
            "",
            ".",
            "..",
            "a/b.mp3",
            "a\\b.mp3",
            "a\0.mp3",
        ] {
            item.current = Some(observation(name, 2));
            assert_eq!(observed_filename(&item), None);
        }
        item.current = Some(observation("filename.mp3", 2));
        item.current.as_mut().unwrap().actual_name = MutationName::Windows(vec![0xd800]);
        assert_eq!(observed_filename(&item), None);
        item.current.as_mut().unwrap().actual_name = MutationName::Provider("filename.mp3".into());
        assert_eq!(observed_filename(&item), None);
    }

    #[test]
    fn backup_staging_and_mismatched_reference_cannot_be_presented_as_current() {
        let mut item = repaired(RepairStatus::Complete);
        item.backup = item.current.clone();
        assert_eq!(observed_filename(&item), None);
        item.backup = None;
        item.staged = item.current.clone();
        assert_eq!(observed_filename(&item), None);
        item.staged = None;
        item.current.as_mut().unwrap().reference = observation("different.mp3", 2).reference;
        assert_eq!(observed_filename(&item), None);
    }

    #[test]
    fn cancelled_job_keeps_completed_file_name_without_changing_item_status_or_counters() {
        let projected = project(RepairResult {
            status: RepairJobStatus::Cancelled,
            total_files: 2,
            processed_files: 1,
            renamed_files: 1,
            repaired_files: 1,
            unchanged_files: 0,
            unsupported_files: 0,
            failed_files: 0,
            retained_backup_bytes: 128,
            items: vec![repaired(RepairStatus::Complete)],
        });
        assert_eq!(projected.blocked_operations, 1);
        assert_eq!(projected.items[0].status, "complete");
        assert_eq!(
            projected.items[0]
                .repair
                .as_ref()
                .unwrap()
                .observed_filename
                .as_deref(),
            Some("שם מתוקן.mp3")
        );
        let wire = serde_json::to_value(&projected).unwrap();
        assert_eq!(wire["items"][0]["repair"]["observed_filename"], "שם מתוקן.mp3");
        assert_eq!(projected.repair.unwrap().renamed_files, 1);
    }

    #[test]
    fn projection_keeps_partial_filename_and_tag_outcomes_separate() {
        let projected = project(RepairResult {
            status: RepairJobStatus::Partial,
            total_files: 3,
            processed_files: 1,
            renamed_files: 1,
            repaired_files: 0,
            unchanged_files: 0,
            unsupported_files: 1,
            failed_files: 0,
            retained_backup_bytes: 0,
            items: vec![RepairItem {
                operation: 0,
                original: StorageRef::WindowsPath {
                    utf16: vec![67, 58, 92, 120],
                },
                status: RepairStatus::UnsupportedTags,
                detail: "unsupported representation".into(),
                renamed: true,
                repaired: false,
                unchanged: false,
                processed: true,
                current: None,
                backup: None,
                staged: None,
            }],
        });
        assert_eq!(projected.committed_destinations, 0);
        assert_eq!(projected.moved_sources, 0);
        assert_eq!(projected.blocked_operations, 2);
        assert_eq!(projected.items[0].status, "unsupported_tags");
        assert!(projected.items[0].repair.as_ref().unwrap().renamed);
        assert!(!projected.items[0].repair.as_ref().unwrap().repaired);
        assert!(
            projected.items[0]
                .repair
                .as_ref()
                .unwrap()
                .observed_filename
                .is_none()
        );
        let wire = serde_json::to_value(&projected).unwrap();
        assert!(wire["items"][0]["repair"].get("observed_filename").is_none());
        let summary = projected.repair.unwrap();
        assert_eq!(summary.renamed_files, 1);
        assert_eq!(summary.repaired_files, 0);
        assert_eq!(summary.unsupported_files, 1);
    }
}
