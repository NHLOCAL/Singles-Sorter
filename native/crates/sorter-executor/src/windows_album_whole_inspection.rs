//! Fresh read-only reconciliation of Whole-Album evidence. Atomic directory
//! operations remain atomic; descendant rows are content inspection, not moves.
use super::super::recorded::{
    AlbumFileInspectionGrants, AlbumFileInspectionJournal, AlbumFileInspectionSource,
    AlbumInspectionSourceState,
};
use super::recorded::mapped;
use super::*;
use crate::windows_copy::{inspection_error, inspection_event};

#[derive(Clone, Debug)]
pub struct RecordedWholeAlbumInspection {
    pub recorded: RecordedWholeAlbumJob,
    pub source_state: AlbumInspectionSourceState,
    pub recovery: WholeAlbumRecovery,
    pub files: Vec<WholeAlbumInspectionFile>,
    pub directories: Vec<AlbumDirectoryEvidence>,
    pub membership: Vec<WholeAlbumMembershipEvidence>,
    pub content_scope: &'static str,
}
#[derive(Clone, Debug)]
pub struct WholeAlbumInspectionFile {
    pub file: RecordedWholeAlbumFile,
    pub main: Option<AlbumFileEvidence>,
    pub transfer: Option<crate::recovery::OperationRecovery>,
}
#[derive(Clone, Debug)]
pub struct WholeAlbumMembershipEvidence {
    pub reference: StorageRef,
    pub status: String,
}
#[derive(Default)]
struct Ledger {
    records: Vec<WindowsAlbumInspectionTarget>,
    indices: HashMap<StorageRef, usize>,
}
impl Ledger {
    fn insert(
        &mut self,
        root: &StorageRef,
        reference: StorageRef,
        identity: Option<ObjectIdentity>,
        parent_identity: ObjectIdentity,
        kind: EntryKind,
    ) -> Result<()> {
        if &reference == root {
            return Ok(());
        }
        if let Some(index) = self.indices.get(&reference) {
            let prior = &self.records[*index];
            if prior.identity != identity
                || prior.parent_identity != parent_identity
                || prior.kind != kind
            {
                return Err(E::CorruptJournal);
            }
            return Ok(());
        }
        self.indices.insert(reference.clone(), self.records.len());
        self.records.push(WindowsAlbumInspectionTarget {
            reference,
            identity,
            parent_identity,
            kind,
        });
        Ok(())
    }
}

fn event_error(error: E) -> AlbumFailure {
    AlbumFailure {
        kind: format!("{error:?}"),
        detail: "fresh read-only observation failed".into(),
        observed: None,
    }
}
fn current(
    owner: &WindowsAlbumReadInspection,
    ledger: &Ledger,
    reference: &StorageRef,
    source: bool,
    present: bool,
    control: &InspectionControl,
) -> Result<Option<MutationObservation>> {
    if source && !present {
        control.checkpoint()?;
        return Ok(None);
    }
    let index = *ledger.indices.get(reference).ok_or(E::Scope)?;
    let cancel = control.cancellation_flag();
    let value = if source {
        owner.inspect_source_entry_observed(index, cancel, &mut |event| {
            inspection_event(control, event)
        })
    } else {
        owner.inspect_entry_observed(index, cancel, &mut |event| inspection_event(control, event))
    };
    value.map_err(inspection_error)
}
fn dir_evidence(
    reference: StorageRef,
    id: ObjectIdentity,
    actual: Result<Option<MutationObservation>>,
) -> Result<AlbumDirectoryEvidence> {
    let mut value = AlbumDirectoryEvidence {
        reference,
        expected_identity: id,
        observed_identity: None,
        status: AlbumDirectoryEvidenceStatus::Unverified,
    };
    match actual {
        Ok(None) => value.status = AlbumDirectoryEvidenceStatus::Absent,
        Ok(Some(observed)) => {
            value.status = if observed.identity == value.expected_identity {
                AlbumDirectoryEvidenceStatus::SameIdentity
            } else {
                AlbumDirectoryEvidenceStatus::Replaced
            };
            value.observed_identity = Some(observed.identity);
        }
        Err(E::Cancelled) => return Err(E::Cancelled),
        Err(_) => {}
    }
    Ok(value)
}
struct MembershipPin {
    pin: WindowsAlbumInspectionDirectory,
    reference: StorageRef,
    expected: Vec<WindowsDirectoryMember>,
}
fn inspect_membership(
    pins: &[MembershipPin],
    control: &InspectionControl,
) -> Result<Vec<WholeAlbumMembershipEvidence>> {
    let mut report = Vec::new();
    for value in pins {
        control.checkpoint()?;
        let status = match value
            .pin
            .members_observed(control.cancellation_flag(), &mut |event| {
                inspection_event(control, event)
            })
            .map_err(inspection_error)
        {
            Ok(actual) => {
                if same_members(actual, value.expected.clone()) {
                    "verified"
                } else {
                    "changed"
                }
            }
            Err(E::Cancelled) => return Err(E::Cancelled),
            Err(_) => "unverified",
        };
        report.push(WholeAlbumMembershipEvidence {
            reference: value.reference.clone(),
            status: status.into(),
        });
    }
    Ok(report)
}

