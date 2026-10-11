//! Bounded transfers for validated synthetic or owned-Windows Copy bindings.
//! There is no production-root adapter or shipping apply entry point.
use crate::{ExecutionError as E, Result, binding::*, physical_journal::*, results::*};
use sha2::{Digest, Sha256};
use sorter_core::discovery::{ObjectIdentity, StorageRef};
use sorter_storage_contracts::*;
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::atomic::{AtomicBool, Ordering},
};

pub const TRANSFER_BUFFER_BYTES: usize = 64 * 1024;
/// Synchronous bounded I/O on the same held guards. A method returning must not
/// leave a write capable of completing later; asynchronous provider adapters must
/// settle that operation or expose PendingUnknown at their mutation boundary.
/// Resolution is read-only and stays under the supplied, identity-bound roots.
/// This slice cannot create unjournaled destination directories.
/// Source and published guards must keep their content, identity and relevant
/// ancestors protected until dropped. An adapter unable to hold those guarantees
/// must reject the operation; re-reading alone is not a replacement for a lease.
pub trait TransferStorage: MutationStorage {
    fn source_parent(
        &mut self,
        source: &SourceEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<Self::Parent, MutationError>;
    fn destination_parent(
        &mut self,
        destination: &DestinationEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<Self::Parent, MutationError>;
    fn parent_identity(&self, parent: &Self::Parent) -> ObjectIdentity;
    fn temp_observation(&self, temp: &Self::OwnedTemp) -> MutationObservation;
    fn rewind_source(
        &mut self,
        source: &mut Self::SourceLease,
    ) -> std::result::Result<(), MutationError>;
    fn read_source(
        &mut self,
        source: &mut Self::SourceLease,
        buffer: &mut [u8],
    ) -> io::Result<usize>;
    fn write_temp(&mut self, temp: &mut Self::OwnedTemp, buffer: &[u8]) -> io::Result<usize>;
    /// Flush buffered output and request the adapter's documented file sync.
    /// This cannot promote an unknown durable_flush capability to Supported.
    fn sync_temp(&mut self, temp: &mut Self::OwnedTemp) -> std::result::Result<(), MutationError>;
    fn verify_temp(
        &mut self,
        temp: &Self::OwnedTemp,
        digest: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError>;
    /// Positive, lease-derived same-volume evidence. False uses the copy path.
    fn rename_eligible(&self, source: &Self::SourceLease, parent: &Self::Parent) -> bool;
    /// Narrow contract: a write-through, pre/post-flushed same-volume rename.
    /// Not global durable_flush or copy-and-retire support. Other adapters refuse.
    #[cfg(all(windows, feature = "experimental-windows-rename"))]
    fn owned_windows_rename_contract(&self) -> bool {
        false
    }
    /// Extra checked file-sync step for the private owned-Windows Copy domain.
    /// Other domains never call this hook. An adapter cannot silently substitute
    /// a no-op; the default refuses the operation after publication.
    #[cfg(all(windows, feature = "experimental-windows-copy"))]
    fn sync_owned_windows_publication(
        &mut self,
        _published: &mut Self::Published,
        _digest: &ContentDigest,
        _cancel: &AtomicBool,
    ) -> std::result::Result<MutationObservation, MutationError> {
        Err(error(
            MutationErrorKind::Unsupported,
            "owned Windows publication sync unavailable",
        ))
    }
}
#[derive(Clone, Debug)]
pub struct TransferProgress {
    pub operation: usize,
    pub bytes_written: u64,
    pub source_length: u64,
}

fn error(kind: MutationErrorKind, detail: impl Into<String>) -> MutationError {
    MutationError::new(kind, detail)
}
fn io_failure(e: io::Error) -> MutationError {
    let kind = match e.kind() {
        io::ErrorKind::StorageFull => MutationErrorKind::DiskFull,
        io::ErrorKind::PermissionDenied => MutationErrorKind::PermissionDenied,
        _ => MutationErrorKind::Io,
    };
    error(kind, e.to_string())
}
fn cancelled(flag: &AtomicBool) -> bool {
    flag.load(Ordering::Acquire)
}
fn source_expected(op: &BoundOperation) -> Result<ExpectedSource> {
    let e = &op.source.observation;
    let mut sha256 = [0; 32];
    if op.source.sha256.len() != 64 {
        return Err(E::InvalidBinding);
    }
    for (i, b) in sha256.iter_mut().enumerate() {
        *b = u8::from_str_radix(&op.source.sha256[i * 2..i * 2 + 2], 16)
            .map_err(|_| E::InvalidBinding)?;
    }
    Ok(ExpectedSource {
        identity: e.identity.clone().ok_or(E::InvalidBinding)?,
        parent_identity: e.parent_identity.clone().ok_or(E::InvalidBinding)?,
        name: match &e.reference {
            StorageRef::WindowsPath { utf16 } => {
                let leaf = utf16
                    .rsplit(|u| *u == 92 || *u == 47)
                    .next()
                    .ok_or(E::InvalidBinding)?;
                MutationName::Windows(leaf.to_vec())
            }
            _ => mutation_name(&e.reference, &e.name)?,
        },
        size: e.size,
        modified: e.modified.clone(),
        digest: ContentDigest {
            sha256,
            length: e.size.ok_or(E::InvalidBinding)?,
        },
    })
}
fn record(
    journal: &mut PhysicalJournalV2,
    sink: &mut dyn JournalStore,
    op: &BoundOperation,
    phase: Phase,
    identity: Option<ObjectIdentity>,
) -> Result<()> {
    journal.append(
        sink,
        Event {
            operation: op.id,
            destination_id: op.destination.id.clone(),
            expected_source: op
                .source
                .observation
                .identity
                .clone()
                .ok_or(E::InvalidBinding)?,
            observed_identity: identity,
            sha256: op.source.sha256.clone(),
            phase,
        },
    )
}
fn check_observation(
    observed: &MutationObservation,
    identity: &ObjectIdentity,
    parent: &ObjectIdentity,
    name: Option<&MutationName>,
    length: Option<u64>,
) -> std::result::Result<(), MutationError> {
    if &observed.identity != identity
        || &observed.parent_identity != parent
        || name.is_some_and(|n| n != &observed.actual_name)
        || length.is_some_and(|n| observed.size.is_some_and(|size| size != n))
        || !matches!(
            (&observed.reference, &observed.identity),
            (StorageRef::TestOpaque { .. }, ObjectIdentity::Test { .. })
                | (
                    StorageRef::WindowsPath { .. },
                    ObjectIdentity::Windows { .. }
                )
        )
    {
        return Err(error(
            MutationErrorKind::VerificationFailed,
            "returned identity, parent, name or size differs from held evidence",
        )
        .with_observation(observed.clone()));
    }
    Ok(())
}
/// A complete stream is bounded by the pre-authorized size plus one EOF probe.
/// Short writes are completed, Interrupted is retried after cancellation checks,
/// WriteZero and unexpected EOF/extra bytes fail without publication.
fn stream<S: TransferStorage>(
    storage: &mut S,
    source: &mut S::SourceLease,
    temp: &mut S::OwnedTemp,
    expected: &ContentDigest,
    op: usize,
    flag: &AtomicBool,
    progress: &mut dyn FnMut(TransferProgress),
) -> std::result::Result<(), MutationError> {
    storage.rewind_source(source)?;
    let mut buffer = [0; TRANSFER_BUFFER_BYTES];
    let mut total = 0u64;
    let mut hash = Sha256::new();
    loop {
        if cancelled(flag) {
            return Err(error(
                MutationErrorKind::Cancelled,
                "cancelled before next read",
            ));
        }
        let remaining = expected.length.saturating_sub(total);
        let bound = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len())
            .max(1);
        let n = match storage.read_source(source, &mut buffer[..bound]) {
            Ok(n) if n <= bound => n,
            Ok(_) => {
                return Err(error(
                    MutationErrorKind::Io,
                    "reader returned an invalid count",
                ));
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(io_failure(e)),
        };
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or_else(|| error(MutationErrorKind::VerificationFailed, "size overflow"))?;
        if total > expected.length {
            return Err(error(
                MutationErrorKind::StalePlan,
                "source exceeds authorized length",
            ));
        }
        hash.update(&buffer[..n]);
        let mut offset = 0;
        while offset < n {
            if cancelled(flag) {
                return Err(error(
                    MutationErrorKind::Cancelled,
                    "cancelled before next write",
                ));
            }
            match storage.write_temp(temp, &buffer[offset..n]) {
                Ok(0) => return Err(error(MutationErrorKind::Io, "write returned zero")),
                Ok(w) if w <= n - offset => offset += w,
                Ok(_) => {
                    return Err(error(
                        MutationErrorKind::Io,
                        "writer returned invalid count",
                    ));
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(io_failure(e)),
            }
        }
        progress(TransferProgress {
            operation: op,
            bytes_written: total,
            source_length: expected.length,
        });
    }
    let observed: [u8; 32] = hash.finalize().into();
    if total != expected.length || observed != expected.sha256 {
        return Err(error(
            MutationErrorKind::VerificationFailed,
            "stream differs from authorized source digest",
        ));
    }
    if cancelled(flag) {
        return Err(error(MutationErrorKind::Cancelled, "cancelled before sync"));
    }
    storage.sync_temp(temp)?;
    Ok(())
}

struct Failure {
    error: MutationError,
    pending: bool,
}
impl From<MutationError> for Failure {
    fn from(error: MutationError) -> Self {
        Self {
            pending: error.kind == MutationErrorKind::UnknownOutcome,
            error,
        }
    }
}
impl Failure {
    fn pending(error: MutationError) -> Self {
        Self {
            error,
            pending: true,
        }
    }
}
fn outcome<T>(value: MutationOutcome<T>) -> std::result::Result<T, Failure> {
    match value {
        MutationOutcome::Applied(v) => Ok(v),
        MutationOutcome::NoEffect(e) => Err(e.into()),
        MutationOutcome::PendingUnknown(e) => Err(Failure::pending(e)),
    }
}

/// One fresh run only. Nonempty or recovery journals are never auto-resumed.
/// Source guards and published guards stay held for the full job. A fault stops
/// dependent effects; ambiguous effects or journal errors stop the whole job.
#[allow(clippy::too_many_arguments)]
pub fn run_job<S: TransferStorage>(
    binding: &ExecutionBinding,
    storage: &mut S,
    journal: &mut PhysicalJournalV2,
    sink: &mut dyn JournalStore,
    flag: &AtomicBool,
    progress: &mut dyn FnMut(TransferProgress),
) -> Result<JobResult> {
    run_job_inner(binding, storage, journal, sink, flag, progress, None)
        .map(|(result, _guards)| result)
}
/// Only the album driver can supply its freshly held destination identity.
pub(crate) fn run_album_transfers<S: TransferStorage>(
    binding: &ExecutionBinding,
    storage: &mut S,
    journal: &mut PhysicalJournalV2,
    sink: &mut dyn JournalStore,
    flag: &AtomicBool,
    progress: &mut dyn FnMut(TransferProgress),
    parent: &ObjectIdentity,
) -> Result<(JobResult, HeldTransferGuards<S>)> {
    run_job_inner(
        binding,
        storage,
        journal,
        sink,
        flag,
        progress,
        Some(parent),
    )
}
#[allow(clippy::too_many_arguments)]
fn run_job_inner<S: TransferStorage>(
    binding: &ExecutionBinding,
    storage: &mut S,
    journal: &mut PhysicalJournalV2,
    sink: &mut dyn JournalStore,
    flag: &AtomicBool,
    progress: &mut dyn FnMut(TransferProgress),
    album_parent: Option<&ObjectIdentity>,
) -> Result<(JobResult, HeldTransferGuards<S>)> {
    if binding.data.album_context.is_some() != album_parent.is_some() {
        return Err(E::Scope);
    }
    if binding.recovery_only
        || !binding.data.valid_domain()
        || binding.sha256()? != journal.binding().sha256()?
        || binding.operations().is_empty()
    {
        return Err(E::InvalidBinding);
    }
    if journal.needs_recovery() || !journal.events().is_empty() {
        return Err(E::NeedsRecovery);
    }
    let mut runner = Runner {
        binding,
        storage,
        journal,
        sink,
        flag,
        progress,
        sources: HashMap::new(),
        published: HashMap::new(),
        items: vec![],
        stop: false,
        album_parent,
    };
    for op in binding.operations() {
        if runner.stop || !runner.journal.dependencies_ready(op.id) {
            runner.items.push(ItemResult {
                operation: op.id,
                status: ItemStatus::Blocked,
                detail: "prior effect requires reconciliation or dependency did not complete"
                    .into(),
                observations: vec![],
                error: None,
            });
            continue;
        }
        let mut item = ItemResult {
            operation: op.id,
            status: ItemStatus::Complete,
            detail: String::new(),
            observations: vec![],
            error: None,
        };
        if record(runner.journal, runner.sink, op, Phase::Prepared, None).is_err() {
            item.status = ItemStatus::NeedsRecovery;
            item.detail = "journal failed before operation".into();
            runner.stop = true;
            runner.items.push(item);
            continue;
        }
        if let Err(failure) = runner.execute(op, &mut item) {
            if let Some(observation) = failure.error.observed.as_deref() {
                item.observations.push(observation.clone())
            }
            item.detail = failure.error.to_string();
            item.error = Some(failure.error.clone());
            let phase = if failure.pending {
                item.status = ItemStatus::NeedsRecovery;
                runner.stop = true;
                Phase::NeedsRecovery
            } else if failure.error.kind == MutationErrorKind::Cancelled {
                item.status = ItemStatus::Cancelled;
                runner.stop = true;
                Phase::Cancelled
            } else {
                item.status = ItemStatus::Failed;
                Phase::Failed
            };
            if record(runner.journal, runner.sink, op, phase, None).is_err() {
                item.status = ItemStatus::NeedsRecovery;
                runner.stop = true;
            }
        }
        runner.items.push(item);
    }
    // A completed duet retires one source after all of its destinations. Earlier
    // cohort items can therefore now be reported complete, without duplicate moves.
    let retired: HashSet<_> = runner
        .journal
        .events()
        .iter()
        .filter(|e| e.phase == Phase::SourceRetired)
        .map(|e| {
            &binding.operations()[e.operation]
                .source
                .observation
                .reference
        })
        .collect();
    for item in &mut runner.items {
        if item.status == ItemStatus::SourceRetained
            && retired.contains(
                &binding.operations()[item.operation]
                    .source
                    .observation
                    .reference,
            )
        {
            item.status = ItemStatus::Complete;
            item.detail.clear();
        }
    }
    let result = JobResult::from_journal(runner.journal, runner.items, cancelled(flag));
    Ok((
        result,
        HeldTransferGuards {
            _sources: runner.sources,
            _published: runner.published,
        },
    ))
}
/// Private ownership carrier: album finalization must keep these guards alive
/// through its final inspection AND parent journal append/sync. No re-opening by
/// name can substitute for retaining the original verified guards.
pub(crate) struct HeldTransferGuards<S: TransferStorage> {
    _sources: HashMap<StorageRef, S::SourceLease>,
    _published: HashMap<usize, (S::Published, MutationObservation)>,
}

struct Runner<'a, S: TransferStorage> {
    binding: &'a ExecutionBinding,
    storage: &'a mut S,
    journal: &'a mut PhysicalJournalV2,
    sink: &'a mut dyn JournalStore,
    flag: &'a AtomicBool,
    progress: &'a mut dyn FnMut(TransferProgress),
    sources: HashMap<StorageRef, S::SourceLease>,
    published: HashMap<usize, (S::Published, MutationObservation)>,
    items: Vec<ItemResult>,
    stop: bool,
    album_parent: Option<&'a ObjectIdentity>,
}
impl<S: TransferStorage> Runner<'_, S> {
    fn append(
        &mut self,
        op: &BoundOperation,
        phase: Phase,
        id: Option<ObjectIdentity>,
    ) -> std::result::Result<(), Failure> {
        record(self.journal, self.sink, op, phase, id).map_err(|e| {
            Failure::pending(error(
                MutationErrorKind::UnknownOutcome,
                format!("journal {phase:?}: {e}"),
            ))
        })
    }
    fn check_cancel(&self) -> std::result::Result<(), Failure> {
        if cancelled(self.flag) {
            Err(error(MutationErrorKind::Cancelled, "cancelled before dispatch").into())
        } else {
            Ok(())
        }
    }
    fn execute(
        &mut self,
        op: &BoundOperation,
        item: &mut ItemResult,
    ) -> std::result::Result<(), Failure> {
        self.check_cancel()?;
        let caps = self.storage.capabilities();
        let rename_only = self.binding.data.windows_rename();
        #[cfg(all(windows, feature = "experimental-windows-rename"))]
        let scoped_rename = rename_only && self.storage.owned_windows_rename_contract();
        #[cfg(not(all(windows, feature = "experimental-windows-rename")))]
        let scoped_rename = false;
        if (rename_only
            && (!scoped_rename
                || !caps.same_volume_no_replace_rename.is_supported()
                || !caps.content_verification.is_supported()))
            || (!rename_only
                && (!caps.exclusive_create.is_supported()
                    || !caps.no_replace_publish.is_supported()
                    || !caps.content_verification.is_supported()
                    || (op.mode == TransferMode::Move
                        && (!caps.leased_source_retirement.is_supported()
                            || !caps.durable_flush.is_supported()))))
        {
            return Err(error(
                MutationErrorKind::Unsupported,
                "required mutation, verification or move durability capability is unproven",
            )
            .into());
        }
        let expected = source_expected(op).map_err(|_| {
            error(
                MutationErrorKind::StalePlan,
                "source requires stronger inspected evidence",
            )
        })?;
        let reference = &op.source.observation.reference;
        if !self.sources.contains_key(reference) {
            let parent = self.storage.source_parent(
                &op.source,
                self.binding.data.source_root.identity.as_ref().unwrap(),
            )?;
            if self.storage.parent_identity(&parent) != expected.parent_identity {
                return Err(error(MutationErrorKind::StalePlan, "source parent changed").into());
            }
            let lease = self.storage.acquire_source(&parent, &expected)?;
            self.sources.insert(reference.clone(), lease);
        }
        let parent = self.storage.destination_parent(
            &op.destination,
            self.binding.data.target_root.identity.as_ref().unwrap(),
        )?;
        let parent_id = self.storage.parent_identity(&parent);
        if self
            .album_parent
            .is_some_and(|expected| expected != &parent_id)
        {
            return Err(error(MutationErrorKind::StalePlan, "owned album parent changed").into());
        }
        let name = mutation_name(
            &op.destination.root,
            op.destination
                .components
                .last()
                .ok_or_else(|| error(MutationErrorKind::InvalidName, "empty destination"))?
                .as_str(),
        )
        .map_err(|_| error(MutationErrorKind::InvalidName, "unsupported name domain"))?;
        let atomic = op.mode == TransferMode::Move
            && !op.copy_effect
            && op.duet_cohort.is_none()
            && self.binding.required_destinations(reference).len() == 1
            && caps.same_volume_no_replace_rename.is_supported()
            && self
                .storage
                .rename_eligible(&self.sources[reference], &parent);
        if rename_only && !atomic {
            return Err(error(
                MutationErrorKind::Unsupported,
                "owned rename cannot fall back to copy/retire",
            )
            .into());
        }
        if atomic {
            self.check_cancel()?;
            self.append(
                op,
                Phase::AtomicRenamePrepared,
                Some(expected.identity.clone()),
            )?;
            if rename_only {
                self.check_cancel()?;
            }
            let published = outcome(self.storage.rename_no_replace(
                self.sources.get_mut(reference).unwrap(),
                &parent,
                &name,
                self.flag,
            ))?;
            let observed = self.storage.inspect(&published).map_err(Failure::pending)?;
            item.observations.push(observed.clone());
            if rename_only && cancelled(self.flag) {
                return Err(Failure::pending(
                    error(
                        MutationErrorKind::Cancelled,
                        "cancelled after owned rename dispatch",
                    )
                    .with_observation(observed),
                ));
            }
            check_observation(
                &observed,
                &expected.identity,
                &parent_id,
                Some(&name),
                Some(expected.digest.length),
            )
            .map_err(Failure::pending)?;
            self.append(
                op,
                Phase::AtomicRenameObserved,
                Some(observed.identity.clone()),
            )?;
            if rename_only && cancelled(self.flag) {
                return Err(Failure::pending(
                    error(
                        MutationErrorKind::Cancelled,
                        "cancelled after owned rename journal acknowledgment",
                    )
                    .with_observation(observed),
                ));
            }
            let verified = self
                .storage
                .verify(&published, &expected.digest)
                .map_err(Failure::pending)?;
            check_observation(
                &verified,
                &expected.identity,
                &parent_id,
                Some(&name),
                Some(expected.digest.length),
            )
            .map_err(Failure::pending)?;
            if rename_only && cancelled(self.flag) {
                return Err(Failure::pending(
                    error(
                        MutationErrorKind::Cancelled,
                        "cancelled after owned rename verification",
                    )
                    .with_observation(verified),
                ));
            }
            self.append(
                op,
                Phase::DestinationVerified,
                Some(verified.identity.clone()),
            )?;
            if rename_only && cancelled(self.flag) {
                return Err(Failure::pending(
                    error(
                        MutationErrorKind::Cancelled,
                        "cancelled before owned rename commit",
                    )
                    .with_observation(verified),
                ));
            }
            self.append(op, Phase::Committed, Some(verified.identity.clone()))?;
            self.published.insert(op.id, (published, verified));
            return Ok(());
        }
        self.check_cancel()?;
        self.append(op, Phase::TempAllocationPrepared, None)?;
        let mut temp = outcome(self.storage.create_temp(
            &parent,
            self.journal.job_id(),
            self.flag,
        ))?;
        let temp_observed = self.storage.temp_observation(&temp);
        item.observations.push(temp_observed.clone());
        check_observation(
            &temp_observed,
            &temp_observed.identity,
            &parent_id,
            None,
            None,
        )
        .map_err(Failure::pending)?;
        self.append(
            op,
            Phase::TempAllocated,
            Some(temp_observed.identity.clone()),
        )?;
        self.check_cancel()?;
        self.append(
            op,
            Phase::BytesWritePrepared,
            Some(temp_observed.identity.clone()),
        )?;
        stream(
            self.storage,
            self.sources.get_mut(reference).unwrap(),
            &mut temp,
            &expected.digest,
            op.id,
            self.flag,
            self.progress,
        )?;
        let synced = self.storage.verify_temp(&temp, &expected.digest)?;
        check_observation(
            &synced,
            &temp_observed.identity,
            &parent_id,
            Some(&temp_observed.actual_name),
            Some(expected.digest.length),
        )?;
        self.append(op, Phase::BytesSynced, Some(temp_observed.identity.clone()))?;
        self.check_cancel()?;
        self.append(
            op,
            Phase::DestinationPublishPrepared,
            Some(temp_observed.identity.clone()),
        )?;
        #[allow(unused_mut)]
        let mut published = outcome(
            self.storage
                .publish_no_replace(&mut temp, &parent, &name, self.flag),
        )?;
        let observed = self.storage.inspect(&published).map_err(Failure::pending)?;
        item.observations.push(observed.clone());
        check_observation(
            &observed,
            &temp_observed.identity,
            &parent_id,
            Some(&name),
            Some(expected.digest.length),
        )
        .map_err(Failure::pending)?;
        self.append(
            op,
            Phase::DestinationPublished,
            Some(observed.identity.clone()),
        )?;
        #[cfg(all(windows, feature = "experimental-windows-copy"))]
        if self.binding.data.windows_copy() {
            let synced = self
                .storage
                .sync_owned_windows_publication(&mut published, &expected.digest, self.flag)
                .map_err(Failure::pending)?;
            check_observation(
                &synced,
                &observed.identity,
                &parent_id,
                Some(&name),
                Some(expected.digest.length),
            )
            .map_err(Failure::pending)?;
            // Publication already occurred; cancellation cannot imply rollback.
            if cancelled(self.flag) {
                return Err(Failure::pending(error(
                    MutationErrorKind::Cancelled,
                    "cancelled after published file sync",
                )));
            }
        }
        let verified = self
            .storage
            .verify(&published, &expected.digest)
            .map_err(Failure::pending)?;
        check_observation(
            &verified,
            &observed.identity,
            &parent_id,
            Some(&name),
            Some(expected.digest.length),
        )
        .map_err(Failure::pending)?;
        self.append(
            op,
            Phase::DestinationVerified,
            Some(verified.identity.clone()),
        )?;
        self.append(op, Phase::Committed, Some(verified.identity.clone()))?;
        self.published.insert(op.id, (published, verified));
        if op.mode == TransferMode::Copy {
            return Ok(());
        }
        item.status = ItemStatus::SourceRetained;
        item.detail = "destination verified; source still retained".into();
        let required: Vec<_> = self
            .binding
            .operations()
            .iter()
            .filter(|o| o.source.observation.reference == *reference)
            .collect();
        if !required.iter().all(|o| self.published.contains_key(&o.id)) {
            return Ok(());
        }
        self.check_cancel()?;
        // Re-read every distinct committed destination while all guards and the
        // source lease remain held. One failed verification preserves the source.
        for prior in required {
            let (published, observed) = &self.published[&prior.id];
            let verified = self
                .storage
                .verify(published, &expected.digest)
                .map_err(Failure::pending)?;
            check_observation(
                &verified,
                &observed.identity,
                &observed.parent_identity,
                Some(&observed.actual_name),
                Some(expected.digest.length),
            )
            .map_err(Failure::pending)?;
        }
        self.check_cancel()?;
        self.append(
            op,
            Phase::SourceRetirementPrepared,
            Some(expected.identity.clone()),
        )?;
        match outcome(
            self.storage
                .retire_source(self.sources.get_mut(reference).unwrap(), self.flag),
        )? {
            RetirementOutcome::Retired(observed) => {
                item.observations.push(observed.clone());
                check_observation(
                    &observed,
                    &expected.identity,
                    &expected.parent_identity,
                    Some(&expected.name),
                    Some(expected.digest.length),
                )
                .map_err(Failure::pending)?;
                self.append(op, Phase::SourceRetired, Some(expected.identity))?;
                item.status = ItemStatus::Complete;
                item.detail.clear();
            }
            RetirementOutcome::DispositionRequested(observed) => {
                return Err(Failure::pending(
                    error(
                        MutationErrorKind::UnknownOutcome,
                        "delete disposition accepted; source removal not proven",
                    )
                    .with_observation(observed),
                ));
            }
        }
        Ok(())
    }
}

fn mutation_name(reference: &StorageRef, name: &str) -> Result<MutationName> {
    match reference {
        StorageRef::TestOpaque { .. } => Ok(MutationName::Provider(name.into())),
        StorageRef::WindowsPath { .. }
            if cfg!(all(windows, feature = "experimental-windows-copy")) =>
        {
            Ok(MutationName::Windows(name.encode_utf16().collect()))
        }
        _ => Err(E::Unsupported),
    }
}
