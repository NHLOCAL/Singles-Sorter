//! Technical CLI projection of observed album effects. No deserialized authority.
use super::*;
use sorter_executor::{
    results::{ItemStatus, JobResult, JobStatus},
    windows_album::{SelectedAlbumRecovery, SelectedWindowsAlbumJob},
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

fn status(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Complete => "complete",
        JobStatus::Partial => "partial",
        JobStatus::Cancelled => "cancelled",
        JobStatus::NeedsRecovery => "needs_recovery",
    }
}
fn transfers(result: &JobResult) -> Value {
    json!({"status":status(result.status),"source_items":result.source_items,
        "committed_destinations":result.committed_destinations,"committed_operations":result.committed_operations,"effect_receipts":result.effect_receipts,"moved_sources":result.moved_sources,
        "failed_operations":result.failed_operations,"blocked_operations":result.blocked_operations,
        "recovery_operations":result.recovery_operations,"cancelled_operations":result.cancelled_operations,
        "items":result.items.iter().map(|item|json!({"operation":item.operation,
            "status":match item.status {ItemStatus::Complete=>"complete",ItemStatus::SourceRetained=>"source_retained",ItemStatus::Failed=>"failed",ItemStatus::Cancelled=>"cancelled",ItemStatus::Blocked=>"blocked",ItemStatus::NeedsRecovery=>"needs_recovery"},
            "detail":item.detail,"observations":item.observations})).collect::<Vec<_>>()})
}
fn recovery(report: SelectedAlbumRecovery) -> Value {
    json!({"manifest_finished":report.manifest_finished,"recorded_status":report.recorded_status,
        "completed_albums":report.completed_albums,"removed_directory_references":report.removed_directory_references,"incomplete_tail":report.incomplete_tail,
        "pending_directories":report.pending_directories,"pending_cleanup":report.pending_cleanup,
        "pending_metadata":report.pending_metadata,"directory_metadata":report.directory_metadata,
        "directory_metadata_status":report.directory_metadata_status,
        "directories":report.directories.iter().map(|entry|json!({"reference":entry.reference,"expected_identity":entry.expected_identity,"observed_identity":entry.observed_identity,"status":format!("{:?}",entry.status)})).collect::<Vec<_>>(),
        "transfers":report.transfers.map(|transfers|json!({"append_blocked":transfers.append_blocked,"operations":transfers.operations.iter().map(|operation|json!({"operation":operation.operation,"status":format!("{:?}",operation.status),"copy_metadata":format!("{:?}",operation.copy_metadata)})).collect::<Vec<_>>()})),
        "whole":report.whole.map(|whole|json!({"recorded_finished":whole.recorded_finished,"incomplete_tail":whole.incomplete_tail,
            "recorded":whole.recorded,"pending_albums":whole.pending_albums,"pending_directories":whole.pending_directories,
            "child_renames":whole.child_renames,
            "child_locations":whole.child_locations.iter().map(|entry|json!({"operation":entry.operation,"planner_index":entry.planner_index,"at_destination":entry.at_destination,"observed":entry.observed,"error":entry.error})).collect::<Vec<_>>(),
            "source_evidence":whole.source_evidence.iter().map(|entry|json!({"reference":entry.reference,"expected_identity":entry.expected_identity,"observed_identity":entry.observed_identity,"status":format!("{:?}",entry.status)})).collect::<Vec<_>>(),
            "destination_evidence":whole.destination_evidence.iter().map(|entry|json!({"reference":entry.reference,"expected_identity":entry.expected_identity,"observed_identity":entry.observed_identity,"status":format!("{:?}",entry.status)})).collect::<Vec<_>>(),
            "files":whole.files.iter().map(|entry|json!({"operation":entry.operation,"reference":entry.reference,"expected_identity":entry.expected_identity,"observed_identity":entry.observed_identity,"status":entry.status})).collect::<Vec<_>>(),
            "singles":whole.singles.map(|singles|recovery(*singles))}))})
}
fn reconciliation_complete(report: &SelectedAlbumRecovery) -> bool {
    use sorter_executor::{
        recovery::{CopyMetadataRecoveryStatus, RecoveryStatus},
        windows_album::AlbumDirectoryEvidenceStatus,
    };
    let removed = report
        .removed_directory_references
        .iter()
        .map(|(reference, identity)| (reference, identity))
        .collect::<std::collections::HashSet<_>>();
    report.manifest_finished
        && report
            .recorded_status
            .as_deref()
            .is_none_or(|status| status == "Complete")
        && !report.incomplete_tail
        && report.pending_directories.is_empty()
        && report.pending_cleanup.is_empty()
        && report.pending_metadata.is_empty()
        && report
            .directory_metadata_status
            .iter()
            .all(|(_, status)| status == "verified")
        && report.directories.iter().all(|entry| {
            if removed.contains(&(&entry.reference, &entry.expected_identity)) {
                matches!(entry.status, AlbumDirectoryEvidenceStatus::Absent)
            } else {
                matches!(entry.status, AlbumDirectoryEvidenceStatus::SameIdentity)
            }
        })
        && report.transfers.as_ref().is_none_or(|transfers| {
            !transfers.append_blocked
                && transfers.operations.iter().all(|operation| {
                    operation.status == RecoveryStatus::Committed
                        && matches!(
                            operation.copy_metadata,
                            CopyMetadataRecoveryStatus::Verified
                                | CopyMetadataRecoveryStatus::NotRequested
                        )
                })
        })
        && report.whole.as_ref().is_none_or(|whole| {
            whole.recorded_finished
                && !whole.incomplete_tail
                && whole.pending_albums.is_empty()
                && whole.pending_directories.is_empty()
                && whole
                    .source_evidence
                    .iter()
                    .all(|entry| matches!(entry.status, AlbumDirectoryEvidenceStatus::Absent))
                && whole
                    .destination_evidence
                    .iter()
                    .all(|entry| matches!(entry.status, AlbumDirectoryEvidenceStatus::SameIdentity))
                && whole.files.iter().all(|entry| entry.status == "verified")
                && whole
                    .child_locations
                    .iter()
                    .all(|entry| entry.error.is_none())
                && whole
                    .singles
                    .as_ref()
                    .is_none_or(|singles| reconciliation_complete(singles))
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute(
    job: &mut SelectedWindowsAlbumJob,
    plan: &sorter_core::job_plan::JobPlanV2,
    directory: &Path,
    source_items: usize,
    cancel: &AtomicBool,
    invalid_control: &AtomicBool,
    event: &mut dyn FnMut(&str, Value) -> Result<(), Failure>,
    copied_album: bool,
) -> Result<(), Failure> {
    let id = job.job_id().to_owned();
    event(
        "started",
        json!({"job_id":id,"plan_id":plan.binding_sha256,"mode":"Sort","album_execution":true,"journal_directory":directory}),
    )?;
    let mut output_failed = false;
    let result = job.run(cancel, &mut |progress| {
        if event("progress",json!({"job_id":id,"operation":progress.operation,"bytes_written":progress.bytes_written,"source_length":progress.source_length})).is_err() {
            output_failed = true;
            cancel.store(true,Ordering::Release);
        }
    });
    let (inspection, recovery_error) = if job.journal_locations().is_empty() {
        (None, None)
    } else if cancel.load(Ordering::Acquire) {
        (None, Some("VERIFICATION_CANCELLED".to_owned()))
    } else {
        let inspected = if copied_album {
            let control =
                sorter_executor::inspection_control::InspectionControl::borrowed(cancel, |_| {});
            if event("verification_started", json!({"job_id":id})).is_err() {
                output_failed = true;
                cancel.store(true, Ordering::Release);
            }
            let inspected = job.inspect_file_recovery_controlled(&control);
            if let Ok(progress) = control.snapshot()
                && event(
                    "verification_progress",
                    json!({"job_id":id,"progress":progress}),
                )
                .is_err()
            {
                output_failed = true;
                cancel.store(true, Ordering::Release);
            }
            inspected
        } else {
            job.inspect_recovery(cancel)
        };
        match inspected {
            Ok(report) => {
                let verified = reconciliation_complete(&report);
                (
                    Some(recovery(report)),
                    (!verified).then(|| "ALBUM_RECONCILIATION_INCOMPLETE".to_owned()),
                )
            }
            Err(error) => (None, Some(error.to_string())),
        }
    };
    let verification_failed = recovery_error.is_some();
    match result {
        Ok(result) => {
            let actual = status(result.status);
            let committed_operations = result
                .transfers
                .as_ref()
                .map(|transfers| transfers.committed_operations.as_slice())
                .unwrap_or_default();
            let legacy_summary = sorter_executor::legacy_summary::LegacySummary::from_receipts(
                &plan.operations,
                committed_operations,
                &result
                    .created_directories
                    .iter()
                    .map(|entry| entry.reference.clone())
                    .collect::<Vec<_>>(),
                result.albums_completed,
                &result.artist_files,
            );
            // Whole directory receipts and per-file transfer receipts are disjoint.
            let whole_files = result
                .whole_albums
                .iter()
                .filter(|album| album.status == "complete")
                .map(|album| {
                    album
                        .descendants
                        .iter()
                        .filter(|(_, observed)| observed.size.is_some())
                        .count()
                })
                .sum::<usize>();
            event(
                "result",
                json!({"job_id":id,"status":actual,"execution_decisions":job.execution_decisions(),"source_items":source_items,
                "committed_destinations":whole_files+result.transfers.as_ref().map_or(0,|r|r.committed_destinations),
                "moved_sources":whole_files+result.transfers.as_ref().map_or(0,|r|r.moved_sources),
                "albums_completed":result.albums_completed,"completed_album_operations":result.completed_album_operations,"artist_files":result.artist_files,
                "legacy_summary":legacy_summary,"committed_operations":committed_operations,"effect_receipts":result.effect_receipts,
                "transfers":result.transfers.as_ref().map(transfers),"whole_albums":result.whole_albums,
                "child_renames":result.child_renames,"created_directories":result.created_directories,
                "removed_directories":result.removed_directories,"retained_directories":result.retained_directories,
                "detail":result.detail,"recovery":inspection,"recovery_error":recovery_error,
                "journal_directory":directory,"invalid_control":invalid_control.load(Ordering::Acquire)}),
            )?;
            if output_failed
                || invalid_control.load(Ordering::Acquire)
                || actual != "complete"
                || verification_failed
            {
                return Err(Failure::new(
                    1,
                    "EXECUTION_INCOMPLETE",
                    "Inspect actual album effects and retained journals before retrying",
                ));
            }
            Ok(())
        }
        Err(error) => {
            event(
                "result",
                json!({"job_id":id,"status":if job.journal_locations().is_empty()&&cancel.load(Ordering::Acquire){"cancelled"}else{"needs_recovery"},
                "error":error.to_string(),"recovery":inspection,"recovery_error":recovery_error,"journal_directory":directory}),
            )?;
            Err(Failure::new(
                1,
                "EXECUTION_INCOMPLETE",
                "Inspect the album journal; no automatic replay or cleanup was attempted",
            ))
        }
    }
}
