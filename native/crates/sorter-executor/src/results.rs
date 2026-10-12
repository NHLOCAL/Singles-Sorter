//! Results count durable evidence, never attempted or merely dispatched effects.
use crate::{
    binding::TransferMode,
    physical_journal::{Phase, PhysicalJournalV2},
};
use serde::Serialize;
use sorter_core::discovery::ObjectIdentity;
use sorter_storage_contracts::{MutationError, MutationObservation};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Complete,
    Partial,
    Cancelled,
    NeedsRecovery,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemStatus {
    Complete,
    SourceRetained,
    Failed,
    Cancelled,
    Blocked,
    NeedsRecovery,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemResult {
    pub operation: usize,
    pub status: ItemStatus,
    pub detail: String,
    pub error: Option<MutationError>,
    /// Actual returned evidence, including pending or unexpected effects.
    pub observations: Vec<MutationObservation>,
}
/// Acknowledged namespace effects in journal order. This omits directory
/// preparation, metadata writes and uncertain dispatches; it is not a replay API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObservedEffectReceipt {
    pub operation: usize,
    pub kind: String,
    pub identity: ObjectIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobResult {
    pub status: JobStatus,
    pub source_items: usize,
    pub committed_destinations: usize,
    /// Committed destination operation IDs in actual acknowledgment order.
    pub committed_operations: Vec<usize>,
    pub effect_receipts: Vec<ObservedEffectReceipt>,
    pub moved_sources: usize,
    pub failed_operations: usize,
    pub blocked_operations: usize,
    pub recovery_operations: usize,
    pub cancelled_operations: usize,
    pub items: Vec<ItemResult>,
}
impl JobResult {
    pub(crate) fn from_journal(
        journal: &PhysicalJournalV2,
        items: Vec<ItemResult>,
        cancelled: bool,
    ) -> Self {
        let events = journal.events();
        let operations = journal.binding().operations();
        let committed: HashSet<_> = events
            .iter()
            .filter(|e| e.phase == Phase::Committed)
            .map(|e| e.destination_id.as_str())
            .collect();
        let moved: HashSet<_> = operations
            .iter()
            .filter(|op| {
                op.mode == TransferMode::Move
                    && (events
                        .iter()
                        .any(|e| e.operation == op.id && e.phase == Phase::SourceRetired)
                        || (events.iter().any(|e| {
                            e.operation == op.id && e.phase == Phase::AtomicRenameObserved
                        }) && committed.contains(op.destination.id.as_str())))
            })
            .map(|op| &op.source.observation.reference)
            .collect();
        let status = if journal.needs_recovery()
            || items.iter().any(|i| i.status == ItemStatus::NeedsRecovery)
        {
            JobStatus::NeedsRecovery
        } else if cancelled || items.iter().any(|i| i.status == ItemStatus::Cancelled) {
            JobStatus::Cancelled
        } else if items.iter().all(|i| i.status == ItemStatus::Complete)
            && items.len() == operations.len()
        {
            JobStatus::Complete
        } else {
            JobStatus::Partial
        };
        Self {
            status,
            source_items: operations
                .iter()
                .map(|o| &o.source.observation.reference)
                .collect::<HashSet<_>>()
                .len(),
            committed_destinations: committed.len(),
            committed_operations: events
                .iter()
                .filter(|event| event.phase == Phase::Committed)
                .map(|event| event.operation)
                .collect(),
            effect_receipts: events
                .iter()
                .filter_map(|event| {
                    let kind = match event.phase {
                        Phase::DestinationPublished => "file_published",
                        Phase::AtomicRenameObserved => "file_renamed",
                        Phase::SourceRetired => "source_retired",
                        _ => return None,
                    };
                    Some(ObservedEffectReceipt {
                        operation: event.operation,
                        kind: kind.into(),
                        identity: event.observed_identity.clone()?,
                    })
                })
                .collect(),
            moved_sources: moved.len(),
            failed_operations: items
                .iter()
                .filter(|i| i.status == ItemStatus::Failed)
                .count(),
            blocked_operations: items
                .iter()
                .filter(|i| i.status == ItemStatus::Blocked)
                .count(),
            recovery_operations: items
                .iter()
                .filter(|i| i.status == ItemStatus::NeedsRecovery)
                .count(),
            cancelled_operations: items
                .iter()
                .filter(|i| i.status == ItemStatus::Cancelled)
                .count(),
            items,
        }
    }
}