pub fn recover_recorded_whole_album_job(
    recorded: &RecordedWholeAlbumJob,
    grants: AlbumFileInspectionGrants,
    journals: Vec<AlbumFileInspectionJournal>,
    control: &InspectionControl,
    start_file: usize,
) -> Result<RecordedWholeAlbumInspection> {
    control.stage("whole_album_restart_journals")?;
    if start_file > recorded.files.len()
        || journals.is_empty()
        || journals.len() > 3
        || journals[0].name != name(&format!("{}.album-move.jsonl", recorded.job_id))
    {
        return Err(E::CorruptJournal);
    }
    if journals.len() > 1 {
        let singles = recorded.singles_job_id.as_ref().ok_or(E::CorruptJournal)?;
        if journals[1].name != name(&format!("{singles}.album.jsonl"))
            || journals
                .get(2)
                .is_some_and(|entry| entry.name != name(&format!("{singles}.transfers.jsonl")))
        {
            return Err(E::CorruptJournal);
        }
    }
    let expected_hashes = journals
        .iter()
        .map(|j| j.sha256.clone())
        .collect::<Vec<_>>();
    let destination = WindowsAlbumJournalInspectionInputs {
        target: grants.target,
        journal: grants.journal,
        recorded_target: recorded.target.clone(),
        journals: journals
            .into_iter()
            .map(|j| WindowsAlbumInspectionJournal {
                name: j.name,
                identity: j.identity,
                length: j.length,
            })
            .collect(),
    };
    let cancel = control.cancellation_flag();
    let present = matches!(&grants.source, AlbumFileInspectionSource::Present(_));
    let mut reader = match grants.source {
        AlbumFileInspectionSource::Present(source) => {
            WindowsAlbumJournalInspection::open_present_observed(
                WindowsAlbumPresentJournalInspectionInputs {
                    source,
                    recorded_source: recorded.source.clone(),
                    destination,
                },
                cancel,
                &mut |e| inspection_event(control, e),
            )
        }
        AlbumFileInspectionSource::Missing(parent) => {
            let original = recorded.original_source.clone().ok_or(E::Unsupported)?;
            let original = WindowsAlbumOriginalLocationInspection::open_observed(
                parent,
                original,
                cancel,
                &mut |e| inspection_event(control, e),
            )
            .map_err(inspection_error)?;
            WindowsAlbumJournalInspection::open_observed(destination, original, cancel, &mut |e| {
                inspection_event(control, e)
            })
        }
    }
    .map_err(inspection_error)?;
    let original = reader
        .inspect_original_observed(cancel, &mut |e| inspection_event(control, e))
        .map_err(inspection_error)?;
    let source_native = match (&original, present) {
        (WindowsAlbumOriginalLocation::OriginalPresent(value), true) => value.clone(),
        (WindowsAlbumOriginalLocation::Absent, false) => {
            recorded.original_source.clone().ok_or(E::Scope)?
        }
        _ => return Err(E::Stale),
    };
    if recorded
        .original_source
        .as_ref()
        .is_some_and(|value| *value != source_native)
    {
        return Err(E::Stale);
    }
    let (header, mut report, nested) = {
        let bytes = reader
            .read_journals_observed(cancel, &mut |e| inspection_event(control, e))
            .map_err(inspection_error)?;
        if bytes.len() != expected_hashes.len() {
            return Err(E::Stale);
        }
        for (bytes, expected) in bytes.iter().zip(&expected_hashes) {
            let mut hash = Sha256::new();
            for chunk in bytes.chunks(65_536) {
                control.checkpoint()?;
                hash.update(chunk);
            }
            control.checkpoint()?;
            if format!("{:x}", hash.finalize()) != *expected {
                return Err(E::Stale);
            }
        }
        let (job, header, report) = recorded::decode(&bytes[0], control)?;
        if recorded::descriptor(job, &header, control)? != *recorded {
            return Err(E::Stale);
        }
        let selected = path(&header.source.reference)?;
        let native = path(&source_native.reference)?;
        for (album, observed) in header.snapshot.albums.iter().zip(&header.observed_sources) {
            control.checkpoint()?;
            let album_path = path(&album.source.reference)?;
            let relative = album_path
                .strip_prefix(&selected)
                .map_err(|_| E::CorruptJournal)?;
            if reference(&native.join(relative)) != observed.reference {
                return Err(E::Stale);
            }
        }
        let nested = read_nested(
            &header,
            &report,
            bytes.get(1),
            bytes.get(2),
            &source_native.reference,
            control,
        )?;
        (header, report, nested)
    };
    reader
        .verify_target_reference_observed(&header.observed_target, cancel, &mut |e| {
            inspection_event(control, e)
        })
        .map_err(inspection_error)?;
    let (sources, targets) = ledgers(&header, &report, nested.as_ref(), control)?;
    let source_indices = sources.indices.clone();
    let target_indices = targets.indices.clone();
    let owner = if present {
        reader.bind_present_ledgers_observed(
            sources.records.clone(),
            targets.records.clone(),
            cancel,
            &mut |e| inspection_event(control, e),
        )
    } else {
        reader.bind_targets_observed(targets.records.clone(), cancel, &mut |e| {
            inspection_event(control, e)
        })
    }
    .map_err(inspection_error)?;
    let (directories, pins, membership_pins) = pin_directories(
        &owner, &header, &report, &sources, &targets, present, control,
    )?;
    let mut membership = inspect_membership(&membership_pins, control)?;
    if let Some(root) = root_membership(&owner, &header, &report, present, control)? {
        membership.push(root);
    }
    let mut nested_rows = HashMap::new();
    if let Some((binding, snapshot, mut nested, physical)) = nested {
        if let Some(physical) = physical {
            let mut transfers = super::super::recorded::recover_deferred_transfers(
                &owner,
                &binding,
                &physical,
                source_indices,
                target_indices,
                control,
            )?;
            for operation in &mut transfers.operations {
                control.checkpoint()?;
                operation.operation = snapshot
                    .files
                    .get(operation.operation)
                    .ok_or(E::CorruptJournal)?
                    .original_operation;
                if nested_rows
                    .insert(operation.operation, operation.clone())
                    .is_some()
                {
                    return Err(E::CorruptJournal);
                }
            }
            nested.transfers = Some(transfers);
        }
        for directory in &mut nested.directories {
            control.checkpoint()?;
            if let Some(current) = directories.iter().find(|current| {
                current.reference == directory.reference
                    && current.expected_identity == directory.expected_identity
            }) {
                *directory = current.clone();
            }
        }
        report.singles = Some(Box::new(nested));
    }
    for album in &header.snapshot.albums {
        control.checkpoint()?;
        let actual = if album.source.reference == header.source.reference {
            Ok(match &original {
                WindowsAlbumOriginalLocation::OriginalPresent(value) => Some(value.clone()),
                _ => None,
            })
        } else {
            current(
                &owner,
                &sources,
                &album.source.reference,
                true,
                present,
                control,
            )
        };
        report.source_evidence.push(dir_evidence(
            album.source.reference.clone(),
            album.source.identity.clone().ok_or(E::CorruptJournal)?,
            actual,
        )?);
        let target = reference(&destination_path(&album.destination)?);
        report.destination_evidence.push(dir_evidence(
            target.clone(),
            album.source.identity.clone().ok_or(E::CorruptJournal)?,
            current(&owner, &targets, &target, false, present, control),
        )?);
    }
    for child in &recorded.child_locations {
        for (at_destination, reference) in [(false, &child.original), (true, &child.prepared)] {
            let (observed, error) =
                match current(&owner, &sources, reference, true, present, control) {
                    Ok(value) => (value, None),
                    Err(E::Cancelled) => return Err(E::Cancelled),
                    Err(error) => (None, Some(event_error(error))),
                };
            report.child_locations.push(AlbumChildLocationEvidence {
                operation: child.operation,
                planner_index: child.planner_index,
                at_destination,
                observed,
                error,
            });
        }
    }
    let album_count = header.snapshot.albums.len();
    let whole_files = recorded
        .files
        .iter()
        .filter(|file| file.operation < album_count)
        .collect::<Vec<_>>();
    let cursor = start_file.min(whole_files.len());
    let evidence = header
        .scope
        .inspected_sources
        .iter()
        .map(|s| (&s.observation.reference, s))
        .collect::<HashMap<_, _>>();
    control.operations(recorded.files.len())?;
    let mut main_rows = HashMap::new();
    for file in whole_files[cursor..].iter().chain(&whole_files[..cursor]) {
        control.begin_operation(file.inspection_index)?;
        let expected = evidence.get(&file.source).ok_or(E::CorruptJournal)?;
        let mut row = AlbumFileEvidence {
            operation: file.operation,
            reference: file.destination.clone(),
            expected_identity: expected
                .observation
                .identity
                .clone()
                .ok_or(E::CorruptJournal)?,
            observed_identity: None,
            status: "unverified".into(),
        };
        if let Some(index) = targets.indices.get(&file.destination) {
            match owner
                .inspect_main_observed(*index, cancel, &mut |e| inspection_event(control, e))
                .map_err(inspection_error)
            {
                Ok((observed, digest)) => {
                    row.observed_identity = Some(observed.identity.clone());
                    row.status = if observed.identity == row.expected_identity
                        && observed.size == expected.observation.size
                        && Some(digest.length) == expected.observation.size
                        && digest
                            .sha256
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                            == expected.sha256
                    {
                        "verified"
                    } else {
                        "changed"
                    }
                    .into();
                }
                Err(E::Cancelled) => return Err(E::Cancelled),
                Err(_) => {}
            }
        }
        main_rows.insert(file.inspection_index, row.clone());
        report.files.push(row);
        control.complete_operation()?;
    }
    for pin in &pins {
        pin.revalidate_observed(cancel, &mut |e| inspection_event(control, e))
            .map_err(inspection_error)?;
    }
    let mut after = inspect_membership(&membership_pins, control)?;
    if let Some(root) = root_membership(&owner, &header, &report, present, control)? {
        after.push(root);
    }
    for (before, after) in membership.iter_mut().zip(after) {
        if after.status != "verified" {
            before.status = after.status;
        }
    }
    owner
        .verify_target_reference_observed(&header.observed_target, cancel, &mut |e| {
            inspection_event(control, e)
        })
        .map_err(inspection_error)?;
    let final_source = owner
        .inspect_original_observed(cancel, &mut |e| inspection_event(control, e))
        .map_err(inspection_error)?;
    if final_source != original {
        return Err(E::Stale);
    }
    let mut files = Vec::new();
    for file in &recorded.files {
        control.checkpoint()?;
        files.push(WholeAlbumInspectionFile {
            file: file.clone(),
            main: main_rows.remove(&file.inspection_index),
            transfer: nested_rows.remove(&file.operation),
        });
    }
    if !main_rows.is_empty() || !nested_rows.is_empty() {
        return Err(E::CorruptJournal);
    }
    let source_state = if present {
        AlbumInspectionSourceState::Present {
            observed: source_native,
        }
    } else {
        AlbumInspectionSourceState::Absent {
            recorded: source_native,
        }
    };
    Ok(RecordedWholeAlbumInspection {
        recorded: recorded.clone(),
        source_state,
        recovery: report,
        files,
        directories,
        membership,
        content_scope: "recorded_main_hash_identity_membership",
    })
}

