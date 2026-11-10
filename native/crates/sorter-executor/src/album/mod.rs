//! Synthetic-only exclusive directory creation and one flat album Copy.
//! The unchanged core planner still describes every original album rule. This
//! executor rejects unsupported plans; it never changes their policy or counters.
mod binding;
mod journal;
mod recovery;
use crate::{ExecutionError as E, Result, physical_journal::*, results::*, transfer::*};
pub use binding::{AlbumBinding, DirectoryRequirement, validate_album_copy};
pub use journal::{AlbumEvent, AlbumJournal};
pub use recovery::*;
use sorter_storage_contracts::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Read-only preflight must compare the complete ordered child set, every child's
/// identity/content, every existing ancestor, every absent directory slot and all
/// final names before any directory is created. Keep this non-forgeable lease and
/// relevant source/ancestor guards alive for the entire run. Source membership,
/// order and child content cannot change while held. Checking repeatedly without
/// this lease is NOT sufficient; unsupported adapters must fail closed.
/// No production adapter implements this trait.
pub trait AlbumStorage: TransferStorage + OwnedDirectoryStorage {
    type AlbumLease;
    fn acquire_album(
        &mut self,
        binding: &AlbumBinding,
    ) -> std::result::Result<Self::AlbumLease, MutationError>;
    /// Return a guard for an existing required parent, from the held lease only.
    fn existing_album_parent(
        &mut self,
        lease: &Self::AlbumLease,
        index: usize,
    ) -> std::result::Result<Self::Parent, MutationError>;
    /// Verify the lease remains valid (including capability revocation). This is
    /// an additional check, never permission to reopen/reacquire changed objects.
    fn check_album(
        &mut self,
        lease: &Self::AlbumLease,
        binding: &AlbumBinding,
    ) -> std::result::Result<(), MutationError>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlbumResult {
    pub status: JobStatus,
    pub transfers: Option<JobResult>,
    pub directories: Vec<MutationObservation>,
    pub error: Option<MutationError>,
}
fn fault(kind: MutationErrorKind, detail: &str) -> MutationError {
    MutationError::new(kind, detail)
}
fn cancelled(flag: &AtomicBool) -> std::result::Result<(), MutationError> {
    if flag.load(Ordering::Acquire) {
        Err(fault(
            MutationErrorKind::Cancelled,
            "cancelled before album boundary",
        ))
    } else {
        Ok(())
    }
}
fn capabilities<S: AlbumStorage>(storage: &S) -> std::result::Result<(), MutationError> {
    let c = storage.directory_capabilities();
    if !c.exclusive_owned_create.is_supported()
        || !c.held_ancestry.is_supported()
        || !c.stable_child_set.is_supported()
        || !c.durable_directory_sync.is_supported()
    {
        return Err(fault(
            MutationErrorKind::Unsupported,
            "directory or stable membership guarantee unproven",
        ));
    }
    let f = storage.capabilities();
    if !f.exclusive_create.is_supported()
        || !f.no_replace_publish.is_supported()
        || !f.content_verification.is_supported()
    {
        return Err(fault(
            MutationErrorKind::Unsupported,
            "file transfer capability unproven",
        ));
    }
    Ok(())
}
fn stop(
    journal: &mut AlbumJournal,
    sink: &mut dyn JournalStore,
    result: &mut AlbumResult,
    error: MutationError,
    ambiguous: bool,
) {
    result.status = if ambiguous {
        JobStatus::NeedsRecovery
    } else if error.kind == MutationErrorKind::Cancelled {
        JobStatus::Cancelled
    } else {
        JobStatus::Partial
    };
    if let Some(o) = error.observed.as_deref() {
        result.directories.push(o.clone());
    }
    let event = AlbumEvent::Stopped {
        detail: error.to_string(),
        observed: error.observed.as_deref().cloned(),
    };
    result.error = Some(error);
    if journal.append(sink, event).is_err() {
        result.status = JobStatus::NeedsRecovery;
    }
}
/// Fresh-run only. No cleanup, adoption, overwrite, source retirement or replay.
/// Directory effects must be durable before the existing file-copy driver runs.
pub fn run_album_copy<S: AlbumStorage>(
    binding: &AlbumBinding,
    storage: &mut S,
    journal: &mut AlbumJournal,
    sink: &mut dyn JournalStore,
    flag: &AtomicBool,
    progress: &mut dyn FnMut(TransferProgress),
) -> Result<AlbumResult> {
    if binding.recovery_only || binding.sha256()? != journal.binding.sha256()? {
        return Err(E::Scope);
    }
    if journal.needs_recovery() || !journal.events().is_empty() {
        return Err(E::NeedsRecovery);
    }
    let mut result = AlbumResult {
        status: JobStatus::Partial,
        transfers: None,
        directories: vec![],
        error: None,
    };
    let lease = match cancelled(flag)
        .and_then(|()| capabilities(storage))
        .and_then(|()| storage.acquire_album(binding))
    {
        Ok(lease) => lease,
        Err(e) => {
            stop(journal, sink, &mut result, e, false);
            return Ok(result);
        }
    };
    let mut parents = vec![];
    // Acquire/check every existing parent before any mutation. Missing prefixes
    // are created individually later, never implicitly by path resolution.
    for (index, d) in binding.directories().iter().enumerate() {
        if let Some(expected) = &d.existing {
            let parent = match storage.existing_album_parent(&lease, index) {
                Ok(p) if Some(storage.parent_identity(&p)) == expected.identity => p,
                Ok(_) => {
                    stop(
                        journal,
                        sink,
                        &mut result,
                        fault(
                            MutationErrorKind::StalePlan,
                            "existing parent identity changed",
                        ),
                        false,
                    );
                    return Ok(result);
                }
                Err(e) => {
                    stop(journal, sink, &mut result, e, false);
                    return Ok(result);
                }
            };
            parents.push(Some(parent));
        } else {
            parents.push(None);
        }
    }
    let mut owned = vec![];
    for index in 1..binding.directories().len() {
        if parents[index].is_some() {
            continue;
        }
        if let Err(e) = cancelled(flag)
            .and_then(|()| capabilities(storage))
            .and_then(|()| storage.check_album(&lease, binding))
        {
            stop(journal, sink, &mut result, e, false);
            return Ok(result);
        }
        if journal
            .append(sink, AlbumEvent::DirectoryIntent { index })
            .is_err()
        {
            result.status = JobStatus::NeedsRecovery;
            return Ok(result);
        }
        // Cancellation after intent/sync but before dispatch is still no effect.
        if let Err(e) = cancelled(flag) {
            stop(journal, sink, &mut result, e, false);
            return Ok(result);
        }
        let name = MutationName::Provider(
            binding.directories()[index]
                .components
                .last()
                .unwrap()
                .clone(),
        );
        let directory = match storage.create_owned_directory(
            parents[index - 1].as_ref().ok_or(E::InvalidBinding)?,
            &name,
            flag,
        ) {
            MutationOutcome::Applied(d) => d,
            MutationOutcome::NoEffect(e) => {
                stop(journal, sink, &mut result, e, false);
                return Ok(result);
            }
            MutationOutcome::PendingUnknown(e) => {
                stop(journal, sink, &mut result, e, true);
                return Ok(result);
            }
        };
        let observed = storage.directory_observation(&directory);
        result.directories.push(observed.clone());
        // Journal validation binds identity, parent, exact name and uniqueness.
        if journal
            .append(
                sink,
                AlbumEvent::DirectoryCreated {
                    index,
                    observed: observed.clone(),
                },
            )
            .is_err()
        {
            stop(
                journal,
                sink,
                &mut result,
                fault(
                    MutationErrorKind::UnknownOutcome,
                    "directory result could not be durably bound",
                )
                .with_observation(observed),
                true,
            );
            return Ok(result);
        }
        if let Err(e) = cancelled(flag).and_then(|()| storage.sync_owned_directory(&directory)) {
            stop(journal, sink, &mut result, e, true);
            return Ok(result);
        }
        if journal
            .append(
                sink,
                AlbumEvent::DirectorySynced {
                    index,
                    identity: observed.identity.clone(),
                },
            )
            .is_err()
        {
            result.status = JobStatus::NeedsRecovery;
            return Ok(result);
        }
        let parent = storage.owned_directory_parent(&directory);
        if storage.parent_identity(&parent) != observed.identity {
            stop(
                journal,
                sink,
                &mut result,
                fault(
                    MutationErrorKind::UnknownOutcome,
                    "owned parent guard identity changed",
                ),
                true,
            );
            return Ok(result);
        }
        parents[index] = Some(parent);
        owned.push(directory);
    }
    if let Err(e) = cancelled(flag).and_then(|()| storage.check_album(&lease, binding)) {
        stop(journal, sink, &mut result, e, false);
        return Ok(result);
    }
    let parent = storage.parent_identity(
        parents
            .last()
            .and_then(Option::as_ref)
            .ok_or(E::InvalidBinding)?,
    );
    let transfers = binding.transfers();
    let job_id = journal.job_id.clone();
    let mut child_sink = journal::ChildSink { journal, sink };
    let mut child_journal = match PhysicalJournalV2::create(&transfers, &job_id, &mut child_sink) {
        Ok(j) => j,
        Err(_) => {
            result.status = JobStatus::NeedsRecovery;
            return Ok(result);
        }
    };
    let (child, child_guards) = run_album_transfers(
        &transfers,
        storage,
        &mut child_journal,
        &mut child_sink,
        flag,
        progress,
        &parent,
    )?;
    result.status = child.status;
    result.transfers = Some(child);
    if result.status == JobStatus::Complete {
        if let Err(e) = storage
            .check_album(&lease, binding)
            .and_then(|()| cancelled(flag))
        {
            let ambiguous = e.kind != MutationErrorKind::Cancelled;
            stop(journal, sink, &mut result, e, ambiguous);
        } else if journal.append(sink, AlbumEvent::Complete).is_err() {
            result.status = JobStatus::NeedsRecovery;
        }
    } else if journal
        .append(
            sink,
            AlbumEvent::Stopped {
                detail: "child transfer stopped; preserve all evidence".into(),
                observed: None,
            },
        )
        .is_err()
    {
        result.status = JobStatus::NeedsRecovery;
    }
    // Explicitly retain these guards through child commitment and parent result.
    drop(child_guards);
    drop(owned);
    drop(parents);
    drop(lease);
    Ok(result)
}
