//! Read-only reconciliation: reports evidence; never appends, retries or deletes.
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
    if binding.sha256()? != journal.binding().sha256()? {
        return Err(E::InvalidBinding);
    }
    let mut report = RecoveryReport {
        operations: vec![],
        append_blocked: journal.needs_recovery(),
    };
    // Capture every metadata receipt before any file-content/stream reads. A
    // retirement cohort can inspect a later destination during the first item.
    let metadata_evidence = binding
        .operations()
        .iter()
        .map(|op| {
            if !binding.data.metadata_required_for(op) {
                return CopyMetadataRecoveryStatus::NotRequested;
            }
            let expected = journal
                .events()
                .iter()
                .filter(|event| event.operation == op.id)
                .find_map(|event| event.copy_metadata.as_ref());
            match expected.map(|expected| storage.verify_copy_metadata(&op.destination, expected)) {
                Some(Ok(true)) => CopyMetadataRecoveryStatus::Verified,
                Some(Ok(false)) => CopyMetadataRecoveryStatus::Mismatch,
                _ => CopyMetadataRecoveryStatus::Unverified,
            }
        })
        .collect::<Vec<_>>();
    let mut observed_source_absent = vec![false; binding.operations().len()];
    for op in binding.operations() {
        let history: Vec<_> = journal
            .events()
            .iter()
            .filter(|event| event.operation == op.id)
            .collect();
        let copy_metadata = metadata_evidence[op.id];
        let source = storage.source(&op.source.observation.reference);
        let dest = storage.destination(&op.destination);
        let mut status = RecoveryStatus::NeedsRecovery;
        if !journal.needs_recovery()
            && let (Ok(source), Ok(dest)) = (source, dest)
        {
            observed_source_absent[op.id] = source.is_none();
            let auxiliary = binding
                .data
                .copied_move_sources
                .as_ref()
                .and_then(|sources| {
                    sources
                        .iter()
                        .find(|source| source.source == op.source.observation.reference)
                });
            let source_streams_ok = auxiliary.is_none_or(|expected| {
                matches!(storage.verify_move_source_streams(expected), Ok(true))
            });
            let source_ok = source_streams_ok
                && source.as_ref().is_some_and(|s| {
                    Some(&s.identity) == op.source.observation.identity.as_ref()
                        && s.sha256 == op.source.sha256
                });
            let atomic = history
                .iter()
                .any(|e| e.phase == Phase::AtomicRenamePrepared);
            let expected = if atomic {
                op.source.observation.identity.as_ref()
            } else {
                history
                    .iter()
                    .find(|e| e.phase == Phase::DestinationPublishPrepared)
                    .and_then(|e| e.observed_identity.as_ref())
            };
            let destination_streams_ok = auxiliary.is_none_or(|auxiliary| {
                expected.is_some_and(|identity| {
                    matches!(
                        storage.verify_move_destination_streams(
                            &op.destination,
                            identity,
                            auxiliary
                        ),
                        Ok(true)
                    )
                })
            });
            let dest_ok = destination_streams_ok
                && dest
                    .as_ref()
                    .is_some_and(|d| Some(&d.identity) == expected && d.sha256 == op.source.sha256);
            let terminal = history.iter().any(|e| {
                matches!(
                    e.phase,
                    Phase::Failed | Phase::Cancelled | Phase::NeedsRecovery
                )
            });
            if !terminal && dest_ok && (source_ok && !atomic || source.is_none() && atomic) {
                status = if history.iter().any(|e| e.phase == Phase::Committed) {
                    RecoveryStatus::Committed
                } else {
                    RecoveryStatus::EffectProvenAwaitingJournal
                };
            } else if !terminal
                && source_ok
                && dest.is_none()
                && history.iter().all(|e| e.phase == Phase::Prepared)
            {
                status = RecoveryStatus::ReadyForExplicitResume;
            }
            // Absence following retirement intent alone cannot prove which actor
            // removed the source. Require the durable post-event and every distinct
            // required destination still carrying the exact committed identity+digest.
            if !terminal && source.is_none() && !atomic && dest_ok {
                let retired = journal.events().iter().any(|e| {
                    e.phase == Phase::SourceRetired
                        && binding.operations()[e.operation]
                            .source
                            .observation
                            .reference
                            == op.source.observation.reference
                });
                if retired {
                    let mut all = true;
                    for required in binding.operations().iter().filter(|o| {
                        o.source.observation.reference == op.source.observation.reference
                    }) {
                        let committed = journal
                            .events()
                            .iter()
                            .find(|e| e.operation == required.id && e.phase == Phase::Committed);
                        let observed = storage.destination(&required.destination);
                        all &= matches!((committed,observed),(Some(e),Ok(Some(d))) if Some(&d.identity)==e.observed_identity.as_ref() && d.sha256==required.source.sha256);
                        if let Some(auxiliary) = auxiliary {
                            all &= committed
                                .and_then(|event| event.observed_identity.as_ref())
                                .is_some_and(|identity| {
                                    matches!(
                                        storage.verify_move_destination_streams(
                                            &required.destination,
                                            identity,
                                            auxiliary
                                        ),
                                        Ok(true)
                                    )
                                });
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
            && journal.events().iter().any(|event| {
                event.phase == Phase::SourceRetired
                    && binding.operations()[event.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            })
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
        report.operations.push(OperationRecovery {
            copy_metadata,
            operation: op.id,
            status,
        });
    }
    // Copied Move retirement is a cohort fact: a bad stream or metadata receipt at
    // any required destination invalidates the whole retired-source assertion.
    if binding.data.copied_move_sources.is_some() {
        let verified = report
            .operations
            .iter()
            .map(|operation| {
                operation.status == RecoveryStatus::Committed
                    && operation.copy_metadata == CopyMetadataRecoveryStatus::Verified
            })
            .collect::<Vec<_>>();
        for operation in &mut report.operations {
            if binding
                .data
                .copied_move_operation(&binding.operations()[operation.operation])
                && observed_source_absent[operation.operation]
                && binding
                    .operations()
                    .iter()
                    .filter(|candidate| {
                        candidate.source.observation.reference
                            == binding.operations()[operation.operation]
                                .source
                                .observation
                                .reference
                    })
                    .any(|candidate| !verified[candidate.id])
            {
                operation.status = RecoveryStatus::NeedsRecovery;
            }
        }
    }
    // An invalidated predecessor prevents resuming a stale simulated timeline.
    for i in 0..report.operations.len() {
        if report.operations[i].status == RecoveryStatus::ReadyForExplicitResume
            && binding.operations()[i].depends_on.iter().any(|d| {
                let prior = &binding.operations()[*d];
                let next = &binding.operations()[i];
                report.operations[*d].status != RecoveryStatus::Committed
                    || !journal.dependency_ready(*d, i)
                    || (prior.mode == TransferMode::Move
                        && !same_duet_cohort(prior, next)
                        && !observed_source_absent[*d])
            })
        {
            report.operations[i].status = RecoveryStatus::NeedsRecovery;
        }
    }
    Ok(report)
}
