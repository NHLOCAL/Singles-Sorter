//! Read-only evidence reporting; no retry, append, cleanup or directory adoption.
use super::*;
use crate::{binding::SourceEvidence, recovery::*};
use sorter_core::discovery::EntryObservation;

pub trait AlbumRecoveryStorage: RecoveryStorage {
    /// Resolve the bound source under its original root. Absence is proven, not
    /// inferred from permission failures. Return the complete ordered child set.
    fn album_source(
        &mut self,
        binding: &AlbumBinding,
    ) -> Result<Option<(EntryObservation, Vec<SourceEvidence>)>>;
    fn directory(
        &mut self,
        binding: &AlbumBinding,
        index: usize,
    ) -> Result<Option<MutationObservation>>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlbumRecoveryReport {
    pub status: RecoveryStatus,
    pub directories: Vec<RecoveryStatus>,
    pub transfers: Option<RecoveryReport>,
    /// Always true: this slice never replays or extends a reopened album journal.
    pub append_blocked: bool,
}
pub fn reconcile_album<S: AlbumRecoveryStorage>(
    journal: &AlbumJournal,
    storage: &mut S,
) -> Result<AlbumRecoveryReport> {
    let binding = journal.binding();
    let source_ok = matches!(storage.album_source(binding), Ok(Some((ref source, ref children)))
        if source == binding.source() && *children == binding.children().cloned().collect::<Vec<_>>());
    let mut report = AlbumRecoveryReport {
        status: RecoveryStatus::NeedsRecovery,
        directories: vec![],
        transfers: None,
        append_blocked: true,
    };
    for (index, d) in binding.directories().iter().enumerate() {
        let observed = storage.directory(binding, index);
        let identity = journal.expected_directory_identity(index);
        let intent = journal
            .events()
            .iter()
            .any(|e| matches!(e, AlbumEvent::DirectoryIntent { index:i } if *i == index));
        let synced = journal
            .events()
            .iter()
            .any(|e| matches!(e, AlbumEvent::DirectorySynced { index:i,.. } if *i == index));
        let expected_name = if index == 0 {
            d.existing
                .as_ref()
                .map(|e| MutationName::Provider(e.name.clone()))
        } else {
            Some(MutationName::Provider(d.components.last().unwrap().clone()))
        };
        let expected_parent = if index == 0 {
            d.existing.as_ref().and_then(|e| e.parent_identity.clone())
        } else {
            journal.expected_directory_identity(index - 1)
        };
        let status = if !source_ok || journal.needs_recovery() {
            RecoveryStatus::NeedsRecovery
        } else {
            match observed {
                Ok(Some(o))
                    if Some(&o.identity) == identity.as_ref()
                        && Some(&o.actual_name) == expected_name.as_ref()
                        && (index == 0 || Some(&o.parent_identity) == expected_parent.as_ref())
                        && matches!(
                            o.reference,
                            sorter_core::discovery::StorageRef::TestOpaque { .. }
                        ) =>
                {
                    if d.existing.is_some() || synced {
                        RecoveryStatus::Committed
                    } else {
                        RecoveryStatus::EffectProvenAwaitingJournal
                    }
                }
                Ok(None) if d.existing.is_none() && !intent => {
                    RecoveryStatus::ReadyForExplicitResume
                }
                _ => RecoveryStatus::NeedsRecovery,
            }
        };
        report.directories.push(status);
    }
    if !journal.child_bytes.is_empty() {
        let transfers = binding.transfers();
        let child = PhysicalJournalV2::open(&transfers, &journal.job_id, &journal.child_bytes)?;
        report.transfers = Some(recover_job(&transfers, &child, storage)?);
    }
    let dirs_ok = report
        .directories
        .iter()
        .all(|s| *s == RecoveryStatus::Committed);
    if source_ok && !journal.needs_recovery() && dirs_ok {
        let children_ok = report.transfers.as_ref().is_some_and(|r| {
            !r.append_blocked
                && r.operations
                    .iter()
                    .all(|o| o.status == RecoveryStatus::Committed)
        });
        if children_ok && matches!(journal.events().last(), Some(AlbumEvent::Complete)) {
            report.status = RecoveryStatus::Committed;
        }
    }
    if (!source_ok || journal.needs_recovery() || !dirs_ok)
        && let Some(children) = &mut report.transfers
    {
        children.append_blocked = true;
        for op in &mut children.operations {
            op.status = RecoveryStatus::NeedsRecovery;
        }
    }
    Ok(report)
}
