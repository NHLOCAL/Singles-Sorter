//! Read-only reconciliation: reports evidence; never appends, retries or deletes.
use crate::inspection_control::{
    InspectionControl, any as controlled_any, checkpoint, find as controlled_find,
};
use crate::{ExecutionError as E, Result, binding::*, physical_journal::*};
use sorter_core::discovery::{ObjectIdentity, StorageRef};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedObject {
    pub identity: ObjectIdentity,
    pub sha256: String,
}
/// Inspection must resolve the bound reference/destination under the bound roots.
/// None means proven absent; access failure is an error, never absence.
pub trait RecoveryStorage {
    fn source(&mut self, reference: &StorageRef) -> Result<Option<ObservedObject>>;
    fn destination(&mut self, destination: &DestinationEvidence) -> Result<Option<ObservedObject>>;
    /// Separate full-stream proof for the preview-bound copied Move domain.
    /// Defaults refuse so a byte-only recovery adapter cannot certify retirement.
    fn verify_move_source_streams(&mut self, _expected: &CopiedMoveSourceEvidence) -> Result<bool> {
        Err(E::Unsupported)
    }
    fn verify_move_destination_streams(
        &mut self,
        _destination: &DestinationEvidence,
        _identity: &ObjectIdentity,
        _expected: &CopiedMoveSourceEvidence,
    ) -> Result<bool> {
        Err(E::Unsupported)
    }
    /// A recorded receipt does not imply that target metadata still matches.
    fn verify_copy_metadata(
        &mut self,
        _destination: &DestinationEvidence,
        _expected: &CopyMetadataEvidence,
    ) -> Result<bool> {
        Err(E::Unsupported)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryStatus {
    Committed,
    /// No effect is adopted. A separately authorized resume must revalidate first.
    ReadyForExplicitResume,
    /// Exact identity+digest proves an effect, but durable completion is missing.
    EffectProvenAwaitingJournal,
    NeedsRecovery,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyMetadataRecoveryStatus {
    NotRequested,
    Unverified,
    Verified,
    Mismatch,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRecovery {
    pub copy_metadata: CopyMetadataRecoveryStatus,
    pub operation: usize,
    pub status: RecoveryStatus,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryReport {
    pub operations: Vec<OperationRecovery>,
    pub append_blocked: bool,
}
pub fn recover_job(
    binding: &ExecutionBinding,
    journal: &PhysicalJournalV2,
    storage: &mut dyn RecoveryStorage,
) -> Result<RecoveryReport> {
    recover_job_inner(binding, journal, storage, None, 0)
}
/// Read-only observations may start at a previous cursor, but every operation is
/// reverified. Report IDs and dependency evaluation retain the original order.
pub fn recover_job_controlled(
    binding: &ExecutionBinding,
    journal: &PhysicalJournalV2,
    storage: &mut dyn RecoveryStorage,
    control: &InspectionControl,
    start_operation: usize,
) -> Result<RecoveryReport> {
    recover_job_inner(binding, journal, storage, Some(control), start_operation)
}
fn observation<T>(
    control: Option<&InspectionControl>,
    read: impl FnOnce() -> Result<T>,
) -> Result<Result<T>> {
    checkpoint(control)?;
    let result = read();
    checkpoint(control)?;
    if result.as_ref().err() == Some(&E::Cancelled) {
        return Err(E::Cancelled);
    }
    Ok(result)
}
pub(crate) fn recover_job_inner(
    binding: &ExecutionBinding,
    journal: &PhysicalJournalV2,
    storage: &mut dyn RecoveryStorage,
    control: Option<&InspectionControl>,
    start_operation: usize,
) -> Result<RecoveryReport> {
    checkpoint(control)?;
    if binding.sha256()? != journal.binding().sha256()? {
        return Err(E::InvalidBinding);
    }
    let count = binding.operations().len();
    if control.is_some() && count > crate::inspection_control::MAX_OPERATIONS {
        return Err(E::Unsupported);
    }
    if start_operation >= count && start_operation != 0 {
        return Err(E::InvalidBinding);
    }
    if let Some(control) = control {
        control.operations(count)?;
        control.stage("metadata")?;
    }
    let mut report = RecoveryReport {
        operations: Vec::with_capacity(count),
        append_blocked: journal.needs_recovery(),
    };
    // Preserve the current whole-cohort metadata prepass: no source, destination
    // or ADS content is read until every route-required receipt has been probed.
    for op in binding.operations() {
        checkpoint(control)?;
        let copy_metadata = if !binding.data.metadata_required_for(op) {
            CopyMetadataRecoveryStatus::NotRequested
        } else {
            let expected = controlled_find(journal.events().iter(), control, |event| {
                event.operation == op.id && event.copy_metadata.is_some()
            })?
            .and_then(|event| event.copy_metadata.as_ref());
            match expected {
                Some(expected) => match observation(control, || {
                    storage.verify_copy_metadata(&op.destination, expected)
                })? {
                    Ok(true) => CopyMetadataRecoveryStatus::Verified,
                    Ok(false) => CopyMetadataRecoveryStatus::Mismatch,
                    Err(_) => CopyMetadataRecoveryStatus::Unverified,
                },
                None => CopyMetadataRecoveryStatus::Unverified,
            }
        };
        report.operations.push(OperationRecovery {
            copy_metadata,
            operation: op.id,
            status: RecoveryStatus::NeedsRecovery,
        });
    }
    let mut observed_source_absent = vec![false; count];
    for index in (start_operation..count).chain(0..start_operation) {
        checkpoint(control)?;
        let op = &binding.operations()[index];
        if let Some(control) = control {
            control.begin_operation(op.id)?;
        }
        let mut history = Vec::new();
        for event in journal.events() {
            checkpoint(control)?;
            if event.operation == op.id {
                history.push(event);
            }
        }
        let copy_metadata = report.operations[op.id].copy_metadata;
        let source = observation(control, || storage.source(&op.source.observation.reference))?;
        let dest = observation(control, || storage.destination(&op.destination))?;
        let mut status = RecoveryStatus::NeedsRecovery;
        if !journal.needs_recovery()
            && let (Ok(source), Ok(dest)) = (source, dest)
        {
            observed_source_absent[op.id] = source.is_none();
            let auxiliary = controlled_find(
                binding.data.copied_move_sources.iter().flatten(),
                control,
                |source| source.source == op.source.observation.reference,
            )?;
            let source_streams_ok = match auxiliary {
                Some(expected) => {
                    observation(control, || storage.verify_move_source_streams(expected))?
                        .unwrap_or(false)
                }
                None => true,
            };
            let source_ok = source_streams_ok
                && source.as_ref().is_some_and(|source| {
                    Some(&source.identity) == op.source.observation.identity.as_ref()
                        && source.sha256 == op.source.sha256
                });
            let atomic = controlled_any(history.iter(), control, |event| {
                event.phase == Phase::AtomicRenamePrepared
            })?;
            let expected = if atomic {
                op.source.observation.identity.as_ref()
            } else {
                controlled_find(history.iter(), control, |event| {
                    event.phase == Phase::DestinationPublishPrepared
                })?
                .and_then(|event| event.observed_identity.as_ref())
            };
            let destination_streams_ok = match (auxiliary, expected) {
                (Some(auxiliary), Some(identity)) => observation(control, || {
                    storage.verify_move_destination_streams(&op.destination, identity, auxiliary)
                })?
                .unwrap_or(false),
                (Some(_), None) => false,
                (None, _) => true,
            };
            let dest_ok = destination_streams_ok
                && dest.as_ref().is_some_and(|dest| {
                    Some(&dest.identity) == expected && dest.sha256 == op.source.sha256
                });
            let terminal = controlled_any(history.iter(), control, |event| {
                matches!(
                    event.phase,
                    Phase::Failed | Phase::Cancelled | Phase::NeedsRecovery
                )
            })?;
            if !terminal && dest_ok && (source_ok && !atomic || source.is_none() && atomic) {
                status = if controlled_any(history.iter(), control, |event| {
                    event.phase == Phase::Committed
                })? {
                    RecoveryStatus::Committed
                } else {
                    RecoveryStatus::EffectProvenAwaitingJournal
                };
            } else if !terminal
                && source_ok
                && dest.is_none()
                && !controlled_any(history.iter(), control, |event| {
                    event.phase != Phase::Prepared
                })?
            {
                status = RecoveryStatus::ReadyForExplicitResume;
            }
            // Retired source absence alone cannot prove the effect. Recheck each
            // required destination's exact identity, main digest and full streams.
            if !terminal && source.is_none() && !atomic && dest_ok {
                let retired = controlled_any(journal.events().iter(), control, |event| {
                    event.phase == Phase::SourceRetired
                        && binding.operations()[event.operation]
                            .source
                            .observation
                            .reference
                            == op.source.observation.reference
                })?;
                if retired {
                    let mut all = true;
                    for required in binding.operations() {
                        checkpoint(control)?;
                        if required.source.observation.reference != op.source.observation.reference
                        {
                            continue;
                        }
                        let committed =
                            controlled_find(journal.events().iter(), control, |event| {
                                event.operation == required.id && event.phase == Phase::Committed
                            })?;
                        let observed =
                            observation(control, || storage.destination(&required.destination))?;
                        all &= matches!((committed, observed), (Some(event), Ok(Some(dest))) if Some(&dest.identity) == event.observed_identity.as_ref() && dest.sha256 == required.source.sha256);
                        if let Some(auxiliary) = auxiliary {
                            all &= match committed
                                .and_then(|event| event.observed_identity.as_ref())
                            {
                                Some(identity) => observation(control, || {
                                    storage.verify_move_destination_streams(
                                        &required.destination,
                                        identity,
                                        auxiliary,
                                    )
                                })?
                                .unwrap_or(false),
                                None => false,
                            };
                        }
                    }
                    if all {
                        status = RecoveryStatus::Committed;
                    }
                }
            }
        }
        if binding.data.copied_move_sources.is_some()
            && !observed_source_absent[op.id]
            && controlled_any(journal.events().iter(), control, |event| {
                event.phase == Phase::SourceRetired
                    && binding.operations()[event.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            })?
        {
            status = RecoveryStatus::NeedsRecovery;
        }
        if binding.data.metadata_required_for(op)
            && matches!(
                status,
                RecoveryStatus::Committed | RecoveryStatus::EffectProvenAwaitingJournal
            )
            && copy_metadata != CopyMetadataRecoveryStatus::Verified
        {
            status = RecoveryStatus::NeedsRecovery;
        }
        report.operations[op.id].status = status;
        if let Some(control) = control {
            control.complete_operation()?;
        }
    }
    // Preserve cohort-wide invalidation and route-specific metadata requirements.
    if binding.data.copied_move_sources.is_some() {
        let mut verified = Vec::with_capacity(count);
        for operation in &report.operations {
            checkpoint(control)?;
            verified.push(
                operation.status == RecoveryStatus::Committed
                    && operation.copy_metadata == CopyMetadataRecoveryStatus::Verified,
            );
        }
        for operation in &mut report.operations {
            checkpoint(control)?;
            if binding
                .data
                .copied_move_operation(&binding.operations()[operation.operation])
                && observed_source_absent[operation.operation]
                && controlled_any(binding.operations().iter(), control, |candidate| {
                    candidate.source.observation.reference
                        == binding.operations()[operation.operation]
                            .source
                            .observation
                            .reference
                        && !verified[candidate.id]
                })?
            {
                operation.status = RecoveryStatus::NeedsRecovery;
            }
        }
    }
    // This pass intentionally stays in original dependency order, regardless of
    // the observation cursor. No previously observed hash is reused as current.
    for index in 0..count {
        checkpoint(control)?;
        if report.operations[index].status != RecoveryStatus::ReadyForExplicitResume {
            continue;
        }
        for dependency in &binding.operations()[index].depends_on {
            checkpoint(control)?;
            let prior = &binding.operations()[*dependency];
            let next = &binding.operations()[index];
            if report.operations[*dependency].status != RecoveryStatus::Committed
                || !journal.dependency_ready_with_control(*dependency, index, control)?
                || (prior.mode == TransferMode::Move
                    && !same_duet_cohort(prior, next)
                    && !observed_source_absent[*dependency])
            {
                report.operations[index].status = RecoveryStatus::NeedsRecovery;
                break;
            }
        }
    }
    checkpoint(control)?;
    Ok(report)
}
