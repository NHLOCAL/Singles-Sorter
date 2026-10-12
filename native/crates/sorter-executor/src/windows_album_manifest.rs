//! Data-only strict file-Album phase replay shared by live and restart readers.
//! This context has no native roots, writable session or constructor from consent.
use super::*;

pub(super) struct FileManifestEvidence<'a> {
    pub(super) binding: &'a ExecutionBinding,
    pub(super) snapshot: &'a Snapshot,
    pub(super) job_id: &'a str,
    pub(super) observed_source_root: &'a StorageRef,
    pub(super) original_source: &'a Option<MutationObservation>,
}

impl FileManifestEvidence<'_> {
    pub(super) fn read_manifest(
        &self,
        bytes: &[u8],
        control: &InspectionControl,
    ) -> Result<(SelectedAlbumRecovery, Vec<(StorageRef, ObjectIdentity)>)> {
        let cancel = control.cancellation_flag();
        crate::inspection_control::check_recorded_input(bytes, self.binding.data.copied_move_sources.is_some().then_some(control))?;
        if bytes.len() > MAX_MANIFEST {
            return Err(E::Unsupported);
        }
        let mut report = SelectedAlbumRecovery {
            directory_metadata: vec![],
            directory_metadata_status: vec![],
            pending_metadata: vec![],
            completed_albums: vec![],
            album_transfers: vec![],
            recorded_status: None,
            recorded_transfers: None,
            whole: None,
            manifest_finished: false,
            incomplete_tail: false,
            pending_directories: vec![],
            pending_cleanup: vec![],
            directories: vec![],
            removed_directory_references: vec![],
            transfers: None,
        };
        let mut observed = Vec::new();
        let mut sequence = 0;
        let mut previous = String::new();
        let mut offset = 0;
        let mut header = false;
        let mut pending_create: Option<(StorageRef, ObjectIdentity, String)> = None;
        let mut pending_cleanup: Option<EntryObservation> = None;
        let mut transfers_prepared = false;
        let mut transfers_observed = false;
        let mut created = HashSet::new();
        let mut created_evidence = Vec::<MutationObservation>::new();
        let mut removed_evidence = Vec::<MutationObservation>::new();
        let mut retained_evidence = Vec::<RetainedDirectory>::new();
        let mut transfer_status = None::<String>;
        let mut cleaned = HashSet::new();
        let mut stopped_status = None::<String>;
        let mut pending_metadata = None::<(usize, ObjectIdentity)>;
        let mut metadata_stopped = None::<String>;
        let mut album_completion_stopped = None::<String>;
        while offset < bytes.len() {
            control.record()?;
            cancelled(cancel)?;
            let Some(end) = bytes[offset..].iter().position(|b| *b == b'\n') else {
                report.incomplete_tail = true;
                break;
            };
            let record: ManifestEnvelope = serde_json::from_slice(&bytes[offset..offset + end])
                .map_err(|_| E::CorruptJournal)?;
            offset += end + 1;
            if report.manifest_finished
                || record.body.sequence != sequence
                || record.body.previous != previous
                || record.body.job_id != self.job_id
                || crate::hash(&record.body)? != record.sha256
            {
                return Err(E::CorruptJournal);
            }
            sequence += 1;
            previous = record.sha256;
            let record = record.body;
            if stopped_status.is_some() && record.phase != "Finished" {
                return Err(E::CorruptJournal);
            }
            if album_completion_stopped.is_some() && !matches!(record.phase.as_str(), "TransfersObserved" | "Finished") { return Err(E::CorruptJournal); }
            match record.phase.as_str() {
                "Header" => {
                    if header
                        || sequence != 1
                        || record.payload
                            != album_header(self.binding, self.snapshot, self.original_source.as_ref())
                    {
                        return Err(E::CorruptJournal);
                    }
                    header = true;
                }
                "DirectoryPrepared" => {
                    if !header || pending_create.is_some() || transfers_prepared {
                        return Err(E::CorruptJournal);
                    }
                    let reference: StorageRef =
                        serde_json::from_value(record.payload["reference"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let parent: ObjectIdentity =
                        serde_json::from_value(record.payload["parent"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let leaf = record.payload["name"]
                        .as_str()
                        .ok_or(E::CorruptJournal)?
                        .to_owned();
                    if !self.snapshot.destinations.iter().any(|d| {
                        d.existing.is_none()
                            && destination_path(&d.destination)
                                .is_ok_and(|p| self::reference(&p) == reference)
                            && d.destination.components.last() == Some(&leaf)
                    }) || created.contains(&reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if directory_parent_identity(
                        self.snapshot,
                        &self.binding.data.target_root.reference,
                        &self.binding.data.scope_evidence.target_root,
                        &reference,
                        &created_evidence,
                    )? != parent
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_create = Some((reference, parent, leaf));
                }
                "DirectoryObserved" => {
                    let observation: MutationObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let (reference, parent, leaf) =
                        pending_create.take().ok_or(E::CorruptJournal)?;
                    if observation.reference != reference
                        || observation.parent_identity != parent
                        || observation.actual_name != name(&leaf)
                        || observation.size.is_some()
                        || !new_directory_identity(
                            self.snapshot,
                            &self.binding.data.scope_evidence.source_root,
                            &self.binding.data.scope_evidence.target_root,
                            &observation.identity,
                            &created_evidence,
                        )
                    {
                        return Err(E::CorruptJournal);
                    }
                    created.insert(reference.clone());
                    created_evidence.push(observation.clone());
                    observed.push((reference, observation.identity));
                }
                "TransfersPrepared" => {
                    if !header
                        || pending_create.is_some()
                        || transfers_prepared
                        || record.payload["binding_sha256"] != self.binding.sha256()?
                        || self
                            .snapshot
                            .destinations
                            .iter()
                            .filter(|d| d.existing.is_none())
                            .any(|d| {
                                destination_path(&d.destination)
                                    .map_or(true, |p| !created.contains(&reference(&p)))
                            })
                    {
                        return Err(E::CorruptJournal);
                    }
                    transfers_prepared = true;
                }
                "TransfersObserved" => {
                    if !transfers_prepared || transfers_observed || pending_metadata.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    if record.payload["committed_destinations"]
                        .as_u64()
                        .is_none_or(|n| n > self.binding.operations().len() as u64)
                        || record.payload["moved_sources"].as_u64().is_none_or(|n| {
                            n > self.binding.data.scope_evidence.inspected_sources.len() as u64
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(
                        status,
                        "Complete" | "Partial" | "Cancelled" | "NeedsRecovery"
                    ) || (status == "Complete"
                        && (record.payload["committed_destinations"].as_u64()
                            != Some(self.binding.operations().len() as u64)
                            || record.payload["moved_sources"].as_u64()
                                != Some(if self.binding.data.selected_windows_copy {
                                    0
                                } else {
                                    self.binding.data.scope_evidence.inspected_sources.len() as u64
                                })))
                    {
                        return Err(E::CorruptJournal);
                    }
                    if album_completion_stopped.as_deref().is_some_and(|expected| expected != status) { return Err(E::CorruptJournal); }
                    if metadata_stopped
                        .as_deref()
                        .is_some_and(|expected| expected != status)
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_transfers = Some((
                        status.to_owned(),
                        record.payload["committed_destinations"]
                            .as_u64()
                            .ok_or(E::CorruptJournal)? as usize,
                        record.payload["moved_sources"]
                            .as_u64()
                            .ok_or(E::CorruptJournal)? as usize,
                    ));
                    transfer_status = Some(status.to_owned());
                    transfers_observed = true;
                }
                "DirectoryMetadataPrepared" => {
                    if !transfers_prepared
                        || transfers_observed
                        || !(self.binding.data.selected_windows_copy || self.binding.data.copied_move_sources.is_some())
                        || pending_metadata.is_some()
                        || metadata_stopped.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let index = self
                        .snapshot
                        .copied_directories
                        .iter()
                        .position(|directory| {
                            record.payload["source"]
                                == serde_json::to_value(&directory.source).unwrap_or_default()
                                && record.payload["destination"]
                                    == serde_json::to_value(&directory.destination)
                                        .unwrap_or_default()
                        })
                        .ok_or(E::CorruptJournal)?;
                    let directory = &self.snapshot.copied_directories[index];
                    if self.binding.data.copied_move_sources.is_some() && !report.album_transfers.iter().any(|ack| ack.operation == directory.operation) { return Err(E::CorruptJournal); }
                    let destination = reference(&destination_path(&directory.destination)?);
                    let created = created_evidence
                        .iter()
                        .find(|entry| entry.reference == destination)
                        .ok_or(E::CorruptJournal)?;
                    if record.payload["created"]
                        != serde_json::to_value(created).map_err(|_| E::Io)?
                        || report
                            .directory_metadata
                            .iter()
                            .any(|receipt| receipt.destination == destination)
                        || self.snapshot.copied_directories[..index]
                            .iter()
                            .filter(|prior| prior.operation == directory.operation)
                            .any(|prior| {
                                destination_path(&prior.destination).map_or(true, |p| {
                                    !report
                                        .directory_metadata
                                        .iter()
                                        .any(|receipt| receipt.destination == reference(&p))
                                })
                            })
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_metadata = Some((index, created.identity.clone()));
                }
                "DirectoryMetadataObserved" => {
                    let (index, id) = pending_metadata.take().ok_or(E::CorruptJournal)?;
                    let directory = &self.snapshot.copied_directories[index];
                    let receipt: AlbumDirectoryMetadataReceipt =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if receipt.destination != reference(&destination_path(&directory.destination)?)
                        || receipt.evidence.destination_identity != id
                        || Some(&receipt.evidence.source_identity)
                            != directory.source.identity.as_ref()
                        || receipt.evidence.access_time <= 0
                        || receipt.evidence.write_time <= 0
                        || receipt.evidence.retained_attributes & 0x81 != 0
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.directory_metadata.push(receipt);
                }
                "DirectoryMetadataStopped" => {
                    let (index, _) = pending_metadata.take().ok_or(E::CorruptJournal)?;
                    let destination = reference(&destination_path(
                        &self.snapshot.copied_directories[index].destination,
                    )?);
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery")
                        || record.payload["destination"]
                            != serde_json::to_value(&destination).map_err(|_| E::Io)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    if status == "NeedsRecovery" {
                        report.pending_metadata.push(destination);
                    }
                    metadata_stopped = Some(status.to_owned());
                }
                "AlbumCompletionStopped" => {
                    let operation = record.payload["operation"].as_u64().and_then(|value| usize::try_from(value).ok()).ok_or(E::CorruptJournal)?;
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if self.binding.data.copied_move_sources.is_none() || transfers_observed
                        || !report.album_transfers.last().is_some_and(|ack| ack.operation == operation)
                        || report.completed_albums.contains(&operation)
                        || !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery")
                        || metadata_stopped.as_deref().is_some_and(|prior| prior != status)
                        || (pending_metadata.is_some() || pending_cleanup.is_some() || !report.pending_cleanup.is_empty()) && status != "NeedsRecovery"
                        || record.payload != serde_json::json!({"operation":operation,"status":status})
                    { return Err(E::CorruptJournal); }
                    album_completion_stopped = Some(status.to_owned());
                }
                "AlbumTransfersObserved" => {
                    if !transfers_prepared || transfers_observed || pending_cleanup.is_some()
                        || pending_metadata.is_some() || metadata_stopped.is_some()
                        || self.binding.data.copied_move_sources.is_none()
                    { return Err(E::CorruptJournal); }
                    let ack: AlbumTransferAcknowledgment = serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let album = self.snapshot.albums.get(report.album_transfers.len()).ok_or(E::CorruptJournal)?;
                    if report.album_transfers.len() != report.completed_albums.len() { return Err(E::CorruptJournal); }
                    ack.validate(album, self.binding)?;
                    report.album_transfers.push(ack);
                }
                "AlbumCompleted" => {
                    if !transfers_prepared
                        || pending_metadata.is_some()
                        || metadata_stopped.is_some()
                        || (self.binding.data.selected_windows_copy && transfers_observed)
                        || (!self.binding.data.selected_windows_copy && self.binding.data.copied_move_sources.is_none() && transfer_status.as_deref() != Some("Complete"))
                    { return Err(E::CorruptJournal); }
                    let operation = record.payload["operation"]
                        .as_u64()
                        .and_then(|id| usize::try_from(id).ok())
                        .ok_or(E::CorruptJournal)?;
                    let album = self
                        .snapshot
                        .albums
                        .iter()
                        .find(|album| album.operation == operation)
                        .ok_or(E::CorruptJournal)?;
                    if self.binding.data.copied_move_sources.is_some() && (transfers_observed
                        || !report.album_transfers.iter().any(|ack| ack.operation == operation)) { return Err(E::CorruptJournal); }
                    if report.completed_albums.contains(&operation)
                        || record.payload
                            != serde_json::json!({"operation":operation,"files":album.files})
                    {
                        return Err(E::CorruptJournal);
                    }
                    if (self.binding.data.selected_windows_copy || self.binding.data.copied_move_sources.is_some()) && self
                        .snapshot
                        .copied_directories
                        .iter()
                        .filter(|directory| directory.operation == operation)
                        .any(|directory| {
                            destination_path(&directory.destination).map_or(true, |p| {
                                !report
                                    .directory_metadata
                                    .iter()
                                    .any(|receipt| receipt.destination == reference(&p))
                            })
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    if !self.binding.data.selected_windows_copy && album.cleanup.iter().any(|entry| !cleaned.contains(&entry.reference)) {
                        return Err(E::CorruptJournal);
                    }
                    report.completed_albums.push(operation);
                }
                "CleanupPrepared" => {
                    if (self.binding.data.copied_move_sources.is_none() && transfer_status.as_deref() != Some("Complete"))
                        || (self.binding.data.copied_move_sources.is_some() && transfers_observed)
                        || self.binding.data.selected_windows_copy
                        || pending_cleanup.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let directory: EntryObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if !self
                        .snapshot
                        .albums
                        .iter()
                        .any(|a| a.cleanup.contains(&directory))
                        || cleaned.contains(&directory.reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if self.binding.data.copied_move_sources.is_some() {
                        let album = self.snapshot.albums.iter().find(|album| album.cleanup.contains(&directory)).ok_or(E::CorruptJournal)?;
                        if !report.album_transfers.iter().any(|ack| ack.operation == album.operation) { return Err(E::CorruptJournal); }
                        for required in self.snapshot.copied_directories.iter().filter(|required| required.operation == album.operation) {
                            let target = reference(&destination_path(&required.destination)?);
                            if !report.directory_metadata.iter().any(|receipt| receipt.destination == target) { return Err(E::CorruptJournal); }
                        }
                    }
                    pending_cleanup = Some(directory);
                }
                "CleanupObserved" | "CleanupUnknown" => {
                    let observation: MutationObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let directory = pending_cleanup.take().ok_or(E::CorruptJournal)?;
                    let relative = path(&directory.reference)?.strip_prefix(path(&self.binding.data.source_root.reference)?).map_err(|_| E::CorruptJournal)?.to_path_buf();
                    let expected_name = if directory.reference == self.binding.data.source_root.reference {
                        self.original_source.as_ref().map(|source| source.actual_name.clone()).unwrap_or(leaf(&directory)?)
                    } else { leaf(&directory)? };
                    if path(&observation.reference)? != path(self.observed_source_root)?.join(relative)
                        || observation.size.is_some()
                        || Some(&observation.identity) != directory.identity.as_ref()
                        || Some(&observation.parent_identity) != directory.parent_identity.as_ref()
                        || observation.actual_name != expected_name
                    {
                        return Err(E::CorruptJournal);
                    }
                    if record.phase == "CleanupObserved" {
                        cleaned.insert(directory.reference.clone());
                        removed_evidence.push(observation.clone());
                        report.removed_directory_references.push((directory.reference.clone(), observation.identity.clone()));
                    } else {
                        report.pending_cleanup.push(directory.reference.clone());
                    }
                    observed.push((directory.reference, observation.identity));
                }
                "DirectoryRetained" => {
                    if !transfers_observed && (self.binding.data.copied_move_sources.is_none() || pending_cleanup.is_none()) {
                        return Err(E::CorruptJournal);
                    }
                    let retained: RetainedDirectory =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if !self
                        .snapshot
                        .albums
                        .iter()
                        .flat_map(|a| &a.cleanup)
                        .any(|d| {
                            d.reference == retained.reference
                                && d.identity.as_ref() == Some(&retained.identity)
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    if let Some(pending) = pending_cleanup.take()
                        && pending.reference != retained.reference
                    {
                        return Err(E::CorruptJournal);
                    }
                    if retained_evidence
                        .iter()
                        .any(|r| r.reference == retained.reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    retained_evidence.push(retained.clone());
                    observed.push((retained.reference, retained.identity));
                }
                "ExecutionStopped" => {
                    if !header || stopped_status.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery") {
                        return Err(E::CorruptJournal);
                    }
                    let valid = match record.payload["stage"].as_str() {
                        Some("directories") => !transfers_prepared,
                        Some("transfer_journal") => {
                            !transfers_prepared
                                && pending_create.is_none()
                                && status == "NeedsRecovery"
                        }
                        Some("membership") => {
                            transfer_status.as_deref() == Some("Complete") && matches!(status, "Partial" | "Cancelled")
                        }
                        Some("cleanup") => {
                            transfer_status.as_deref() == Some("Complete")
                                && !self.binding.data.selected_windows_copy
                        }
                        _ => false,
                    };
                    if !valid {
                        return Err(E::CorruptJournal);
                    }
                    stopped_status = Some(status.to_owned());
                }
                "Finished" => {
                    if !header || pending_metadata.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(
                        status,
                        "Complete" | "Partial" | "Cancelled" | "NeedsRecovery"
                    ) || record.payload["created"]
                        != serde_json::to_value(&created_evidence).map_err(|_| E::Io)?
                        || record.payload["removed"]
                            != serde_json::to_value(&removed_evidence).map_err(|_| E::Io)?
                        || record.payload["retained"]
                            != serde_json::to_value(&retained_evidence).map_err(|_| E::Io)?
                        || record.payload["albums_completed"].as_u64()
                            != Some(report.completed_albums.len() as u64)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if status == "Complete"
                        && (pending_metadata.is_some()
                            || !report.pending_metadata.is_empty()
                            || report.completed_albums.len() != self.snapshot.albums.len()
                            || transfer_status.as_deref() != Some("Complete")
                            || pending_create.is_some()
                            || pending_cleanup.is_some()
                            || !report.pending_cleanup.is_empty()
                            || !retained_evidence.is_empty())
                    {
                        return Err(E::CorruptJournal);
                    }
                    if (transfer_status.as_deref() == Some("NeedsRecovery")
                        || !report.pending_cleanup.is_empty())
                        && status != "NeedsRecovery"
                    {
                        return Err(E::CorruptJournal);
                    }
                    let expected = stopped_status
                        .as_deref()
                        .or(album_completion_stopped.as_deref())
                        .or({
                            if !retained_evidence.is_empty() {
                                Some("Partial")
                            } else {
                                transfer_status.as_deref()
                            }
                        })
                        .ok_or(E::CorruptJournal)?;
                    if status != expected {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_status = Some(status.to_owned());
                    report.manifest_finished = true;
                }
                _ => return Err(E::CorruptJournal),
            }
        }
        if !header {
            return Err(E::CorruptJournal);
        }
        if let Some((index, _)) = pending_metadata {
            report.pending_metadata.push(reference(&destination_path(
                &self.snapshot.copied_directories[index].destination,
            )?));
        }
        if let Some((reference, ..)) = pending_create {
            report.pending_directories.push(reference);
        }
        if let Some(directory) = pending_cleanup {
            report.pending_cleanup.push(directory.reference);
        }
        Ok((report, observed))
    }
}
