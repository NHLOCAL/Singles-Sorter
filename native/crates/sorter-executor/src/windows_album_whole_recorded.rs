//! Untrusted Whole-Album facts for explicit fresh root selection. No authority
//! or completion assertion is returned from recorded bytes.
use super::*;
use crate::inspection_control::{MAX_OPERATIONS, check_recorded_input};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedWholeAlbumJob {
    pub job_id: String,
    pub header_sha256: String,
    pub plan_sha256: String,
    pub source: EntryObservation,
    pub target: EntryObservation,
    pub original_source: Option<MutationObservation>,
    pub albums: Vec<RecordedWholeAlbum>,
    pub files: Vec<RecordedWholeAlbumFile>,
    pub directories: Vec<RecordedWholeAlbumDirectory>,
    pub child_locations: Vec<RecordedWholeAlbumChildLocation>,
    pub singles_job_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedWholeAlbum {
    pub operation: usize,
    pub source: EntryObservation,
    pub original: MutationObservation,
    pub destination: Destination,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedWholeAlbumFile {
    pub inspection_index: usize,
    pub operation: usize,
    pub source: StorageRef,
    pub destination: StorageRef,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedWholeAlbumDirectory {
    pub operation: usize,
    pub source: StorageRef,
    pub prepared_source: StorageRef,
    pub destination: StorageRef,
    pub identity: ObjectIdentity,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordedWholeAlbumChildLocation {
    pub operation: usize,
    pub planner_index: usize,
    pub original: StorageRef,
    pub prepared: StorageRef,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeader {
    domain: String,
    plan_sha256: String,
    source: EntryObservation,
    target: EntryObservation,
    observed_target: StorageRef,
    observed_sources: Vec<MutationObservation>,
    singles_binding: Option<BindingData>,
    singles_job_id: Option<String>,
    scope: ScopeEvidence,
    snapshot: super::super::recorded::RecordedSnapshot,
}
fn invalid<T>() -> Result<T> {
    Err(E::CorruptJournal)
}
fn hash_string(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn local_path(value: &StorageRef) -> Result<PathBuf> {
    let StorageRef::WindowsPath { utf16 } = value else {
        return invalid();
    };
    if utf16.contains(&0) {
        return invalid();
    }
    // Check the lossless spelling first: Path::components normalizes interior '.'.
    let raw = utf16
        .split(|c| *c == b'\\' as u16 || *c == b'/' as u16)
        .collect::<Vec<_>>();
    if raw
        .iter()
        .any(|part| *part == [b'.' as u16] || *part == [b'.' as u16, b'.' as u16])
    {
        return invalid();
    }
    let path = path(value)?;
    let mut parts = path.components();
    match parts.next() {
        Some(std::path::Component::Prefix(prefix))
            if matches!(
                prefix.kind(),
                std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
            ) => {}
        _ => return invalid(),
    }
    if !matches!(parts.next(), Some(std::path::Component::RootDir)) {
        return invalid();
    }
    for part in parts {
        let std::path::Component::Normal(part) = part else {
            return invalid();
        };
        if part.encode_wide().any(|c| c == b':' as u16 || c == 0) {
            return invalid();
        }
    }
    Ok(path)
}
fn scope_hash(scope: &ScopeEvidence) -> Result<String> {
    crate::hash(&(
        &scope.inspected_plan,
        &scope.source_root,
        &scope.target_root,
        &scope.inspected_sources,
        &scope.resources,
        &scope.allowed_modes,
    ))
}
fn validate(raw: RawHeader, control: &InspectionControl) -> Result<Header> {
    control.checkpoint()?;
    if raw.domain != "SelectedWindowsWholeAlbumRenameV1"
        || !hash_string(&raw.plan_sha256)
        || raw.scope.inspected_plan != raw.plan_sha256
        || raw.source.identity.as_ref() != Some(&raw.scope.source_root)
        || raw.target.identity.as_ref() != Some(&raw.scope.target_root)
        || !matches!((&raw.scope.source_root,&raw.scope.target_root),(ObjectIdentity::Windows{volume:a,..},ObjectIdentity::Windows{volume:b,..}) if a==b)
        || raw.scope.allowed_modes != [TransferMode::Move]
    {
        return invalid();
    }
    let resources = &raw.scope.resources;
    if resources.resource_ids.len() != resources.resource_hashes.len()
        || resources.adapter_versions.len() != resources.adapter_hashes.len()
        || !resources
            .resource_hashes
            .iter()
            .chain(&resources.adapter_hashes)
            .all(|h| hash_string(h))
        || !matches!(
            (&resources.ai_profile, &resources.model_hash),
            (AiProfile::Disabled, None)
        ) && !matches!((&resources.ai_profile,&resources.model_hash),(AiProfile::Configured{available:true,..},Some(hash)) if hash_string(hash))
    {
        return invalid();
    }
    let snapshot = raw.snapshot.into_snapshot(control)?;
    super::super::recorded::validate_snapshot_projection(
        &raw.source,
        &raw.target,
        false,
        &snapshot,
        control,
    )?;
    if snapshot.albums.is_empty()
        || snapshot.albums.len() != raw.observed_sources.len()
        || raw.scope.inspected_sources.len() > MAX_OPERATIONS
    {
        return invalid();
    }
    local_path(&raw.observed_target)?;
    let selected_source = local_path(&raw.source.reference)?;
    let mut native_source_root = None;
    for (index, (album, observed)) in snapshot
        .albums
        .iter()
        .zip(&raw.observed_sources)
        .enumerate()
    {
        control.checkpoint()?;
        let native = local_path(&observed.reference)?;
        if album.operation != index
            || observed.size.is_some()
            || Some(&observed.identity) != album.source.identity.as_ref()
            || Some(&observed.parent_identity) != album.source.parent_identity.as_ref()
            || native
                .file_name()
                .map(|n| MutationName::Windows(n.encode_wide().collect()))
                .as_ref()
                != Some(&observed.actual_name)
        {
            return invalid();
        }
        let selected = local_path(&album.source.reference)?;
        let relative = selected
            .strip_prefix(&selected_source)
            .map_err(|_| E::CorruptJournal)?;
        let mut root = native.clone();
        for component in relative.components().rev() {
            if root.file_name() != Some(component.as_os_str()) || !root.pop() {
                return invalid();
            }
        }
        if native_source_root
            .as_ref()
            .is_some_and(|prior| *prior != root)
        {
            return invalid();
        }
        native_source_root = Some(root);
        if relative.as_os_str().is_empty() && snapshot.albums.len() != 1 {
            return invalid();
        }
    }
    let mut seen = HashSet::new();
    let mut expected_sources = Vec::new();
    let mut evidence_by_ref = HashMap::new();
    for evidence in &raw.scope.inspected_sources {
        control.checkpoint()?;
        if !hash_string(&evidence.sha256)
            || evidence_by_ref
                .insert(&evidence.observation.reference, evidence)
                .is_some()
        {
            return invalid();
        }
    }
    for file in &snapshot.files {
        control.checkpoint()?;
        let evidence = evidence_by_ref
            .get(&file.source.reference)
            .ok_or(E::CorruptJournal)?;
        if evidence.observation != file.source {
            return invalid();
        }
        if seen.insert(&file.source.reference) {
            expected_sources.push((*evidence).clone());
        }
    }
    if expected_sources != raw.scope.inspected_sources {
        return invalid();
    }
    let singles = snapshot
        .files
        .iter()
        .filter(|file| file.original_operation >= snapshot.albums.len())
        .collect::<Vec<_>>();
    match (
        &raw.singles_binding,
        &raw.singles_job_id,
        singles.is_empty(),
    ) {
        (None, None, true) => {}
        (Some(binding), Some(job), false) => {
            if !job.starts_with("album-")
                || job.len() > 100
                || !job.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return invalid();
            }
            let mut scope = raw.scope.clone();
            let single_refs = singles
                .iter()
                .map(|f| (&f.source.reference, &f.source))
                .collect::<HashMap<_, _>>();
            let mut inspected_sources = Vec::new();
            for source in &scope.inspected_sources {
                control.checkpoint()?;
                if single_refs
                    .get(&source.observation.reference)
                    .is_some_and(|entry| **entry == source.observation)
                {
                    inspected_sources.push(source.clone());
                }
            }
            scope.inspected_sources = inspected_sources;
            let mut operations = Vec::new();
            for (id, file) in singles.iter().enumerate() {
                control.checkpoint()?;
                if file.original_operation != snapshot.albums.len() + id {
                    return invalid();
                }
                operations.push(BoundOperation {
                    id,
                    depends_on: if id == 0 { vec![] } else { vec![id - 1] },
                    source: (*evidence_by_ref
                        .get(&file.source.reference)
                        .ok_or(E::CorruptJournal)?)
                    .clone(),
                    destination: DestinationEvidence {
                        id: crate::hash(&file.destination)?,
                        root: file.destination.root.clone(),
                        components: file.destination.components.clone(),
                    },
                    mode: TransferMode::Move,
                    copy_effect: false,
                    duet_cohort: None,
                });
            }
            let expected = BindingData {
                schema: 2,
                album_context: None,
                copy_metadata_version: None,
                copied_move_sources: None,
                plan_sha256: raw.plan_sha256.clone(),
                source_root: raw.source.clone(),
                target_root: raw.target.clone(),
                resources: resources.clone(),
                operations,
                scope_sha256: scope_hash(&scope)?,
                scope_evidence: scope,
                synthetic_only: false,
                owned_windows_copy: false,
                owned_windows_rename: false,
                selected_windows_copy: false,
                selected_windows_rename: true,
            };
            if *binding != expected
                || snapshot
                    .albums
                    .iter()
                    .any(|a| a.source.reference == raw.source.reference)
            {
                return invalid();
            }
        }
        _ => return invalid(),
    }
    Ok(Header {
        domain: "SelectedWindowsWholeAlbumRenameV1",
        plan_sha256: raw.plan_sha256,
        source: raw.source,
        target: raw.target,
        observed_target: raw.observed_target,
        observed_sources: raw.observed_sources,
        singles_binding: raw.singles_binding,
        singles_job_id: raw.singles_job_id,
        scope: raw.scope,
        snapshot,
    })
}

pub(super) fn decode(
    bytes: &[u8],
    control: &InspectionControl,
) -> Result<(String, Header, WholeAlbumRecovery)> {
    control.stage("whole_album_descriptor")?;
    check_recorded_input(bytes, Some(control))?;
    let end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .ok_or(E::CorruptJournal)?;
    let envelope: ManifestEnvelope =
        serde_json::from_slice(&bytes[..end]).map_err(|_| E::CorruptJournal)?;
    if envelope.body.sequence != 0
        || !envelope.body.previous.is_empty()
        || envelope.body.phase != "Header"
        || crate::hash(&envelope.body)? != envelope.sha256
        || !envelope.body.job_id.starts_with("album-move-")
        || envelope.body.job_id.len() > 100
        || !envelope
            .body
            .job_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return invalid();
    }
    let raw =
        serde_json::from_value(envelope.body.payload.clone()).map_err(|_| E::CorruptJournal)?;
    let header = validate(raw, control)?;
    if serde_json::to_value(&header).map_err(|_| E::Io)? != envelope.body.payload {
        return invalid();
    }
    let report = recorded_manifest::WholeManifestEvidence {
        header: &header,
        job_id: &envelope.body.job_id,
    }
    .read_manifest_controlled(bytes, control)?;
    Ok((envelope.body.job_id, header, report))
}
pub(super) fn descriptor(
    job_id: String,
    header: &Header,
    control: &InspectionControl,
) -> Result<RecordedWholeAlbumJob> {
    let mut albums = Vec::new();
    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut child_locations = Vec::new();
    for (album, original) in header.snapshot.albums.iter().zip(&header.observed_sources) {
        control.checkpoint()?;
        albums.push(RecordedWholeAlbum {
            operation: album.operation,
            source: album.source.clone(),
            original: original.clone(),
            destination: album.destination.clone(),
        });
        for directory in &header.snapshot.directories {
            control.checkpoint()?;
            if path(&directory.source.reference)?.starts_with(path(&album.source.reference)?) {
                directories.push(RecordedWholeAlbumDirectory {
                    operation: album.operation,
                    source: directory.source.reference.clone(),
                    prepared_source: mapped(album, &directory.source.reference, false)?,
                    destination: mapped(album, &directory.source.reference, true)?,
                    identity: directory.source.identity.clone().ok_or(E::CorruptJournal)?,
                });
            }
        }
        for (planner_index, child) in album.children.iter().enumerate() {
            control.checkpoint()?;
            if leaf(&child.source)? != name(&child.destination_name) {
                child_locations.push(RecordedWholeAlbumChildLocation {
                    operation: album.operation,
                    planner_index,
                    original: child.source.reference.clone(),
                    prepared: reference(
                        &path(&album.source.reference)?.join(&child.destination_name),
                    ),
                });
            }
        }
    }
    for (inspection_index, file) in header.snapshot.files.iter().enumerate() {
        control.checkpoint()?;
        files.push(RecordedWholeAlbumFile {
            inspection_index,
            operation: file.original_operation,
            source: file.source.reference.clone(),
            destination: reference(&destination_path(&file.destination)?),
        });
    }
    Ok(RecordedWholeAlbumJob {
        job_id,
        header_sha256: crate::hash(header)?,
        plan_sha256: header.plan_sha256.clone(),
        source: header.source.clone(),
        target: header.target.clone(),
        original_source: albums
            .iter()
            .find(|a| a.source.reference == header.source.reference)
            .map(|a| a.original.clone()),
        albums,
        files,
        directories,
        child_locations,
        singles_job_id: header.singles_job_id.clone(),
    })
}
pub fn inspect_recorded_whole_album_job(
    bytes: &[u8],
    control: &InspectionControl,
) -> Result<RecordedWholeAlbumJob> {
    let (job, header, _) = decode(bytes, control)?;
    descriptor(job, &header, control)
}

pub(super) fn mapped(
    album: &AlbumGroup,
    source: &StorageRef,
    destination: bool,
) -> Result<StorageRef> {
    let source_path = path(source)?;
    let album_path = path(&album.source.reference)?;
    let relative = source_path
        .strip_prefix(&album_path)
        .map_err(|_| E::CorruptJournal)?;
    let mut parts = relative.components();
    let mut output = if destination {
        destination_path(&album.destination)?
    } else {
        album_path
    };
    if let Some(first) = parts.next() {
        let child = album
            .children
            .iter()
            .find(|child| {
                path(&child.source.reference)
                    .ok()
                    .as_deref()
                    .and_then(|p| p.file_name())
                    == Some(first.as_os_str())
            })
            .ok_or(E::CorruptJournal)?;
        output.push(&child.destination_name);
        for part in parts {
            output.push(part.as_os_str());
        }
    }
    Ok(reference(&output))
}
