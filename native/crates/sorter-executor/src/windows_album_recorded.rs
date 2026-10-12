//! Untrusted file-Album header data. No deserialized execution scope or native
//! grant is exposed; strict live evidence and phase replay remain separate.
use super::*;
use crate::inspection_control::{MAX_OPERATIONS, check_recorded_input};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedAlbumFileJob {
    pub job_id: String,
    pub source: EntryObservation,
    pub target: EntryObservation,
    pub original_source: Option<MutationObservation>,
    pub binding_sha256: String,
    pub file_count: usize,
    pub files: Vec<RecordedAlbumFile>,
    pub album_operations: Vec<usize>,
    pub copied_move: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedAlbumFile {
    pub physical_operation: usize,
    pub operation: usize,
    pub source: StorageRef,
    pub destination: StorageRef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: u16,
    binding: BindingData,
    snapshot: RecordedSnapshot,
    #[serde(default)]
    original_source: Option<MutationObservation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedSnapshot {
    copied_directories: Vec<RecordedCopiedDirectory>,
    directories: Vec<RecordedDirectory>,
    destinations: Vec<RecordedDestination>,
    files: Vec<RecordedFile>,
    albums: Vec<RecordedAlbum>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedCopiedDirectory {
    operation: usize,
    source: EntryObservation,
    destination: RecordedDestinationValue,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedDirectory {
    source: EntryObservation,
    children: Vec<EntryObservation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedDestination {
    destination: RecordedDestinationValue,
    existing: Option<EntryObservation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedFile {
    original_operation: usize,
    source: EntryObservation,
    destination: RecordedDestinationValue,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedChild {
    source: EntryObservation,
    destination_name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedAlbum {
    children: Vec<RecordedChild>,
    operation: usize,
    artist: String,
    source: EntryObservation,
    destination: RecordedDestinationValue,
    files: Vec<usize>,
    cleanup: Vec<EntryObservation>,
}

// Journal values remain data-only until their root and components are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordedDestinationValue {
    root: StorageRef,
    components: Vec<String>,
}
impl RecordedDestinationValue {
    fn into_destination(self) -> Result<Destination> {
        checked_path(&self.root)?;
        child_destination(&self.root, &self.components).map_err(|_| E::CorruptJournal)
    }
}

struct ValidatedHeader {
    binding: BindingData,
    snapshot: Snapshot,
    original_source: Option<MutationObservation>,
}
fn invalid<T>() -> Result<T> {
    Err(E::CorruptJournal)
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn checked_path(value: &StorageRef) -> Result<PathBuf> {
    let path = path(value)?;
    if !path.is_absolute()
        || !matches!(path.components().next(), Some(std::path::Component::Prefix(prefix))
        if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
        || path.as_os_str().encode_wide().count() > 32_767
    {
        return invalid();
    }
    Ok(path)
}
fn child_destination(root: &StorageRef, components: &[String]) -> Result<Destination> {
    if components.is_empty() || components.len() > 128 {
        return invalid();
    }
    let destination = Destination {
        root: root.clone(),
        components: components.to_vec(),
    };
    destination_path(&destination)?;
    Ok(destination)
}
fn volume(identity: &ObjectIdentity) -> Result<u64> {
    match identity {
        ObjectIdentity::Windows { volume, .. } => Ok(*volume),
        _ => invalid(),
    }
}
fn check_observation(
    entry: &EntryObservation,
    root: &PathBuf,
    root_id: &ObjectIdentity,
) -> Result<()> {
    let observed = checked_path(&entry.reference)?;
    let relative = observed.strip_prefix(root).map_err(|_| E::CorruptJournal)?;
    // Only the explicitly selected root may retain an alias/display spelling.
    // Every descendant name must describe its lossless reference's exact leaf.
    if !relative.as_os_str().is_empty()
        && observed
            .file_name()
            .map(|leaf| leaf.encode_wide().collect::<Vec<_>>())
            != Some(entry.name.encode_utf16().collect())
    {
        return invalid();
    }
    let identity = entry.identity.as_ref().ok_or(E::CorruptJournal)?;
    let parent = entry.parent_identity.as_ref().ok_or(E::CorruptJournal)?;
    if volume(identity)? != volume(root_id)?
        || volume(parent)? != volume(identity)?
        || identity == parent
        || relative.components().count() > 128
        || !matches!(entry.kind, EntryKind::File | EntryKind::Directory)
        || (entry.kind == EntryKind::File
            && (entry.size.is_none()
                || entry.modified.is_none()
                || relative.as_os_str().is_empty()))
        || (entry.kind == EntryKind::Directory && entry.size.is_some())
    {
        return invalid();
    }
    Ok(())
}
impl RecordedSnapshot {
    fn into_snapshot(self, control: &InspectionControl) -> Result<Snapshot> {
        if self.files.is_empty()
            || self.files.len() > MAX_OPERATIONS
            || self.albums.len() > MAX_OPERATIONS
            || self.directories.len() > 8192
            || self.destinations.len() > 8192
            || self.copied_directories.len() > 8192
        {
            return Err(E::Unsupported);
        }
        let mut snapshot = Snapshot {
            copied_directories: vec![],
            directories: vec![],
            destinations: vec![],
            files: vec![],
            albums: vec![],
        };
        let mut entries = 0usize;
        for directory in self.directories {
            control.checkpoint()?;
            entries = entries
                .checked_add(directory.children.len())
                .filter(|count| *count <= 100_000)
                .ok_or(E::Unsupported)?;
            snapshot.directories.push(DirectorySnapshot {
                source: directory.source,
                children: directory.children,
            });
        }
        for directory in self.copied_directories {
            control.checkpoint()?;
            snapshot.copied_directories.push(CopiedDirectory {
                operation: directory.operation,
                source: directory.source,
                destination: directory.destination.into_destination()?,
            });
        }
        for destination in self.destinations {
            control.checkpoint()?;
            snapshot.destinations.push(DirectoryRequirement {
                destination: destination.destination.into_destination()?,
                existing: destination.existing,
            });
        }
        for file in self.files {
            control.checkpoint()?;
            snapshot.files.push(FileIntent {
                original_operation: file.original_operation,
                source: file.source,
                destination: file.destination.into_destination()?,
            });
        }
        for album in self.albums {
            control.checkpoint()?;
            if album.children.len() > 100_000
                || album.cleanup.len() > 8192
                || album.files.len() > MAX_OPERATIONS
            {
                return Err(E::Unsupported);
            }
            let mut children = Vec::new();
            for child in album.children {
                control.checkpoint()?;
                children.push(AlbumChildPlan {
                    source: child.source,
                    destination_name: child.destination_name,
                });
            }
            snapshot.albums.push(AlbumGroup {
                children,
                operation: album.operation,
                artist: album.artist,
                source: album.source,
                destination: album.destination.into_destination()?,
                files: album.files,
                cleanup: album.cleanup,
            });
        }
        Ok(snapshot)
    }
}

// Reconstruct exactly the collector's ordered directory/file expansion from
// recorded membership, then compare it with the other serialized projections.
// Merely comparing an untrusted Header to itself would validate no invariant.
struct Expansion<'a> {
    control: &'a InspectionControl<'a>,
    directories: HashMap<StorageRef, &'a DirectorySnapshot>,
    visited: HashSet<StorageRef>,
    directory_order: Vec<StorageRef>,
    target: &'a StorageRef,
    destinations: Vec<Destination>,
    seen_destinations: HashSet<StorageRef>,
    files: Vec<FileIntent>,
    copied_directories: Vec<CopiedDirectory>,
}
impl Expansion<'_> {
    fn directory(&mut self, source: &EntryObservation) -> Result<Vec<EntryObservation>> {
        self.control.checkpoint()?;
        let directory = self
            .directories
            .get(&source.reference)
            .ok_or(E::CorruptJournal)?;
        if directory.source != *source {
            return invalid();
        }
        if self.visited.insert(source.reference.clone()) {
            self.directory_order.push(source.reference.clone());
        }
        Ok(directory.children.clone())
    }
    fn destination(&mut self, destination: &Destination) -> Result<()> {
        if destination.root != *self.target || destination.components.len() > 128 {
            return invalid();
        }
        for count in 1..=destination.components.len() {
            self.control.checkpoint()?;
            let partial = child_destination(self.target, &destination.components[..count])?;
            if self
                .seen_destinations
                .insert(reference(&destination_path(&partial)?))
            {
                self.destinations.push(partial);
            }
        }
        Ok(())
    }
    fn expand(
        &mut self,
        source: &EntryObservation,
        destination: Destination,
        operation: usize,
        cleanup: &mut Vec<EntryObservation>,
        depth: usize,
    ) -> Result<()> {
        self.control.checkpoint()?;
        if depth > 128 {
            return Err(E::Unsupported);
        }
        child_destination(self.target, &destination.components)?;
        match source.kind {
            EntryKind::File => {
                self.destination(&Destination {
                    root: destination.root.clone(),
                    components: destination.components[..destination.components.len() - 1].to_vec(),
                })?;
                if self.files.len() >= MAX_OPERATIONS {
                    return Err(E::Unsupported);
                }
                self.files.push(FileIntent {
                    original_operation: operation,
                    source: source.clone(),
                    destination,
                });
            }
            EntryKind::Directory => {
                self.destination(&destination)?;
                for child in self.directory(source)? {
                    self.control.checkpoint()?;
                    if leaf(&child)? != name(&child.name) {
                        return invalid();
                    }
                    let mut next = destination.clone();
                    next.components.push(child.name.clone());
                    self.expand(&child, next, operation, cleanup, depth + 1)?;
                }
                if self.copied_directories.len() >= 8192 {
                    return Err(E::Unsupported);
                }
                self.copied_directories.push(CopiedDirectory {
                    operation,
                    source: source.clone(),
                    destination,
                });
                cleanup.push(source.clone());
            }
            _ => return invalid(),
        }
        Ok(())
    }
}

fn validate_snapshot(
    binding: &BindingData,
    snapshot: &Snapshot,
    control: &InspectionControl,
) -> Result<()> {
    let source_path = checked_path(&binding.source_root.reference)?;
    let target_path = checked_path(&binding.target_root.reference)?;
    let source_id = binding
        .source_root
        .identity
        .as_ref()
        .ok_or(E::CorruptJournal)?;
    let target_id = binding
        .target_root
        .identity
        .as_ref()
        .ok_or(E::CorruptJournal)?;
    if source_path.starts_with(&target_path)
        || target_path.starts_with(&source_path)
        || source_id == target_id
        || binding.source_root.kind != EntryKind::Directory
        || binding.target_root.kind != EntryKind::Directory
    {
        return invalid();
    }
    check_observation(&binding.source_root, &source_path, source_id)?;
    check_observation(&binding.target_root, &target_path, target_id)?;
    let mut directory_map = HashMap::new();
    let mut catalog = HashMap::from([(
        binding.source_root.reference.clone(),
        binding.source_root.clone(),
    )]);
    for directory in &snapshot.directories {
        control.checkpoint()?;
        check_observation(&directory.source, &source_path, source_id)?;
        if directory.source.kind != EntryKind::Directory
            || directory_map
                .insert(directory.source.reference.clone(), directory)
                .is_some()
        {
            return invalid();
        }
        let mut children = HashSet::new();
        for child in &directory.children {
            control.checkpoint()?;
            check_observation(child, &source_path, source_id)?;
            if checked_path(&child.reference)?.parent()
                != Some(checked_path(&directory.source.reference)?.as_path())
                || child.parent_identity != directory.source.identity
                || !children.insert(child.reference.clone())
            {
                return invalid();
            }
        }
        for entry in std::iter::once(&directory.source).chain(&directory.children) {
            control.checkpoint()?;
            if catalog
                .insert(entry.reference.clone(), entry.clone())
                .is_some_and(|prior| prior != *entry)
            {
                return invalid();
            }
        }
    }
    let mut album_operations = HashMap::new();
    let mut original_operations = HashSet::new();
    for album in &snapshot.albums {
        control.checkpoint()?;
        if album.operation >= MAX_OPERATIONS
            || album.artist.is_empty()
            || album_operations.insert(album.operation, album).is_some()
        {
            return invalid();
        }
        original_operations.insert(album.operation);
    }
    for file in &snapshot.files {
        control.checkpoint()?;
        check_observation(&file.source, &source_path, source_id)?;
        if file.source.kind != EntryKind::File || file.original_operation >= MAX_OPERATIONS {
            return invalid();
        }
        original_operations.insert(file.original_operation);
        if catalog
            .get(&file.source.reference)
            .is_some_and(|entry| *entry != file.source)
        {
            return invalid();
        }
    }
    let mut source_identities = HashMap::new();
    for entry in catalog
        .values()
        .chain(snapshot.files.iter().map(|file| &file.source))
    {
        control.checkpoint()?;
        let identity = entry.identity.as_ref().ok_or(E::CorruptJournal)?;
        if source_identities
            .insert(identity.clone(), entry.reference.clone())
            .is_some_and(|prior| prior != entry.reference)
        {
            return invalid();
        }
        let parent_path = reference(
            checked_path(&entry.reference)?
                .parent()
                .ok_or(E::CorruptJournal)?,
        );
        if catalog
            .get(&parent_path)
            .is_some_and(|parent| parent.identity != entry.parent_identity)
        {
            return invalid();
        }
    }
    if original_operations.len() > MAX_OPERATIONS
        || (0..original_operations.len()).any(|id| !original_operations.contains(&id))
    {
        return invalid();
    }
    let mut expansion = Expansion {
        control,
        directories: directory_map,
        visited: HashSet::new(),
        directory_order: vec![],
        target: &binding.target_root.reference,
        destinations: vec![],
        seen_destinations: HashSet::new(),
        files: vec![],
        copied_directories: vec![],
    };
    for operation in 0..original_operations.len() {
        control.checkpoint()?;
        if let Some(album) = album_operations.get(&operation) {
            let all = expansion.directory(&album.source)?;
            let mut children = Vec::new();
            for child in &all {
                control.checkpoint()?;
                if let Some(destination_name) = sorter_core::sanitize(&child.name) {
                    children.push(AlbumChildPlan {
                        source: child.clone(),
                        destination_name,
                    });
                }
            }
            if children != album.children {
                return invalid();
            }
            expansion.destination(&album.destination)?;
            let start = expansion.files.len();
            let mut cleanup = Vec::new();
            for child in &children {
                let mut destination = album.destination.clone();
                destination.components.push(child.destination_name.clone());
                expansion.expand(&child.source, destination, operation, &mut cleanup, 0)?;
            }
            if !binding.selected_windows_copy && children.len() == all.len() {
                cleanup.push(album.source.clone());
            }
            if cleanup != album.cleanup
                || album.files != (start..expansion.files.len()).collect::<Vec<_>>()
            {
                return invalid();
            }
        } else {
            let mut files = snapshot
                .files
                .iter()
                .filter(|file| file.original_operation == operation);
            let file = files.next().ok_or(E::CorruptJournal)?;
            if files.next().is_some() {
                return invalid();
            }
            expansion.expand(
                &file.source,
                file.destination.clone(),
                operation,
                &mut vec![],
                0,
            )?;
        }
    }
    if expansion.files != snapshot.files
        || expansion.copied_directories != snapshot.copied_directories
        || expansion.directory_order
            != snapshot
                .directories
                .iter()
                .map(|directory| directory.source.reference.clone())
                .collect::<Vec<_>>()
        || expansion.destinations
            != snapshot
                .destinations
                .iter()
                .map(|entry| entry.destination.clone())
                .collect::<Vec<_>>()
        || !snapshot
            .albums
            .windows(2)
            .all(|pair| pair[0].operation < pair[1].operation)
    {
        return invalid();
    }
    let mut target_directories = HashMap::from([(
        binding.target_root.reference.clone(),
        binding.target_root.identity.clone(),
    )]);
    for requirement in &snapshot.destinations {
        control.checkpoint()?;
        let destination = reference(&destination_path(&requirement.destination)?);
        let parent = reference(
            checked_path(&destination)?
                .parent()
                .ok_or(E::CorruptJournal)?,
        );
        let expected_parent = target_directories.get(&parent).ok_or(E::CorruptJournal)?;
        if let Some(existing) = &requirement.existing {
            check_observation(existing, &target_path, target_id)?;
            if existing.reference != destination
                || existing.kind != EntryKind::Directory
                || expected_parent.is_none()
                || &existing.parent_identity != expected_parent
            {
                return invalid();
            }
        }
        target_directories.insert(
            destination,
            requirement
                .existing
                .as_ref()
                .and_then(|entry| entry.identity.clone()),
        );
    }
    Ok(())
}

fn validate_header(header: Header, control: &InspectionControl) -> Result<ValidatedHeader> {
    control.checkpoint()?;
    let Header {
        schema,
        binding,
        snapshot,
        original_source,
    } = header;
    if binding.operations.is_empty()
        || binding.operations.len() > MAX_OPERATIONS
        || binding.scope_evidence.inspected_sources.len() > MAX_OPERATIONS
    {
        return Err(E::Unsupported);
    }
    if binding.schema != 2
        || !binding.valid_domain()
        || binding.album_context.is_some()
        || !(binding.selected_windows_copy
            || binding.selected_windows_rename
            || binding.copied_move_sources.is_some())
        || !valid_hash(&binding.plan_sha256)
        || !valid_hash(&binding.scope_sha256)
    {
        return invalid();
    }
    control.checkpoint()?;
    match (
        schema,
        &original_source,
        binding.copied_move_sources.is_some(),
    ) {
        (1, None, false) => {}
        (2, Some(source), true) => {
            let native_path = checked_path(&source.reference)?;
            let actual_leaf = MutationName::Windows(
                native_path
                    .file_name()
                    .ok_or(E::CorruptJournal)?
                    .encode_wide()
                    .collect(),
            );
            if source.size.is_some()
                || source.actual_name != actual_leaf
                || Some(&source.identity) != binding.source_root.identity.as_ref()
                || Some(&source.parent_identity) != binding.source_root.parent_identity.as_ref()
            {
                return invalid();
            }
        }
        _ => return invalid(),
    }
    let evidence = &binding.scope_evidence;
    if binding.source_root.identity.as_ref() != Some(&evidence.source_root)
        || binding.target_root.identity.as_ref() != Some(&evidence.target_root)
        || binding.plan_sha256 != evidence.inspected_plan
        || binding.resources != evidence.resources
        || crate::hash(&(
            &evidence.inspected_plan,
            &evidence.source_root,
            &evidence.target_root,
            &evidence.inspected_sources,
            &evidence.resources,
            &evidence.allowed_modes,
        ))? != binding.scope_sha256
        || binding.resources.resource_hashes.len() != binding.resources.resource_ids.len()
        || binding.resources.adapter_hashes.len() != binding.resources.adapter_versions.len()
        || !binding
            .resources
            .resource_hashes
            .iter()
            .chain(&binding.resources.adapter_hashes)
            .all(|hash| valid_hash(hash))
    {
        return invalid();
    }
    match &binding.resources.ai_profile {
        AiProfile::Disabled if binding.resources.model_hash.is_none() => {}
        AiProfile::Configured {
            available: true, ..
        } if binding
            .resources
            .model_hash
            .as_deref()
            .is_some_and(valid_hash) => {}
        _ => return invalid(),
    }
    let snapshot = snapshot.into_snapshot(control)?;
    validate_snapshot(&binding, &snapshot, control)?;
    if binding.operations.len() != snapshot.files.len() || snapshot.albums.is_empty() {
        return invalid();
    }
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    let mut destinations = HashSet::new();
    for (index, (operation, file)) in binding.operations.iter().zip(&snapshot.files).enumerate() {
        control.checkpoint()?;
        if operation.id != index
            || operation.depends_on != (if index == 0 { vec![] } else { vec![index - 1] })
            || operation.source.observation != file.source
            || operation.destination.root != file.destination.root
            || operation.destination.components != file.destination.components
            || operation.destination.id != crate::hash(&file.destination)?
            || !destinations.insert(operation.destination.id.clone())
            || operation.duet_cohort.is_some()
            || operation.copy_effect != binding.selected_windows_copy
            || !valid_hash(&operation.source.sha256)
            || (binding.selected_windows_rename
                && volume(
                    operation
                        .source
                        .observation
                        .identity
                        .as_ref()
                        .ok_or(E::CorruptJournal)?,
                )? != volume(&evidence.target_root)?)
        {
            return invalid();
        }
        if seen.insert(operation.source.observation.reference.clone()) {
            sources.push(operation.source.clone());
        } else if !sources.contains(&operation.source) {
            return invalid();
        }
    }
    if sources != evidence.inspected_sources {
        return invalid();
    }
    // Prepared copied directory metadata cannot later be invalidated by another
    // original operation writing under that directory.
    for directory in &snapshot.copied_directories {
        control.checkpoint()?;
        for file in &snapshot.files {
            control.checkpoint()?;
            if file.original_operation != directory.operation
                && file.destination.root == directory.destination.root
                && file
                    .destination
                    .components
                    .starts_with(&directory.destination.components)
            {
                return invalid();
            }
        }
    }
    Ok(ValidatedHeader {
        binding,
        snapshot,
        original_source,
    })
}

/// Describe a bounded, internally consistent file-Album header for root selection.
/// This verifies framing, not phase completion or any current filesystem fact.
/// The result carries neither a scope nor a writable or read-only native grant.
fn read_recorded_header(
    bytes: &[u8],
    control: &InspectionControl,
) -> Result<(String, ValidatedHeader)> {
    control.stage("album_descriptor")?;
    check_recorded_input(bytes, Some(control))?;
    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or(E::CorruptJournal)?;
    let first: ManifestEnvelope =
        serde_json::from_slice(&bytes[..end]).map_err(|_| E::CorruptJournal)?;
    if first.body.sequence != 0
        || !first.body.previous.is_empty()
        || first.body.phase != "Header"
        || crate::hash(&first.body)? != first.sha256
        || first.body.job_id.len() > 100
        || !first.body.job_id.starts_with("album-")
        || !first
            .body
            .job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return invalid();
    }
    let header: Header =
        serde_json::from_value(first.body.payload.clone()).map_err(|_| E::CorruptJournal)?;
    let validated = validate_header(header, control)?;
    let canonical = match &validated.original_source {
        Some(source) => {
            serde_json::json!({"schema":2,"binding":validated.binding,"snapshot":validated.snapshot,"original_source":source})
        }
        None => {
            serde_json::json!({"schema":1,"binding":validated.binding,"snapshot":validated.snapshot})
        }
    };
    if canonical != first.body.payload {
        return invalid();
    }
    let mut sequence = 0u64;
    let mut previous = String::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        control.record()?;
        let Some(end) = bytes[offset..].iter().position(|byte| *byte == b'\n') else {
            break;
        };
        let envelope: ManifestEnvelope =
            serde_json::from_slice(&bytes[offset..offset + end]).map_err(|_| E::CorruptJournal)?;
        if envelope.body.sequence != sequence
            || envelope.body.previous != previous
            || envelope.body.job_id != first.body.job_id
            || crate::hash(&envelope.body)? != envelope.sha256
            || (sequence != 0 && envelope.body.phase == "Header")
        {
            return invalid();
        }
        previous = envelope.sha256;
        sequence = sequence.checked_add(1).ok_or(E::Unsupported)?;
        offset = offset
            .checked_add(end)
            .and_then(|offset| offset.checked_add(1))
            .ok_or(E::Unsupported)?;
    }
    control.checkpoint()?;
    Ok((first.body.job_id, validated))
}

fn descriptor(job_id: String, validated: &ValidatedHeader) -> Result<RecordedAlbumFileJob> {
    Ok(RecordedAlbumFileJob {
        job_id,
        source: validated.binding.source_root.clone(),
        target: validated.binding.target_root.clone(),
        original_source: validated.original_source.clone(),
        binding_sha256: crate::hash(&validated.binding)?,
        file_count: validated.snapshot.files.len(),
        files: validated
            .snapshot
            .files
            .iter()
            .enumerate()
            .map(|(physical_operation, file)| {
                Ok(RecordedAlbumFile {
                    physical_operation,
                    operation: file.original_operation,
                    source: file.source.reference.clone(),
                    destination: reference(&destination_path(&file.destination)?),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        album_operations: validated
            .snapshot
            .albums
            .iter()
            .map(|album| album.operation)
            .collect(),
        copied_move: validated.binding.copied_move_sources.is_some(),
    })
}

pub fn inspect_recorded_album_file_job(
    bytes: &[u8],
    control: &InspectionControl,
) -> Result<RecordedAlbumFileJob> {
    let (job_id, validated) = read_recorded_header(bytes, control)?;
    descriptor(job_id, &validated)
}

/// Fresh read-only selection, never decoded from historical evidence.
pub enum AlbumFileInspectionSource {
    Present(WindowsSelectedRootGrant),
    Missing(WindowsAlbumInspectionParentGrant),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AlbumInspectionSourceState {
    Present { observed: MutationObservation },
    Absent { recorded: MutationObservation },
}

#[derive(Clone, Debug)]
pub struct RecordedAlbumFileInspection {
    pub recorded: RecordedAlbumFileJob,
    pub source_state: AlbumInspectionSourceState,
    pub recovery: SelectedAlbumRecovery,
    pub files: Vec<RecordedAlbumFileEvidence>,
}

#[derive(Clone, Debug)]
pub struct RecordedAlbumFileEvidence {
    pub file: RecordedAlbumFile,
    /// Operation ID stays physical here; the legacy aggregate report below is
    /// separately remapped to the original plan's possibly repeated Album IDs.
    pub recovery: Option<crate::recovery::OperationRecovery>,
}

/// Retained discovery evidence; compared with the actual freshly held journal.
#[derive(Clone, Debug)]
pub struct AlbumFileInspectionJournal {
    pub name: MutationName,
    pub identity: ObjectIdentity,
    pub length: u64,
    pub sha256: String,
}

pub struct AlbumFileInspectionGrants {
    pub source: AlbumFileInspectionSource,
    pub target: WindowsSelectedRootGrant,
    pub journal: WindowsSelectedRootGrant,
}

/// Reopen only the retained fixed journal identities under newly selected roots.
/// A recorded descriptor is compared again with the actual held journal bytes;
/// it never supplies a native grant, replay permission or cleanup authority.
pub fn recover_recorded_album_file_job(
    recorded: &RecordedAlbumFileJob,
    grants: AlbumFileInspectionGrants,
    journals: Vec<AlbumFileInspectionJournal>,
    control: &InspectionControl,
    start_operation: usize,
) -> Result<RecordedAlbumFileInspection> {
    use crate::windows_copy::{inspection_error, inspection_event};
    control.stage("album_restart_journals")?;
    if journals.is_empty()
        || journals.len() > 2
        || journals[0].name != name(&format!("{}.album.jsonl", recorded.job_id))
        || journals.get(1).is_some_and(|journal| {
            journal.name != name(&format!("{}.transfers.jsonl", recorded.job_id))
        })
    {
        return Err(E::CorruptJournal);
    }
    if journals.iter().any(|journal| !valid_hash(&journal.sha256)) {
        return Err(E::InvalidBinding);
    }
    let expected_hashes = journals
        .iter()
        .map(|journal| journal.sha256.clone())
        .collect::<Vec<_>>();
    let journals = journals
        .into_iter()
        .map(|journal| WindowsAlbumInspectionJournal {
            name: journal.name,
            identity: journal.identity,
            length: journal.length,
        })
        .collect();
    let cancel = control.cancellation_flag();
    let destination = WindowsAlbumJournalInspectionInputs {
        target: grants.target,
        journal: grants.journal,
        recorded_target: recorded.target.clone(),
        journals,
    };
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
                &mut |event| inspection_event(control, event),
            )
        }
        AlbumFileInspectionSource::Missing(parent) => {
            let original = recorded.original_source.clone().ok_or(E::Unsupported)?;
            let inspection = WindowsAlbumOriginalLocationInspection::open_observed(
                parent,
                original,
                cancel,
                &mut |event| inspection_event(control, event),
            )
            .map_err(inspection_error)?;
            WindowsAlbumJournalInspection::open_observed(
                destination,
                inspection,
                cancel,
                &mut |event| inspection_event(control, event),
            )
        }
    }
    .map_err(inspection_error)?;
    let original = reader
        .inspect_original_observed(cancel, &mut |event| inspection_event(control, event))
        .map_err(inspection_error)?;
    let observed_source = match (&original, present) {
        (WindowsAlbumOriginalLocation::OriginalPresent(actual), true) => {
            if recorded
                .original_source
                .as_ref()
                .is_some_and(|expected| expected != actual)
            {
                return Err(E::Stale);
            }
            actual.reference.clone()
        }
        (WindowsAlbumOriginalLocation::Absent, false) => recorded
            .original_source
            .as_ref()
            .ok_or(E::Scope)?
            .reference
            .clone(),
        _ => return Err(E::Stale),
    };
    let (validated, binding, mut report, observed, physical) = {
        let bytes = reader
            .read_journals_observed(cancel, &mut |event| inspection_event(control, event))
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
        let (job_id, validated) = read_recorded_header(&bytes[0], control)?;
        if descriptor(job_id.clone(), &validated)? != *recorded {
            return Err(E::Stale);
        }
        let binding = ExecutionBinding {
            data: validated.binding.clone(),
            recovery_only: true,
            execution_decisions: None,
        };
        let (report, observed) = file_manifest::FileManifestEvidence {
            binding: &binding,
            snapshot: &validated.snapshot,
            job_id: &job_id,
            observed_source_root: &observed_source,
            original_source: &validated.original_source,
        }
        .read_manifest(&bytes[0], control)?;
        let physical = bytes
            .get(1)
            .map(|bytes| PhysicalJournalV2::open_controlled(&binding, &job_id, bytes, control))
            .transpose()?;
        validate_transfer_receipts(
            &validated.snapshot,
            &binding,
            &report,
            physical.as_ref(),
            control,
        )?;
        (validated, binding, report, observed, physical)
    };
    let (sources, source_indices) = source_records(&validated.snapshot, &binding, control)?;
    let (targets, target_indices) = target_records(
        &validated.snapshot,
        &binding,
        &observed,
        physical.as_ref(),
        control,
    )?;
    let target_directories = targets
        .iter()
        .enumerate()
        .filter_map(|(index, record)| (record.kind == EntryKind::Directory).then_some(index))
        .collect();
    let owner = if present {
        reader.bind_present_ledgers_observed(sources, targets, cancel, &mut |event| {
            inspection_event(control, event)
        })
    } else {
        reader.bind_targets_observed(targets, cancel, &mut |event| {
            inspection_event(control, event)
        })
    }
    .map_err(inspection_error)?;
    let mut storage = RecordedAlbumReads {
        owner,
        binding: &binding,
        source_indices,
        target_indices,
        present,
        control,
        destination_operations: binding
            .operations()
            .iter()
            .enumerate()
            .map(|(index, operation)| (operation.destination.id.clone(), index))
            .collect(),
        operation_sources: binding
            .operations()
            .iter()
            .map(|operation| operation.source.observation.reference.clone())
            .collect(),
        move_sources: binding
            .data
            .copied_move_sources
            .iter()
            .flatten()
            .map(|source| (source.source.clone(), source))
            .collect(),
        target_directories,
    };
    inspect_directory_metadata(&mut report, &storage)?;
    let pins = inspect_directories(&mut report, &observed, &storage)?;
    let mut file_evidence = recorded
        .files
        .iter()
        .cloned()
        .map(|file| RecordedAlbumFileEvidence {
            file,
            recovery: None,
        })
        .collect::<Vec<_>>();
    if let Some(physical) = physical {
        let mut transfers = crate::recovery::recover_job_controlled(
            &binding,
            &physical,
            &mut storage,
            control,
            start_operation,
        )?;
        for operation in &mut transfers.operations {
            control.checkpoint()?;
            let evidence = file_evidence
                .get_mut(operation.operation)
                .ok_or(E::CorruptJournal)?;
            if evidence.file.physical_operation != operation.operation
                || evidence.recovery.is_some()
            {
                return Err(E::CorruptJournal);
            }
            evidence.recovery = Some(operation.clone());
            operation.operation = validated
                .snapshot
                .files
                .get(operation.operation)
                .ok_or(E::CorruptJournal)?
                .original_operation;
        }
        report.transfers = Some(transfers);
    } else if start_operation != 0 {
        return Err(E::InvalidBinding);
    }
    for pin in &pins {
        pin.revalidate_observed(cancel, &mut |event| inspection_event(control, event))
            .map_err(inspection_error)?;
    }
    let final_source = storage
        .owner
        .inspect_original_observed(cancel, &mut |event| inspection_event(control, event))
        .map_err(inspection_error)?;
    if final_source != original {
        return Err(E::Stale);
    }
    control.checkpoint()?;
    let source_state = match original {
        WindowsAlbumOriginalLocation::OriginalPresent(observed) => {
            AlbumInspectionSourceState::Present { observed }
        }
        WindowsAlbumOriginalLocation::Absent => AlbumInspectionSourceState::Absent {
            recorded: recorded.original_source.clone().ok_or(E::Scope)?,
        },
        WindowsAlbumOriginalLocation::ForeignPresent(_) => return Err(E::Stale),
    };
    Ok(RecordedAlbumFileInspection {
        recorded: recorded.clone(),
        source_state,
        recovery: report,
        files: file_evidence,
    })
}

fn validate_transfer_receipts(
    snapshot: &Snapshot,
    binding: &ExecutionBinding,
    report: &SelectedAlbumRecovery,
    journal: Option<&PhysicalJournalV2>,
    control: &InspectionControl,
) -> Result<()> {
    let Some(journal) = journal else {
        if report.recorded_transfers.is_some()
            || !report.album_transfers.is_empty()
            || !report.completed_albums.is_empty()
        {
            return Err(E::CorruptJournal);
        }
        return Ok(());
    };
    if let Some((status, committed, moved)) = &report.recorded_transfers {
        let counts = JobResult::from_journal(journal, vec![], false);
        if *committed != counts.committed_destinations
            || *moved != counts.moved_sources
            || (journal.needs_recovery() && status != "NeedsRecovery")
            || (status == "Complete" && *committed != binding.operations().len())
        {
            return Err(E::CorruptJournal);
        }
    }
    let albums = snapshot
        .albums
        .iter()
        .map(|album| (album.operation, album))
        .collect::<HashMap<_, _>>();
    let mut terminal_events: HashMap<usize, Vec<(usize, &crate::physical_journal::Event)>> =
        HashMap::new();
    for (position, event) in journal.events().iter().enumerate() {
        control.checkpoint()?;
        if matches!(
            event.phase,
            crate::physical_journal::Phase::Committed
                | crate::physical_journal::Phase::SourceRetired
        ) {
            terminal_events
                .entry(event.operation)
                .or_default()
                .push((position, event));
        }
    }
    for ack in &report.album_transfers {
        control.checkpoint()?;
        let album = albums.get(&ack.operation).ok_or(E::CorruptJournal)?;
        let mut events = Vec::new();
        for id in &album.files {
            control.checkpoint()?;
            events.extend(
                terminal_events
                    .get(id)
                    .ok_or(E::CorruptJournal)?
                    .iter()
                    .copied(),
            );
        }
        events.sort_by_key(|(position, _)| *position);
        let expected = AlbumTransferAcknowledgment {
            operation: album.operation,
            events: events.into_iter().map(|(_, event)| event.clone()).collect(),
        };
        expected.validate(album, binding)?;
        control.checkpoint()?;
        if *ack != expected {
            return Err(E::CorruptJournal);
        }
    }
    let committed = journal
        .events()
        .iter()
        .filter(|event| event.phase == crate::physical_journal::Phase::Committed)
        .map(|event| event.operation)
        .collect::<HashSet<_>>();
    for operation in &report.completed_albums {
        control.checkpoint()?;
        let album = albums.get(operation).ok_or(E::CorruptJournal)?;
        if album.files.iter().any(|id| !committed.contains(id)) {
            return Err(E::CorruptJournal);
        }
    }
    Ok(())
}

type ReadIndices = HashMap<StorageRef, usize>;
fn source_records(
    snapshot: &Snapshot,
    binding: &ExecutionBinding,
    control: &InspectionControl,
) -> Result<(Vec<WindowsAlbumInspectionTarget>, ReadIndices)> {
    let mut records = Vec::new();
    let mut indices = HashMap::new();
    for entry in snapshot
        .directories
        .iter()
        .map(|directory| &directory.source)
        .chain(snapshot.files.iter().map(|file| &file.source))
    {
        control.checkpoint()?;
        if entry.reference == binding.data.source_root.reference {
            continue;
        }
        if indices.contains_key(&entry.reference) {
            continue;
        }
        let index = records.len();
        records.push(WindowsAlbumInspectionTarget {
            reference: entry.reference.clone(),
            identity: entry.identity.clone(),
            parent_identity: entry.parent_identity.clone().ok_or(E::CorruptJournal)?,
            kind: entry.kind,
        });
        indices.insert(entry.reference.clone(), index);
    }
    Ok((records, indices))
}

fn target_records(
    snapshot: &Snapshot,
    binding: &ExecutionBinding,
    observed: &[(StorageRef, ObjectIdentity)],
    journal: Option<&PhysicalJournalV2>,
    control: &InspectionControl,
) -> Result<(Vec<WindowsAlbumInspectionTarget>, ReadIndices)> {
    let root = &binding.data.target_root;
    let mut identities = HashMap::from([(
        root.reference.clone(),
        root.identity.clone().ok_or(E::CorruptJournal)?,
    )]);
    for destination in &snapshot.destinations {
        control.checkpoint()?;
        if let Some(existing) = &destination.existing {
            identities.insert(
                existing.reference.clone(),
                existing.identity.clone().ok_or(E::CorruptJournal)?,
            );
        }
    }
    let target_path = path(&root.reference)?;
    for (reference, identity) in observed {
        control.checkpoint()?;
        if path(reference)?.starts_with(&target_path)
            && identities
                .insert(reference.clone(), identity.clone())
                .is_some()
        {
            return Err(E::CorruptJournal);
        }
    }
    let mut records = Vec::new();
    let mut indices = HashMap::new();
    for destination in &snapshot.destinations {
        control.checkpoint()?;
        let full = destination_path(&destination.destination)?;
        let reference = reference(&full);
        if reference == root.reference {
            continue;
        }
        let Some(identity) = identities.get(&reference) else {
            continue;
        };
        let parent_reference = super::reference(full.parent().ok_or(E::CorruptJournal)?);
        let parent_identity = identities
            .get(&parent_reference)
            .ok_or(E::CorruptJournal)?
            .clone();
        indices.insert(reference.clone(), records.len());
        records.push(WindowsAlbumInspectionTarget {
            reference,
            identity: Some(identity.clone()),
            parent_identity,
            kind: EntryKind::Directory,
        });
    }
    let mut published = HashMap::new();
    if let Some(journal) = journal {
        for event in journal.events() {
            control.checkpoint()?;
            if matches!(
                event.phase,
                crate::physical_journal::Phase::DestinationPublished
                    | crate::physical_journal::Phase::AtomicRenameObserved
            ) {
                published.insert(
                    event.operation,
                    event.observed_identity.clone().ok_or(E::CorruptJournal)?,
                );
            }
        }
    }
    for operation in binding.operations() {
        control.checkpoint()?;
        let full = destination_evidence_path(&operation.destination)?;
        let Some(parent_identity) =
            identities.get(&reference(full.parent().ok_or(E::CorruptJournal)?))
        else {
            continue;
        };
        let reference = reference(&full);
        if indices.insert(reference.clone(), records.len()).is_some() {
            return Err(E::CorruptJournal);
        }
        records.push(WindowsAlbumInspectionTarget {
            reference,
            identity: published.get(&operation.id).cloned(),
            parent_identity: parent_identity.clone(),
            kind: EntryKind::File,
        });
    }
    Ok((records, indices))
}
fn destination_evidence_path(destination: &DestinationEvidence) -> Result<PathBuf> {
    let mut full = path(&destination.root)?;
    for component in &destination.components {
        full.push(component);
    }
    Ok(full)
}

struct RecordedAlbumReads<'a, 'c> {
    owner: WindowsAlbumReadInspection,
    binding: &'a ExecutionBinding,
    source_indices: ReadIndices,
    target_indices: ReadIndices,
    present: bool,
    control: &'a InspectionControl<'c>,
    destination_operations: HashMap<String, usize>,
    operation_sources: HashSet<StorageRef>,
    move_sources: HashMap<StorageRef, &'a CopiedMoveSourceEvidence>,
    target_directories: Vec<usize>,
}
impl RecordedAlbumReads<'_, '_> {
    fn event(
        &self,
    ) -> impl FnMut(WindowsInspectionEvent) -> std::result::Result<(), MutationError> + '_ {
        |event| crate::windows_copy::inspection_event(self.control, event)
    }
    fn current(&self, reference: &StorageRef, source: bool) -> Result<Option<MutationObservation>> {
        self.control.checkpoint()?;
        if source && !self.present {
            if !self.source_indices.contains_key(reference) {
                return Err(E::Scope);
            }
            let state = self
                .owner
                .inspect_original_observed(self.control.cancellation_flag(), &mut self.event())
                .map_err(crate::windows_copy::inspection_error)?;
            return if state == WindowsAlbumOriginalLocation::Absent {
                Ok(None)
            } else {
                Err(E::Stale)
            };
        }
        let index = if source {
            self.source_indices.get(reference)
        } else {
            self.target_indices.get(reference)
        }
        .ok_or(E::Scope)?;
        let result = if source {
            self.owner.inspect_source_entry_observed(
                *index,
                self.control.cancellation_flag(),
                &mut self.event(),
            )
        } else {
            self.owner.inspect_entry_observed(
                *index,
                self.control.cancellation_flag(),
                &mut self.event(),
            )
        };
        result.map_err(crate::windows_copy::inspection_error)
    }
    fn observe(
        &self,
        reference: &StorageRef,
        source: bool,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        let Some(before) = self.current(reference, source)? else {
            return Ok(None);
        };
        let index = if source {
            self.source_indices.get(reference)
        } else {
            self.target_indices.get(reference)
        }
        .ok_or(E::Scope)?;
        let result = if source {
            self.owner.inspect_source_main_observed(
                *index,
                self.control.cancellation_flag(),
                &mut self.event(),
            )
        } else {
            self.owner.inspect_main_observed(
                *index,
                self.control.cancellation_flag(),
                &mut self.event(),
            )
        };
        let (actual, digest) = result.map_err(crate::windows_copy::inspection_error)?;
        if actual != before {
            return Err(E::Stale);
        }
        Ok(Some(crate::recovery::ObservedObject {
            identity: actual.identity,
            sha256: digest
                .sha256
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        }))
    }
    fn destination_index(&self, destination: &DestinationEvidence) -> Result<usize> {
        let operation = self
            .binding
            .operations()
            .get(
                *self
                    .destination_operations
                    .get(&destination.id)
                    .ok_or(E::Scope)?,
            )
            .ok_or(E::Scope)?;
        if operation.destination != *destination {
            return Err(E::Scope);
        }
        self.target_indices
            .get(&reference(&destination_evidence_path(destination)?))
            .copied()
            .ok_or(E::Scope)
    }
    fn metadata(&self, reference: &StorageRef) -> Result<Option<WindowsCopyMetadataObservation>> {
        let index = *self.target_indices.get(reference).ok_or(E::Scope)?;
        self.owner
            .inspect_metadata_observed(index, self.control.cancellation_flag(), &mut self.event())
            .map_err(crate::windows_copy::inspection_error)
    }
}
impl crate::recovery::RecoveryStorage for RecordedAlbumReads<'_, '_> {
    fn source(
        &mut self,
        reference: &StorageRef,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        if !self.operation_sources.contains(reference) {
            return Err(E::Scope);
        }
        self.observe(reference, true)
    }
    fn destination(
        &mut self,
        destination: &DestinationEvidence,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        self.destination_index(destination)?;
        self.observe(&reference(&destination_evidence_path(destination)?), false)
    }
    fn verify_copy_metadata(
        &mut self,
        destination: &DestinationEvidence,
        expected: &crate::physical_journal::CopyMetadataEvidence,
    ) -> Result<bool> {
        self.destination_index(destination)?;
        let operation = &self.binding.operations()[*self
            .destination_operations
            .get(&destination.id)
            .ok_or(E::Scope)?];
        if operation.source.observation.identity.as_ref() != Some(&expected.source_identity) {
            return Err(E::Scope);
        }
        Ok(self
            .metadata(&reference(&destination_evidence_path(destination)?))?
            .is_some_and(|actual| {
                actual.observation.identity == expected.destination_identity
                    && metadata_matches(&actual, expected)
            }))
    }
    fn verify_move_source_streams(&mut self, expected: &CopiedMoveSourceEvidence) -> Result<bool> {
        if !self.present || self.move_sources.get(&expected.source).copied() != Some(expected) {
            return Err(E::Scope);
        }
        let index = *self.source_indices.get(&expected.source).ok_or(E::Scope)?;
        let actual = self
            .owner
            .inspect_source_streams_observed(
                index,
                self.control.cancellation_flag(),
                &mut self.event(),
            )
            .map_err(crate::windows_copy::inspection_error)?;
        Ok(actual.identity == expected.identity
            && actual.attributes == expected.attributes
            && crate::windows_copy::move_streams_match(&actual, expected))
    }
    fn verify_move_destination_streams(
        &mut self,
        destination: &DestinationEvidence,
        identity: &ObjectIdentity,
        expected: &CopiedMoveSourceEvidence,
    ) -> Result<bool> {
        let index = self.destination_index(destination)?;
        let operation = &self.binding.operations()[*self
            .destination_operations
            .get(&destination.id)
            .ok_or(E::Scope)?];
        if self.move_sources.get(&expected.source).copied() != Some(expected)
            || operation.source.observation.reference != expected.source
        {
            return Err(E::Scope);
        }
        let actual = self
            .owner
            .inspect_streams_observed(index, self.control.cancellation_flag(), &mut self.event())
            .map_err(crate::windows_copy::inspection_error)?;
        Ok(actual.identity == *identity
            && crate::windows_copy::move_streams_match(&actual, expected))
    }
}
fn metadata_matches(
    actual: &WindowsCopyMetadataObservation,
    expected: &crate::physical_journal::CopyMetadataEvidence,
) -> bool {
    actual.creation_time == expected.retained_creation_time
        && actual.access_time == expected.access_time
        && actual.write_time == expected.write_time
        && actual.readonly == expected.readonly
        && actual.retained_attributes == expected.retained_attributes
}
fn inspect_directory_metadata(
    report: &mut SelectedAlbumRecovery,
    storage: &RecordedAlbumReads,
) -> Result<()> {
    storage.control.stage("album_directory_metadata")?;
    for receipt in &report.directory_metadata {
        storage.control.checkpoint()?;
        let result = storage.metadata(&receipt.destination);
        storage.control.checkpoint()?;
        let status = match result {
            Ok(Some(actual))
                if actual.observation.identity == receipt.evidence.destination_identity
                    && metadata_matches(&actual, &receipt.evidence) =>
            {
                "verified"
            }
            Ok(Some(_)) => "changed",
            Ok(None) => "missing",
            Err(E::Cancelled) => return Err(E::Cancelled),
            Err(_) => "unverified",
        };
        report
            .directory_metadata_status
            .push((receipt.destination.clone(), status.into()));
    }
    Ok(())
}
fn inspect_directories(
    report: &mut SelectedAlbumRecovery,
    observed: &[(StorageRef, ObjectIdentity)],
    storage: &RecordedAlbumReads,
) -> Result<Vec<WindowsAlbumInspectionDirectory>> {
    storage.control.stage("album_directories")?;
    let source_root = &storage.binding.data.source_root.reference;
    let source_path = path(source_root)?;
    let mut pins = vec![];
    for index in &storage.target_directories {
        storage.control.checkpoint()?;
        pins.push(
            storage
                .owner
                .pin_directory_observed(
                    *index,
                    storage.control.cancellation_flag(),
                    &mut storage.event(),
                )
                .map_err(crate::windows_copy::inspection_error)?,
        );
    }
    for (reference, identity) in observed {
        storage.control.checkpoint()?;
        let source = path(reference)?.starts_with(&source_path);
        let current = if reference == source_root {
            match storage
                .owner
                .inspect_original_observed(
                    storage.control.cancellation_flag(),
                    &mut storage.event(),
                )
                .map_err(crate::windows_copy::inspection_error)?
            {
                WindowsAlbumOriginalLocation::Absent => Ok(None),
                WindowsAlbumOriginalLocation::OriginalPresent(actual)
                | WindowsAlbumOriginalLocation::ForeignPresent(actual) => Ok(Some(actual)),
            }
        } else {
            storage.current(reference, source)
        };
        storage.control.checkpoint()?;
        let mut evidence = AlbumDirectoryEvidence {
            reference: reference.clone(),
            expected_identity: identity.clone(),
            observed_identity: None,
            status: AlbumDirectoryEvidenceStatus::Unverified,
        };
        match current {
            Ok(Some(actual)) => {
                evidence.observed_identity = Some(actual.identity.clone());
                evidence.status = if actual.size.is_none() && actual.identity == *identity {
                    AlbumDirectoryEvidenceStatus::SameIdentity
                } else {
                    AlbumDirectoryEvidenceStatus::Replaced
                };
            }
            Ok(None) => evidence.status = AlbumDirectoryEvidenceStatus::Absent,
            Err(E::Cancelled) => return Err(E::Cancelled),
            Err(_) => {}
        }
        if !source && !matches!(evidence.status, AlbumDirectoryEvidenceStatus::SameIdentity) {
            return Err(E::Stale);
        }
        report.directories.push(evidence);
    }
    Ok(pins)
}