type Nested = (
    ExecutionBinding,
    Snapshot,
    SelectedAlbumRecovery,
    Option<PhysicalJournalV2>,
);
fn read_nested(
    header: &Header,
    whole: &WholeAlbumRecovery,
    bytes: Option<&Vec<u8>>,
    physical: Option<&Vec<u8>>,
    source_native: &StorageRef,
    control: &InspectionControl,
) -> Result<Option<Nested>> {
    let Some(bytes) = bytes else {
        if physical.is_some() || whole.recorded_singles.as_ref().is_some_and(|(_, ack)| *ack) {
            return Err(E::CorruptJournal);
        }
        return Ok(None);
    };
    let data = header.singles_binding.clone().ok_or(E::CorruptJournal)?;
    let job = header.singles_job_id.as_ref().ok_or(E::CorruptJournal)?;
    let files = header
        .snapshot
        .files
        .iter()
        .filter(|f| f.original_operation >= header.snapshot.albums.len())
        .cloned()
        .collect::<Vec<_>>();
    let mut destinations = Vec::new();
    for requirement in &header.snapshot.destinations {
        control.checkpoint()?;
        if !files.iter().any(|f| {
            f.destination
                .components
                .starts_with(&requirement.destination.components)
                && f.destination.components.len() > requirement.destination.components.len()
        }) {
            continue;
        }
        let mut value = requirement.clone();
        if value.existing.is_none() {
            let reference = reference(&destination_path(&value.destination)?);
            if let Some(created) = whole
                .created_directories
                .iter()
                .find(|o| o.reference == reference)
            {
                value.existing = Some(EntryObservation {
                    reference: created.reference.clone(),
                    name: value
                        .destination
                        .components
                        .last()
                        .cloned()
                        .ok_or(E::CorruptJournal)?,
                    identity: Some(created.identity.clone()),
                    parent_identity: Some(created.parent_identity.clone()),
                    kind: EntryKind::Directory,
                    size: None,
                    modified: None,
                    capabilities: Capabilities {
                        list: Some(true),
                        read: Some(false),
                        seek: Some(false),
                    },
                });
            }
        }
        destinations.push(value);
    }
    let snapshot = Snapshot {
        copied_directories: vec![],
        directories: vec![],
        destinations,
        files,
        albums: vec![],
    };
    let binding = ExecutionBinding {
        data,
        recovery_only: true,
        execution_decisions: None,
    };
    let (mut report, observed) = super::super::file_manifest::FileManifestEvidence {
        binding: &binding,
        snapshot: &snapshot,
        job_id: job,
        observed_source_root: source_native,
        original_source: &None,
    }
    .read_manifest(bytes, control)?;
    report.directories = observed
        .into_iter()
        .map(|(reference, identity)| AlbumDirectoryEvidence {
            reference,
            expected_identity: identity,
            observed_identity: None,
            status: AlbumDirectoryEvidenceStatus::Unverified,
        })
        .collect();
    if whole
        .recorded_singles
        .as_ref()
        .is_some_and(|(status, ack)| *ack && report.recorded_status.as_ref() != Some(status))
    {
        return Err(E::CorruptJournal);
    }
    let physical = physical
        .map(|bytes| PhysicalJournalV2::open_controlled(&binding, job, bytes, control))
        .transpose()?;
    super::super::recorded::validate_transfer_receipts(
        &snapshot,
        &binding,
        &report,
        physical.as_ref(),
        control,
    )?;
    Ok(Some((binding, snapshot, report, physical)))
}
fn ledgers(
    header: &Header,
    report: &WholeAlbumRecovery,
    nested: Option<&Nested>,
    control: &InspectionControl,
) -> Result<(Ledger, Ledger)> {
    let mut sources = Ledger::default();
    let mut targets = Ledger::default();
    for entry in header
        .snapshot
        .directories
        .iter()
        .map(|d| &d.source)
        .chain(header.snapshot.directories.iter().flat_map(|d| &d.children))
        .chain(header.snapshot.files.iter().map(|f| &f.source))
    {
        control.checkpoint()?;
        sources.insert(
            &header.source.reference,
            entry.reference.clone(),
            entry.identity.clone(),
            entry.parent_identity.clone().ok_or(E::CorruptJournal)?,
            entry.kind,
        )?;
        if let Some(album) = header.snapshot.albums.iter().find(|album| {
            path(&entry.reference)
                .is_ok_and(|p| path(&album.source.reference).is_ok_and(|a| p.starts_with(a)))
        }) {
            sources.insert(
                &header.source.reference,
                mapped(album, &entry.reference, false)?,
                entry.identity.clone(),
                entry.parent_identity.clone().ok_or(E::CorruptJournal)?,
                entry.kind,
            )?;
        }
    }
    let mut directory_ids = HashMap::from([(
        header.target.reference.clone(),
        header.target.identity.clone().ok_or(E::CorruptJournal)?,
    )]);
    for requirement in &header.snapshot.destinations {
        control.checkpoint()?;
        if let Some(existing) = &requirement.existing {
            directory_ids.insert(
                existing.reference.clone(),
                existing.identity.clone().ok_or(E::CorruptJournal)?,
            );
        }
    }
    let selected = path(&header.target.reference)?;
    for created in &report.created_directories {
        control.checkpoint()?;
        // Whole records created parents in the selected spelling, which the
        // strict phase reader already compared with the exact intended path.
        if !path(&created.reference)?.starts_with(&selected) {
            return Err(E::CorruptJournal);
        }
        directory_ids.insert(created.reference.clone(), created.identity.clone());
    }
    if let Some((_, _, nested, _)) = nested {
        for directory in &nested.directories {
            control.checkpoint()?;
            directory_ids.insert(
                directory.reference.clone(),
                directory.expected_identity.clone(),
            );
        }
    }
    for album in &header.snapshot.albums {
        control.checkpoint()?;
        directory_ids.insert(
            reference(&destination_path(&album.destination)?),
            album.source.identity.clone().ok_or(E::CorruptJournal)?,
        );
        for directory in &header.snapshot.directories {
            if path(&directory.source.reference)?.starts_with(path(&album.source.reference)?) {
                directory_ids.insert(
                    mapped(album, &directory.source.reference, true)?,
                    directory.source.identity.clone().ok_or(E::CorruptJournal)?,
                );
            }
        }
    }
    // Only bind descendants whose complete parent identity chain is recorded.
    let mut directory_entries = directory_ids
        .iter()
        .map(|(r, id)| (r.clone(), id.clone()))
        .collect::<Vec<_>>();
    directory_entries.sort_by_key(|(r, _)| {
        path(r)
            .map(|p| p.components().count())
            .unwrap_or(usize::MAX)
    });
    let mut admitted = HashSet::from([header.target.reference.clone()]);
    for (full, id) in directory_entries {
        control.checkpoint()?;
        if full == header.target.reference {
            continue;
        }
        let parent = reference(path(&full)?.parent().ok_or(E::CorruptJournal)?);
        if admitted.contains(&parent) {
            targets.insert(
                &header.target.reference,
                full.clone(),
                Some(id),
                directory_ids.get(&parent).ok_or(E::CorruptJournal)?.clone(),
                EntryKind::Directory,
            )?;
            admitted.insert(full);
        }
    }
    let mut published = HashMap::new();
    if let Some((_, snapshot, _, Some(journal))) = nested {
        for event in journal.events() {
            control.checkpoint()?;
            if matches!(
                event.phase,
                crate::physical_journal::Phase::DestinationPublished
                    | crate::physical_journal::Phase::AtomicRenameObserved
            ) {
                let file = snapshot
                    .files
                    .get(event.operation)
                    .ok_or(E::CorruptJournal)?;
                published.insert(
                    file.original_operation,
                    event.observed_identity.clone().ok_or(E::CorruptJournal)?,
                );
            }
        }
    }
    for file in &header.snapshot.files {
        control.checkpoint()?;
        let full = destination_path(&file.destination)?;
        let parent = reference(full.parent().ok_or(E::CorruptJournal)?);
        if !admitted.contains(&parent) {
            continue;
        }
        let identity = if file.original_operation < header.snapshot.albums.len() {
            file.source.identity.clone()
        } else {
            published.get(&file.original_operation).cloned()
        };
        targets.insert(
            &header.target.reference,
            reference(&full),
            identity,
            directory_ids.get(&parent).ok_or(E::CorruptJournal)?.clone(),
            EntryKind::File,
        )?;
    }
    Ok((sources, targets))
}
fn pin_directories(
    owner: &WindowsAlbumReadInspection,
    header: &Header,
    whole: &WholeAlbumRecovery,
    sources: &Ledger,
    targets: &Ledger,
    present: bool,
    control: &InspectionControl,
) -> Result<(
    Vec<AlbumDirectoryEvidence>,
    Vec<WindowsAlbumInspectionDirectory>,
    Vec<MembershipPin>,
)> {
    let mut report = Vec::new();
    let mut pins = Vec::new();
    let mut members = Vec::new();
    for (source, ledger) in [(true, sources), (false, targets)] {
        for (index, record) in ledger.records.iter().enumerate() {
            control.checkpoint()?;
            if record.kind != EntryKind::Directory {
                continue;
            }
            let expected = record.identity.clone().ok_or(E::CorruptJournal)?;
            let actual = current(owner, ledger, &record.reference, source, present, control);
            let evidence = dir_evidence(record.reference.clone(), expected, actual)?;
            if matches!(evidence.status, AlbumDirectoryEvidenceStatus::SameIdentity) {
                let result = if source {
                    owner.pin_source_directory_observed(
                        index,
                        control.cancellation_flag(),
                        &mut |e| inspection_event(control, e),
                    )
                } else {
                    owner.pin_directory_observed(index, control.cancellation_flag(), &mut |e| {
                        inspection_event(control, e)
                    })
                };
                let pin = result.map_err(inspection_error)?;
                let expected = header.snapshot.directories.iter().find_map(|directory| {
                    header.snapshot.albums.iter().find_map(|album| {
                        if !path(&directory.source.reference)
                            .ok()?
                            .starts_with(path(&album.source.reference).ok()?)
                        {
                            return None;
                        }
                        let mapped_ref =
                            mapped(album, &directory.source.reference, !source).ok()?;
                        if record.reference == directory.source.reference && source
                            || record.reference == mapped_ref
                        {
                            Some((directory, album))
                        } else {
                            None
                        }
                    })
                });
                if let Some((directory, album)) = expected {
                    let mut children = Vec::new();
                    for child in &directory.children {
                        control.checkpoint()?;
                        let use_prepared = !source
                            || record.reference != directory.source.reference
                            || album.children.iter().enumerate().any(|(index, direct)| {
                                direct.source.reference == child.reference
                                    && whole.child_renames.iter().any(|receipt| {
                                        receipt.operation == album.operation
                                            && receipt.planner_index == index
                                    })
                            });
                        let child_ref = if use_prepared {
                            mapped(album, &child.reference, !source)?
                        } else {
                            child.reference.clone()
                        };
                        children.push(WindowsDirectoryMember {
                            name: MutationName::Windows(
                                path(&child_ref)?
                                    .file_name()
                                    .ok_or(E::CorruptJournal)?
                                    .encode_wide()
                                    .collect(),
                            ),
                            identity: child.identity.clone().ok_or(E::CorruptJournal)?,
                            kind: child.kind,
                        });
                    }
                    members.push(MembershipPin {
                        pin,
                        reference: record.reference.clone(),
                        expected: children,
                    });
                } else {
                    pins.push(pin);
                }
            }
            report.push(evidence);
        }
    }
    Ok((report, pins, members))
}

