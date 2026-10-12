//! Source-only FixNames host projection. Only retained in-process repair evidence
//! can reach the native mutation driver; renderer fields never grant authority.
use super::*;
use sorter_executor::{
    windows_copy::WindowsPlanningStorage,
    windows_repair::{
        RepairJobStatus, RepairResult, RepairScope, RepairStatus, SelectedWindowsRepairJob,
    },
};
use sorter_io::desktop::windows_mutation::{WindowsMutationStorage, WindowsSelectedRootGrant};

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
                                    .map(|evidence| format!("{}:{:?}", evidence.role, evidence.status))
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
        items: result
            .items
            .into_iter()
            .map(|item| ExecutionItem {
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
                }),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sorter_executor::windows_repair::RepairItem;

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
        let summary = projected.repair.unwrap();
        assert_eq!(summary.renamed_files, 1);
        assert_eq!(summary.repaired_files, 0);
        assert_eq!(summary.unsupported_files, 1);
    }
}
