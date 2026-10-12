//! Read-only target/journal owner for reopening a recorded Album with no source
//! root grant. The original containing parent never becomes a storage root.
use super::*;
use crate::storage::EntryObservation;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct WindowsAlbumInspectionJournal {
    pub name: MutationName,
    pub identity: ObjectIdentity,
    pub length: u64,
}

/// Data validated by strict recorded replay, never a directory capability.
#[derive(Clone, Debug)]
pub struct WindowsAlbumInspectionTarget {
    pub reference: StorageRef,
    pub identity: Option<ObjectIdentity>,
    pub parent_identity: ObjectIdentity,
    pub kind: EntryKind,
}

pub struct WindowsAlbumReadInspectionInputs {
    pub target: WindowsSelectedRootGrant,
    pub journal: WindowsSelectedRootGrant,
    pub recorded_target: EntryObservation,
    pub journals: Vec<WindowsAlbumInspectionJournal>,
    pub targets: Vec<WindowsAlbumInspectionTarget>,
}

/// Initial read-only state. There is no target ledger or target lookup method.
pub struct WindowsAlbumJournalInspectionInputs {
    pub target: WindowsSelectedRootGrant,
    pub journal: WindowsSelectedRootGrant,
    pub recorded_target: EntryObservation,
    pub journals: Vec<WindowsAlbumInspectionJournal>,
}

/// A separately selected, still-present source root. The nested inputs retain
/// the same fixed journal selection and independent target grant as absent jobs.
pub struct WindowsAlbumPresentJournalInspectionInputs {
    pub source: WindowsSelectedRootGrant,
    pub recorded_source: EntryObservation,
    pub destination: WindowsAlbumJournalInspectionInputs,
}

pub struct WindowsAlbumJournalInspection {
    owner: WindowsAlbumReadInspection,
    replayed: Option<Vec<Vec<u8>>>,
}

impl WindowsAlbumJournalInspection {
    pub fn open_present_observed(
        inputs: WindowsAlbumPresentJournalInspectionInputs,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        cancelled(cancel)?;
        let selected_source = path_of(&inputs.recorded_source.reference)?;
        if selected_source != inputs.source.path
            || inputs.recorded_source.kind != EntryKind::Directory
            || inputs.recorded_source.identity.as_ref() != Some(&inputs.source.expected_identity)
            || inputs.recorded_source.size.is_some()
        {
            return Err(stale("recorded source differs from fresh source selection"));
        }
        let source = Arc::new(ReadRoot::open(inputs.source, cancel, progress)?);
        if Some(&source.observed.parent_identity) != inputs.recorded_source.parent_identity.as_ref()
        {
            return Err(stale("recorded source parent changed"));
        }
        let original = OriginalReadState::Present {
            source,
            selected_source,
        };
        let inputs = inputs.destination;
        let owner = WindowsAlbumReadInspection::open_state_observed(
            WindowsAlbumReadInspectionInputs {
                target: inputs.target,
                journal: inputs.journal,
                recorded_target: inputs.recorded_target,
                journals: inputs.journals,
                targets: vec![],
            },
            original,
            cancel,
            progress,
        )?;
        Ok(Self {
            owner,
            replayed: None,
        })
    }

    pub fn open_observed(
        inputs: WindowsAlbumJournalInspectionInputs,
        original: WindowsAlbumOriginalLocationInspection,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        let owner = WindowsAlbumReadInspection::open_observed(
            WindowsAlbumReadInspectionInputs {
                target: inputs.target,
                journal: inputs.journal,
                recorded_target: inputs.recorded_target,
                journals: inputs.journals,
                targets: vec![],
            },
            original,
            cancel,
            progress,
        )?;
        Ok(Self {
            owner,
            replayed: None,
        })
    }