fn root_membership(
    owner: &WindowsAlbumReadInspection,
    header: &Header,
    report: &WholeAlbumRecovery,
    present: bool,
    control: &InspectionControl,
) -> Result<Option<WholeAlbumMembershipEvidence>> {
    if !present {
        return Ok(None);
    }
    let Some(album) = header
        .snapshot
        .albums
        .iter()
        .find(|a| a.source.reference == header.source.reference)
    else {
        return Ok(None);
    };
    let mut expected = Vec::new();
    for (index, child) in album.children.iter().enumerate() {
        control.checkpoint()?;
        let renamed = report
            .child_renames
            .iter()
            .any(|r| r.operation == album.operation && r.planner_index == index);
        expected.push(WindowsDirectoryMember {
            name: if renamed {
                name(&child.destination_name)
            } else {
                leaf(&child.source)?
            },
            identity: child.source.identity.clone().ok_or(E::CorruptJournal)?,
            kind: child.source.kind,
        });
    }
    let status = match owner
        .source_root_members_observed(control.cancellation_flag(), &mut |e| {
            inspection_event(control, e)
        })
        .map_err(inspection_error)
    {
        Ok(actual) => {
            if same_members(actual, expected) {
                "verified"
            } else {
                "changed"
            }
        }
        Err(E::Cancelled) => return Err(E::Cancelled),
        Err(_) => "unverified",
    };
    Ok(Some(WholeAlbumMembershipEvidence {
        reference: header.source.reference.clone(),
        status: status.into(),
    }))
}