    /// Read the fixed journal set once. Returned bytes are immutable historical
    /// input for strict replay, never current evidence after an unchecked reopen.
    pub fn read_journals_observed(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<&[Vec<u8>], MutationError> {
        cancelled(cancel)?;
        if self.replayed.is_some() {
            return Err(stale("journal set already read for replay"));
        }
        let mut bytes = Vec::new();
        for index in 0..self.owner.journals.len() {
            cancelled(cancel)?;
            bytes.push(self.owner.read_journal_observed(index, cancel, progress)?);
        }
        cancelled(cancel)?;
        self.replayed = Some(bytes);
        Ok(self
            .replayed
            .as_deref()
            .expect("complete journal set retained"))
    }

    pub fn inspect_original_observed(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumOriginalLocation, MutationError> {
        self.owner.inspect_original_observed(cancel, progress)
    }

    /// Consume this state after strict executor replay supplies its validated
    /// target ledger. Every journal is freshly read again and compared exactly.
    /// The returned type has no rebinding method, even for an empty ledger.
    pub fn bind_targets_observed(
        self,
        targets: Vec<WindowsAlbumInspectionTarget>,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumReadInspection, MutationError> {
        if !matches!(&self.owner.original, OriginalReadState::Absent(_)) {
            return Err(stale(
                "present-source inspection requires its separate source ledger",
            ));
        }
        self.bind_ledgers_observed(None, targets, cancel, progress)
    }

    pub fn bind_present_ledgers_observed(
        self,
        sources: Vec<WindowsAlbumInspectionTarget>,
        targets: Vec<WindowsAlbumInspectionTarget>,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumReadInspection, MutationError> {
        if !matches!(&self.owner.original, OriginalReadState::Present { .. }) {
            return Err(stale(
                "absent-source inspection cannot acquire a source ledger",
            ));
        }
        self.bind_ledgers_observed(Some(sources), targets, cancel, progress)
    }

    fn bind_ledgers_observed(
        mut self,
        sources: Option<Vec<WindowsAlbumInspectionTarget>>,
        targets: Vec<WindowsAlbumInspectionTarget>,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumReadInspection, MutationError> {
        cancelled(cancel)?;
        let replayed = self.replayed.take().ok_or_else(|| {
            stale("journal bytes must be read and replayed before target binding")
        })?;
        inspection_probe(cancel, progress, || {
            self.owner.ledger.target.validate(cancel)
        })?;
        inspection_probe(cancel, progress, || self.owner.journal.validate(cancel))?;
        self.owner.original.validate_observed(cancel, progress)?;
        for (index, expected) in replayed.iter().enumerate() {
            cancelled(cancel)?;
            let current = self.owner.read_journal_observed(index, cancel, progress)?;
            if current != *expected {
                return Err(stale("journal contents changed since strict replay"));
            }
        }
        let (targets, directory_ids) = checked_ledger(
            targets,
            &self.owner.ledger.selected_target,
            &self.owner.ledger.target.observed.identity,
            cancel,
        )?;
        inspection_probe(cancel, progress, || {
            self.owner.ledger.target.validate(cancel)
        })?;
        inspection_probe(cancel, progress, || self.owner.journal.validate(cancel))?;
        self.owner.original.validate_observed(cancel, progress)?;
        cancelled(cancel)?;
        self.owner.source_ledger = match (&self.owner.original, sources) {
            (OriginalReadState::Absent(_), None) => None,
            (
                OriginalReadState::Present {
                    source,
                    selected_source,
                },
                Some(records),
            ) => {
                let (targets, directory_ids) =
                    checked_ledger(records, selected_source, &source.observed.identity, cancel)?;
                Some(ReadLedger {
                    selected_target: selected_source.clone(),
                    target: source.clone(),
                    targets,
                    directory_ids,
                })
            }
            _ => return Err(stale("inspection source state cannot change")),
        };
        self.owner.original.validate_observed(cancel, progress)?;
        cancelled(cancel)?;
        self.owner.ledger.targets = targets;
        self.owner.ledger.directory_ids = directory_ids;
        Ok(self.owner)
    }
}

struct ReadRoot {
    handles: Vec<File>,
    ancestry: Vec<(ObjectIdentity, StorageRef)>,
    observed: MutationObservation,
}

impl ReadRoot {
    fn open(
        grant: WindowsSelectedRootGrant,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        cancelled(cancel)?;
        validate_root_path(&grant.path)?;
        let handles = read_parent_chain(&grant.path, cancel, progress)?;
        let mut ancestry = Vec::new();
        for file in &handles {
            ancestry.push((
                inspection_probe(cancel, progress, || identity(file).map_err(read_error))?,
                observed_reference(file, cancel, progress)?,
            ));
        }
        let directory = handles
            .last()
            .ok_or_else(|| stale("read root has no handle"))?;
        let parent = handles
            .iter()
            .rev()
            .nth(1)
            .ok_or_else(|| stale("read root has no parent"))?;
        let parent_id =
            inspection_probe(cancel, progress, || identity(parent).map_err(read_error))?;
        let observed = read_observation(directory, &parent_id, cancel, progress)?;
        if observed.identity != grant.expected_identity || observed.size.is_some() {
            return Err(stale("freshly selected read root changed"));
        }
        require_local_ntfs(directory, cancel, progress)?;
        Ok(Self {
            handles,
            ancestry,
            observed,
        })
    }

    fn handle(&self) -> &File {
        self.handles.last().expect("validated read root")
    }

    fn validate(&self, cancel: &AtomicBool) -> Result<(), MutationError> {
        for (file, (expected_id, expected_path)) in self.handles.iter().zip(&self.ancestry) {
            cancelled(cancel)?;
            ensure_directory(file)?;
            if identity(file).map_err(read_error)? != *expected_id
                || observed_reference(file, cancel, &mut |_| Ok(()))? != *expected_path
            {
                return Err(stale("held read root ancestry moved or changed"));
            }
            cancelled(cancel)?;
        }
        let actual = read_observation(
            self.handle(),
            &self.observed.parent_identity,
            cancel,
            &mut |_| Ok(()),
        )?;
        if actual != self.observed {
            return Err(stale("held read root moved or changed"));
        }
        cancelled(cancel)
    }
}

struct CheckedTarget {
    recorded: WindowsAlbumInspectionTarget,
    relative: PathBuf,
}

/// Contains no WindowsMutationStorage or mutation session. All lookup names are
/// fixed by the retained journal list and validated target ledger at creation.
pub struct WindowsAlbumReadInspection {
    original: OriginalReadState,
    ledger: ReadLedger,
    source_ledger: Option<ReadLedger>,
    journal: Arc<ReadRoot>,
    journals: Vec<WindowsAlbumInspectionJournal>,
}

/// A fixed index space for exactly one held root. A source ledger and a target
/// ledger never share relative-path maps or reinterpret one another's indices.
struct ReadLedger {
    selected_target: PathBuf,
    target: Arc<ReadRoot>,
    targets: Vec<CheckedTarget>,
    directory_ids: HashMap<PathBuf, ObjectIdentity>,
}

enum OriginalReadState {
    Absent(WindowsAlbumOriginalLocationInspection),
    Present {
        source: Arc<ReadRoot>,
        selected_source: PathBuf,
    },
}
impl OriginalReadState {
    fn observation(&self) -> &MutationObservation {
        match self {
            Self::Absent(original) => &original.original,
            Self::Present { source, .. } => &source.observed,
        }
    }
    fn inspect_observed(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumOriginalLocation, MutationError> {
        match self {
            Self::Absent(original) => original.inspect_original_observed(cancel, progress),
            Self::Present { source, .. } => {
                inspection_probe(cancel, progress, || source.validate(cancel))?;
                Ok(WindowsAlbumOriginalLocation::OriginalPresent(
                    source.observed.clone(),
                ))
            }
        }
    }
    fn validate_observed(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<(), MutationError> {
        let actual = self.inspect_observed(cancel, progress)?;
        match (self, actual) {
            (Self::Absent(_), WindowsAlbumOriginalLocation::Absent) => Ok(()),
            (
                Self::Present { source, .. },
                WindowsAlbumOriginalLocation::OriginalPresent(actual),
            ) if actual == source.observed => Ok(()),
            _ => Err(stale(
                "original source state changed during read-only inspection",
            )),
        }
    }
}

struct ReadParent {
    root: Arc<ReadRoot>,
    handles: Vec<File>,
    observations: Vec<MutationObservation>,
}

impl ReadParent {
    fn handle(&self) -> &File {
        self.handles.last().unwrap_or_else(|| self.root.handle())
    }
    fn identity(&self) -> &ObjectIdentity {
        self.observations
            .last()
            .map_or(&self.root.observed.identity, |value| &value.identity)
    }
    fn validate(&self, cancel: &AtomicBool) -> Result<(), MutationError> {
        self.root.validate(cancel)?;
        for (file, expected) in self.handles.iter().zip(&self.observations) {
            cancelled(cancel)?;
            ensure_directory(file)?;
            let actual =
                read_observation(file, &expected.parent_identity, cancel, &mut |_| Ok(()))?;
            if actual != *expected {
                return Err(stale("held target parent moved or changed"));
            }
        }
        cancelled(cancel)
    }
}

/// Retains the exact directory and its ancestry through descendant observations.
/// Revalidate before using the complete inspection report; no raw handle escapes.
pub struct WindowsAlbumInspectionDirectory {
    parent: ReadParent,
    file: File,
    observed: MutationObservation,
}

impl WindowsAlbumInspectionDirectory {
    pub fn observation(&self) -> &MutationObservation {
        &self.observed
    }
    pub fn revalidate_observed(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<(), MutationError> {
        inspection_probe(cancel, progress, || self.parent.validate(cancel))?;
        let actual = read_observation(&self.file, self.parent.identity(), cancel, progress)?;
        if actual != self.observed {
            return Err(stale("held target directory moved or changed"));
        }
        cancelled(cancel)
    }
}

impl WindowsAlbumReadInspection {
    pub fn open_observed(
        inputs: WindowsAlbumReadInspectionInputs,
        original: WindowsAlbumOriginalLocationInspection,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        Self::open_state_observed(
            inputs,
            OriginalReadState::Absent(original),
            cancel,
            progress,
        )
    }

    fn open_state_observed(
        inputs: WindowsAlbumReadInspectionInputs,
        original: OriginalReadState,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        cancelled(cancel)?;
        original.validate_observed(cancel, progress)?;
        if inputs.journals.is_empty() || inputs.journals.len() > 8 || inputs.targets.len() > 12_288
        {
            return Err(err(
                MutationErrorKind::Unsupported,
                "Album inspection ledger exceeds limits",
            ));
        }
        let selected = path_of(&inputs.recorded_target.reference)?;
        if selected != inputs.target.path
            || inputs.recorded_target.kind != EntryKind::Directory
            || inputs.recorded_target.identity.as_ref() != Some(&inputs.target.expected_identity)
        {
            return Err(stale("recorded target differs from fresh target selection"));
        }
        let target = Arc::new(ReadRoot::open(inputs.target, cancel, progress)?);
        let journal = Arc::new(ReadRoot::open(inputs.journal, cancel, progress)?);
        if target.observed.parent_identity
            != inputs
                .recorded_target
                .parent_identity
                .ok_or_else(|| stale("recorded target parent missing"))?
        {
            return Err(stale("recorded target parent changed"));
        }
        let target_path = path_of(&target.observed.reference)?;
        let journal_path = path_of(&journal.observed.reference)?;
        let source_path = path_of(&original.observation().reference)?;
        if overlaps(&target_path, &journal_path)
            || overlaps(&target_path, &source_path)
            || overlaps(&journal_path, &source_path)
            || target.observed.identity == journal.observed.identity
            || target.observed.identity == original.observation().identity
            || journal.observed.identity == original.observation().identity
        {
            return Err(stale("Album read roots overlap"));
        }
        let mut names = HashSet::new();
        for record in &inputs.journals {
            cancelled(cancel)?;
            let name = units(&record.name)?;
            if !name.ends_with(&".jsonl".encode_utf16().collect::<Vec<_>>())
                || !names.insert(name.to_vec())
                || record.length > 64 * 1024 * 1024
                || !matches!(record.identity, ObjectIdentity::Windows { .. })
            {
                return Err(stale("invalid or repeated retained Album journal"));
            }
        }
        let (targets, directory_ids) =
            checked_ledger(inputs.targets, &selected, &target.observed.identity, cancel)?;
        original.validate_observed(cancel, progress)?;
        cancelled(cancel)?;
        Ok(Self {
            original,
            source_ledger: None,
            ledger: ReadLedger {
                selected_target: selected,
                target,
                targets,
                directory_ids,
            },
            journal,
            journals: inputs.journals,
        })
    }

    pub fn inspect_original_observed(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumOriginalLocation, MutationError> {
        self.original.inspect_observed(cancel, progress)
    }

    pub fn read_journal_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Vec<u8>, MutationError> {
        let record = self
            .journals
            .get(index)
            .ok_or_else(|| stale("unbound journal index"))?;
        inspection_probe(cancel, progress, || self.journal.validate(cancel))?;
        let mut file = inspection_probe(cancel, progress, || {
            open_relative_file(
                self.journal.handle(),
                units(&record.name)?,
                false,
                FILE_GENERIC_READ,
            )
        })?;
        inspection_probe(cancel, progress, || ensure_regular(&file))?;
        let observed = read_observation(&file, &self.journal.observed.identity, cancel, progress)?;
        let before = inspection_probe(cancel, progress, || repair_basic(&file))?;
        if observed.identity != record.identity
            || observed.size != Some(record.length)
            || observed.actual_name != record.name
            || observed.reference
                != child_reference(&self.journal.observed.reference, &record.name)?
        {
            return Err(stale("retained journal identity/length changed"));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(record.length as usize)
            .map_err(|_| err(MutationErrorKind::Io, "journal allocation failed"))?;
        let mut chunk = [0u8; 65_536];
        loop {
            cancelled(cancel)?;
            let remaining = record.length as usize - bytes.len();
            let amount = remaining.min(chunk.len()).max(1);
            let read = file.read(&mut chunk[..amount]);
            if let Ok(count) = &read {
                let notification = progress(WindowsInspectionEvent::BytesRead(*count));
                cancelled(cancel)?;
                notification?;
            }
            cancelled(cancel)?;
            let count = read.map_err(io_error)?;
            if count == 0 {
                break;
            }
            if count > remaining {
                return Err(stale("journal grew during inspection"));
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
        if bytes.len() as u64 != record.length
            || read_observation(&file, &self.journal.observed.identity, cancel, progress)?
                != observed
            || !same_basic(
                &before,
                &inspection_probe(cancel, progress, || repair_basic(&file))?,
            )
        {
            return Err(stale("journal changed during inspection"));
        }
        inspection_probe(cancel, progress, || self.journal.validate(cancel))?;
        cancelled(cancel)?;
        Ok(bytes)
    }

    pub fn inspect_source_entry_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<MutationObservation>, MutationError> {
        self.source_ledger
            .as_ref()
            .ok_or_else(|| stale("no present-source ledger was bound"))?
            .inspect_entry_observed(index, cancel, progress)
    }

    pub fn inspect_entry_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<MutationObservation>, MutationError> {
        self.ledger.inspect_entry_observed(index, cancel, progress)
    }

    pub fn pin_source_directory_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumInspectionDirectory, MutationError> {
        self.source_ledger
            .as_ref()
            .ok_or_else(|| stale("no present-source ledger was bound"))?
            .pin_directory_observed(index, cancel, progress)
    }

    pub fn inspect_source_metadata_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<WindowsCopyMetadataObservation>, MutationError> {
        self.source_ledger
            .as_ref()
            .ok_or_else(|| stale("no present-source ledger was bound"))?
            .inspect_metadata_observed(index, cancel, progress)
    }

    pub fn inspect_source_main_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<(MutationObservation, ContentDigest), MutationError> {
        self.source_ledger
            .as_ref()
            .ok_or_else(|| stale("no present-source ledger was bound"))?
            .inspect_main_observed(index, cancel, progress)
    }

    pub fn inspect_source_streams_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        self.source_ledger
            .as_ref()
            .ok_or_else(|| stale("no present-source ledger was bound"))?
            .inspect_streams_observed(index, cancel, progress)
    }

    pub fn pin_directory_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumInspectionDirectory, MutationError> {
        self.ledger.pin_directory_observed(index, cancel, progress)
    }

    pub fn inspect_metadata_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<WindowsCopyMetadataObservation>, MutationError> {
        self.ledger
            .inspect_metadata_observed(index, cancel, progress)
    }

    pub fn inspect_main_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<(MutationObservation, ContentDigest), MutationError> {
        self.ledger.inspect_main_observed(index, cancel, progress)
    }

    pub fn inspect_streams_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        self.ledger
            .inspect_streams_observed(index, cancel, progress)
    }
}

impl ReadLedger {
    fn parent(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<ReadParent, MutationError> {
        self.parent_if_present(index, cancel, progress)?
            .ok_or_else(|| stale("bound inspection parent is absent"))
    }

    fn parent_if_present(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<ReadParent>, MutationError> {
        let target = self
            .targets
            .get(index)
            .ok_or_else(|| stale("unbound target index"))?;
        inspection_probe(cancel, progress, || self.target.validate(cancel))?;
        let mut parent = ReadParent {
            root: self.target.clone(),
            handles: Vec::new(),
            observations: Vec::new(),
        };
        let mut relative = PathBuf::new();
        let mut native = path_of(&self.target.observed.reference)?;
        for component in target
            .relative
            .parent()
            .ok_or_else(|| stale("target parent missing"))?
            .components()
        {
            let Component::Normal(value) = component else {
                return Err(stale("invalid target parent"));
            };
            relative.push(value);
            native.push(value);
            let expected = self
                .directory_ids
                .get(&relative)
                .ok_or_else(|| stale("unbound target ancestry"))?;
            let file =
                inspection_probe(cancel, progress, || {
                    match super::super::super::open_child(
                        parent.handle(),
                        &value.encode_wide().collect::<Vec<_>>(),
                        false,
                    ) {
                        Ok(file) => Ok(Some(file)),
                        Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => {
                            Ok(None)
                        }
                        Err(error) => Err(read_error(error)),
                    }
                })?;
            let Some(file) = file else {
                inspection_probe(cancel, progress, || parent.validate(cancel))?;
                return Ok(None);
            };
            inspection_probe(cancel, progress, || ensure_directory(&file))?;
            let observed = read_observation(&file, parent.identity(), cancel, progress)?;
            if observed.identity != *expected || observed.reference != reference_of(&native) {
                return Err(stale("recorded target ancestry changed"));
            }
            parent.handles.push(file);
            parent.observations.push(observed);
        }
        if parent.identity() != &target.recorded.parent_identity {
            return Err(stale("target parent differs from replayed evidence"));
        }
        inspection_probe(cancel, progress, || parent.validate(cancel))?;
        Ok(Some(parent))
    }

    fn inspect_entry_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<MutationObservation>, MutationError> {
        let Some(parent) = self.parent_if_present(index, cancel, progress)? else {
            return Ok(None);
        };
        let name = self.leaf(index)?;
        let file = inspection_probe(cancel, progress, || {
            match super::super::super::open_child(parent.handle(), units(&name)?, false) {
                Ok(file) => Ok(Some(file)),
                Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(None),
                Err(error) => Err(read_error(error)),
            }
        })?;
        let actual = file
            .as_ref()
            .map(|file| read_observation(file, parent.identity(), cancel, progress))
            .transpose()?;
        if let Some(actual) = &actual {
            self.validate_target(index, actual)?;
        }
        inspection_probe(cancel, progress, || parent.validate(cancel))?;
        cancelled(cancel)?;
        Ok(actual)
    }

    fn leaf(&self, index: usize) -> Result<MutationName, MutationError> {
        let target = self
            .targets
            .get(index)
            .ok_or_else(|| stale("unbound target index"))?;
        Ok(MutationName::Windows(
            target
                .relative
                .file_name()
                .ok_or_else(|| stale("target leaf missing"))?
                .encode_wide()
                .collect(),
        ))
    }

    fn validate_target(
        &self,
        index: usize,
        actual: &MutationObservation,
    ) -> Result<(), MutationError> {
        let target = self
            .targets
            .get(index)
            .ok_or_else(|| stale("unbound target index"))?;
        let expected_path = path_of(&self.target.observed.reference)?.join(&target.relative);
        if actual.reference != reference_of(&expected_path)
            || actual.actual_name != self.leaf(index)?
            || actual.parent_identity != target.recorded.parent_identity
            || target
                .recorded
                .identity
                .as_ref()
                .is_some_and(|expected| expected != &actual.identity)
            || (actual.size.is_none()) != (target.recorded.kind == EntryKind::Directory)
        {
            return Err(stale("target observation differs from the replayed ledger"));
        }
        Ok(())
    }

    pub fn pin_directory_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumInspectionDirectory, MutationError> {
        let target = self
            .targets
            .get(index)
            .ok_or_else(|| stale("unbound target index"))?;
        if target.recorded.kind != EntryKind::Directory || target.recorded.identity.is_none() {
            return Err(stale("directory pin requires recorded identity"));
        }
        let parent = self.parent(index, cancel, progress)?;
        let name = self.leaf(index)?;
        let file = inspection_probe(cancel, progress, || {
            super::super::super::open_child(parent.handle(), units(&name)?, false)
                .map_err(read_error)
        })?;
        inspection_probe(cancel, progress, || ensure_directory(&file))?;
        let observed = read_observation(&file, parent.identity(), cancel, progress)?;
        self.validate_target(index, &observed)?;
        let pin = WindowsAlbumInspectionDirectory {
            parent,
            file,
            observed,
        };
        pin.revalidate_observed(cancel, progress)?;
        Ok(pin)
    }

    pub fn inspect_metadata_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Option<WindowsCopyMetadataObservation>, MutationError> {
        let parent = self.parent(index, cancel, progress)?;
        let name = self.leaf(index)?;
        let file = inspection_probe(cancel, progress, || {
            match super::super::super::open_child(parent.handle(), units(&name)?, false) {
                Ok(file) => Ok(Some(file)),
                Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(None),
                Err(error) => Err(read_error(error)),
            }
        })?;
        let Some(file) = file else {
            inspection_probe(cancel, progress, || parent.validate(cancel))?;
            return Ok(None);
        };
        let observed = read_observation(&file, parent.identity(), cancel, progress)?;
        self.validate_target(index, &observed)?;
        let kind = self.targets[index].recorded.kind;
        inspection_probe(cancel, progress, || {
            if kind == EntryKind::Directory {
                ensure_directory(&file)
            } else {
                ensure_regular(&file)
            }
        })?;
        let basic = inspection_probe(cancel, progress, || repair_basic(&file))?;
        let after = read_observation(&file, parent.identity(), cancel, progress)?;
        if after != observed {
            return Err(stale("target changed during metadata inspection"));
        }
        inspection_probe(cancel, progress, || parent.validate(cancel))?;
        cancelled(cancel)?;
        Ok(Some(WindowsCopyMetadataObservation {
            observation: observed,
            kind,
            creation_time: basic.CreationTime,
            access_time: basic.LastAccessTime,
            write_time: basic.LastWriteTime,
            readonly: basic.FileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0,
            retained_attributes: copy_metadata::retained_attributes(basic.FileAttributes),
        }))
    }

    pub fn inspect_main_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<(MutationObservation, ContentDigest), MutationError> {
        let parent = self.parent(index, cancel, progress)?;
        let name = self.leaf(index)?;
        let file = inspection_probe(cancel, progress, || {
            open_relative_file(parent.handle(), units(&name)?, false, FILE_GENERIC_READ)
        })?;
        let before = read_observation(&file, parent.identity(), cancel, progress)?;
        self.validate_target(index, &before)?;
        let length = before
            .size
            .ok_or_else(|| stale("main stream inspection requires a file"))?;
        let basic = inspection_probe(cancel, progress, || repair_basic(&file))?;
        let digest = move_streams::stream_digest_observed(&file, length, cancel, progress)?;
        if read_observation(&file, parent.identity(), cancel, progress)? != before
            || !same_basic(
                &basic,
                &inspection_probe(cancel, progress, || repair_basic(&file))?,
            )
        {
            return Err(stale("main stream changed during inspection"));
        }
        inspection_probe(cancel, progress, || parent.validate(cancel))?;
        cancelled(cancel)?;
        Ok((before, digest))
    }

    pub fn inspect_streams_observed(
        &self,
        index: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        let target = self
            .targets
            .get(index)
            .ok_or_else(|| stale("unbound target index"))?;
        if target.recorded.kind != EntryKind::File {
            return Err(stale("stream inspection requires a recorded file"));
        }
        let expected = target
            .recorded
            .identity
            .as_ref()
            .ok_or_else(|| stale("full-stream inspection requires recorded target identity"))?;
        let parent = self.parent(index, cancel, progress)?;
        let name = self.leaf(index)?;
        let held = inspection_probe(cancel, progress, || {
            super::super::super::open_child(parent.handle(), units(&name)?, false)
                .map_err(read_error)
        })?;
        let before = read_observation(&held, parent.identity(), cancel, progress)?;
        self.validate_target(index, &before)?;
        let evidence = move_streams::inspect_bound_streams_observed(
            parent.handle(),
            &name,
            expected,
            cancel,
            progress,
            || parent.validate(cancel),
        )?;
        if read_observation(&held, parent.identity(), cancel, progress)? != before {
            return Err(stale("stream target changed during inspection"));
        }
        inspection_probe(cancel, progress, || parent.validate(cancel))?;
        cancelled(cancel)?;
        Ok(evidence)
    }
}

fn checked_ledger(
    records: Vec<WindowsAlbumInspectionTarget>,
    selected: &Path,
    root_identity: &ObjectIdentity,
    cancel: &AtomicBool,
) -> Result<(Vec<CheckedTarget>, HashMap<PathBuf, ObjectIdentity>), MutationError> {
    if records.len() > 12_288 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "Album inspection ledger exceeds limits",
        ));
    }
    let mut references = HashSet::new();
    let mut directory_ids = HashMap::from([(PathBuf::new(), root_identity.clone())]);
    let mut targets = Vec::new();
    for record in records {
        cancelled(cancel)?;
        let full = path_of(&record.reference)?;
        let relative = full
            .strip_prefix(selected)
            .map_err(|_| stale("target ledger escapes the recorded root"))?
            .to_path_buf();
        if relative.as_os_str().is_empty()
            || !references.insert(relative.clone())
            || !matches!(record.kind, EntryKind::File | EntryKind::Directory)
            || !matches!(record.parent_identity, ObjectIdentity::Windows { .. })
            || record
                .identity
                .as_ref()
                .is_some_and(|value| !matches!(value, ObjectIdentity::Windows { .. }))
        {
            return Err(stale("invalid or repeated Album target ledger entry"));
        }
        for component in relative.components() {
            let Component::Normal(value) = component else {
                return Err(stale("target ledger has non-child components"));
            };
            units(&MutationName::Windows(value.encode_wide().collect()))?;
        }
        if relative.components().count() > 128 {
            return Err(err(
                MutationErrorKind::Unsupported,
                "target inspection depth exceeds limit",
            ));
        }
        if record.kind == EntryKind::Directory
            && let Some(identity) = &record.identity
        {
            directory_ids.insert(relative.clone(), identity.clone());
        }
        targets.push(CheckedTarget {
            recorded: record,
            relative,
        });
    }
    for target in &targets {
        cancelled(cancel)?;
        if directory_ids.get(
            target
                .relative
                .parent()
                .ok_or_else(|| stale("target ledger parent missing"))?,
        ) != Some(&target.recorded.parent_identity)
        {
            return Err(stale("target ledger parent identity is not bound"));
        }
    }
    cancelled(cancel)?;
    Ok((targets, directory_ids))
}

fn stale(detail: &str) -> MutationError {
    err(MutationErrorKind::StalePlan, detail)
}
fn path_of(reference: &StorageRef) -> Result<PathBuf, MutationError> {
    let StorageRef::WindowsPath { utf16 } = reference else {
        return Err(stale("Windows reference required"));
    };
    let path = PathBuf::from(std::ffi::OsString::from_wide(utf16));
    super::super::super::validate_path(&path).map_err(read_error)?;
    Ok(path)
}
fn reference_of(path: &Path) -> StorageRef {
    StorageRef::WindowsPath {
        utf16: path.as_os_str().encode_wide().collect(),
    }
}
fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
fn same_basic(a: &FILE_BASIC_INFO, b: &FILE_BASIC_INFO) -> bool {
    // Read access time is allowed to advance. Content/representation timestamps
    // and attributes must remain the same through a content or journal read.
    a.CreationTime == b.CreationTime
        && a.LastWriteTime == b.LastWriteTime
        && a.FileAttributes == b.FileAttributes
}

#[cfg(test)]
#[path = "windows_album_inspection_storage_tests.rs"]
mod tests;
