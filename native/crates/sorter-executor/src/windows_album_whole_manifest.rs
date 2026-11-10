//! Capability-free replay context extracted from the existing whole protocol.
use super::*;

pub(super) struct WholeManifestEvidence<'a> {
    pub(super) header: &'a Header,
    pub(super) job_id: &'a str,
}
impl WholeManifestEvidence<'_> {
    pub(super) fn read_manifest(
        &self,
        bytes: &[u8],
        cancel: &AtomicBool,
    ) -> Result<WholeAlbumRecovery> {
        self.read_manifest_inner(bytes, cancel, None)
    }
    pub(super) fn read_manifest_controlled(
        &self,
        bytes: &[u8],
        control: &InspectionControl,
    ) -> Result<WholeAlbumRecovery> {
        crate::inspection_control::check_recorded_input(bytes, Some(control))?;
        self.read_manifest_inner(bytes, control.cancellation_flag(), Some(control))
    }
    fn read_manifest_inner(
        &self,
        bytes: &[u8],
        cancel: &AtomicBool,
        control: Option<&InspectionControl>,
    ) -> Result<WholeAlbumRecovery> {
        if bytes.len() > MAX_MANIFEST {
            return Err(E::Unsupported);
        }
        let mut report = WholeAlbumRecovery {
            recorded_singles: None,
            created_directories: vec![],
            child_renames: vec![],
            child_locations: vec![],
            recorded_finished: false,
            incomplete_tail: false,
            recorded: vec![],
            source_evidence: vec![],
            destination_evidence: vec![],
            files: vec![],
            pending_directories: vec![],
            pending_albums: vec![],
            singles: None,
        };
        let mut previous = String::new();
        let mut sequence = 0u64;
        let mut offset = 0;
        let mut header = false;
        let mut pending_directory: Option<(StorageRef, ObjectIdentity, String)> = None;
        let mut created = Vec::<MutationObservation>::new();
        let mut pending_album: Option<(usize, ObjectIdentity)> = None;
        let mut stopped = false;
        let mut singles_prepared = false;
        let mut singles_status = None::<String>;
        let mut parent_stopped = None::<String>;
        let mut pending_child = None::<(usize, usize)>;
        let mut child_stopped = None::<String>;
        while offset < bytes.len() {
            if let Some(control) = control {
                control.record()?;
            }
            cancelled(cancel)?;
            let Some(end) = bytes[offset..].iter().position(|byte| *byte == b'\n') else {
                report.incomplete_tail = true;
                break;
            };
            let envelope: ManifestEnvelope = serde_json::from_slice(&bytes[offset..offset + end])
                .map_err(|_| E::CorruptJournal)?;
            offset += end + 1;
            if report.recorded_finished
                || envelope.body.sequence != sequence
                || envelope.body.previous != previous
                || envelope.body.job_id != self.job_id
                || crate::hash(&envelope.body)? != envelope.sha256
            {
                return Err(E::CorruptJournal);
            }
            sequence += 1;
            previous = envelope.sha256;
            let record = envelope.body;
            if (parent_stopped.is_some() || child_stopped.is_some()) && record.phase != "Finished" {
                return Err(E::CorruptJournal);
            }
            match record.phase.as_str() {
                "Header" => {
                    if header
                        || sequence != 1
                        || record.payload != serde_json::to_value(self.header).map_err(|_| E::Io)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    header = true;
                }
                "DirectoryPrepared" => {
                    if !header || stopped || pending_directory.is_some() || pending_album.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let path: StorageRef =
                        serde_json::from_value(record.payload["reference"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let parent: ObjectIdentity =
                        serde_json::from_value(record.payload["parent"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let leaf = record.payload["name"]
                        .as_str()
                        .ok_or(E::CorruptJournal)?
                        .to_owned();
                    let requirement = self
                        .header
                        .snapshot
                        .destinations
                        .iter()
                        .find(|d| {
                            destination_path(&d.destination).is_ok_and(|p| reference(&p) == path)
                        })
                        .ok_or(E::CorruptJournal)?;
                    if requirement.existing.is_some()
                        || requirement.destination.components.last() != Some(&leaf)
                        || created.iter().any(|o| o.reference == path)
                        || !self.header.snapshot.albums.iter().any(|a| {
                            a.destination
                                .components
                                .starts_with(&requirement.destination.components)
                                && a.destination.components.len()
                                    > requirement.destination.components.len()
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    if self.expected_parent(&path, &created)? != parent {
                        return Err(E::CorruptJournal);
                    }
                    pending_directory = Some((path, parent, leaf));
                }
                "DirectoryObserved" => {
                    let observation: MutationObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let (path, parent, leaf) = pending_directory.take().ok_or(E::CorruptJournal)?;
                    if observation.reference != path
                        || observation.parent_identity != parent
                        || observation.actual_name != name(&leaf)
                        || !new_directory_identity(
                            &self.header.snapshot,
                            &self.header.scope.source_root,
                            &self.header.scope.target_root,
                            &observation.identity,
                            &created,
                        )
                        || observation.size.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    created.push(observation);
                }
                "ChildRenamePrepared" => {
                    if !header
                        || stopped
                        || singles_prepared
                        || pending_directory.is_some()
                        || pending_album.is_some()
                        || pending_child.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let index = report.recorded.len();
                    let album = self
                        .header
                        .snapshot
                        .albums
                        .get(index)
                        .ok_or(E::CorruptJournal)?;
                    let child_index = self
                        .next_recorded_child(album, &report.child_renames)
                        .ok_or(E::CorruptJournal)?;
                    let child = &album.children[child_index];
                    if record.payload
                        != serde_json::json!({"operation":album.operation,"planner_index":child_index,"source":leaf(&child.source)?,"destination":name(&child.destination_name),"identity":child.source.identity})
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_child = Some((index, child_index));
                }
                "ChildRenameObserved" => {
                    let (index, child_index) = pending_child.take().ok_or(E::CorruptJournal)?;
                    let album = &self.header.snapshot.albums[index];
                    let receipt: AlbumChildRenameReceipt =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if receipt.operation != album.operation || receipt.planner_index != child_index
                    {
                        return Err(E::CorruptJournal);
                    }
                    self.validate_child_receipt(index, &receipt)?;
                    report.child_renames.push(receipt);
                }
                "ChildRenameStopped" | "ChildRenameUnknown" => {
                    let (index, child_index) = pending_child.take().ok_or(E::CorruptJournal)?;
                    let album = &self.header.snapshot.albums[index];
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if record.payload["operation"].as_u64() != Some(album.operation as u64)
                        || record.payload["planner_index"].as_u64() != Some(child_index as u64)
                        || !(if record.phase == "ChildRenameUnknown" {
                            status == "NeedsRecovery"
                        } else {
                            matches!(status, "Partial" | "Cancelled")
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    let _: AlbumFailure = serde_json::from_value(record.payload["failure"].clone())
                        .map_err(|_| E::CorruptJournal)?;
                    child_stopped = Some(status.to_owned());
                    stopped = true;
                    if status == "NeedsRecovery" {
                        report.pending_albums.push(album.operation);
                    }
                }
                "AlbumPrepared" => {
                    if !header
                        || stopped
                        || pending_directory.is_some()
                        || pending_album.is_some()
                        || pending_child.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let index = report.recorded.len();
                    let album = self
                        .header
                        .snapshot
                        .albums
                        .get(index)
                        .ok_or(E::CorruptJournal)?;
                    if self
                        .next_recorded_child(album, &report.child_renames)
                        .is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let parent = self.expected_parent(
                        &reference(&destination_path(&album.destination)?),
                        &created,
                    )?;
                    if record.payload
                        != serde_json::json!({"operation":album.operation,"source":album.source,"destination":album.destination,"parent":parent})
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_album = Some((index, parent));
                }
                "AlbumObserved" | "AlbumStopped" | "AlbumUnknown" => {
                    let (index, parent) = pending_album.take().ok_or(E::CorruptJournal)?;
                    let album = &self.header.snapshot.albums[index];
                    let outcome: WholeAlbumOutcome =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if outcome.operation != album.operation
                        || outcome.child_renames
                            != report
                                .child_renames
                                .iter()
                                .filter(|receipt| receipt.operation == album.operation)
                                .cloned()
                                .collect::<Vec<_>>()
                    {
                        return Err(E::CorruptJournal);
                    }
                    if record.phase == "AlbumObserved" {
                        self.validate_outcome(album, &parent, &outcome)?;
                    } else {
                        if !outcome.descendants.is_empty()
                            || outcome.failure.is_none()
                            || !(if record.phase == "AlbumUnknown" {
                                outcome.status == "needs_recovery"
                            } else {
                                matches!(outcome.status.as_str(), "failed" | "cancelled")
                            })
                        {
                            return Err(E::CorruptJournal);
                        }
                        if let Some(root) = &outcome.root {
                            let original = &self.header.observed_sources[index];
                            let destination = destination_path(&Destination {
                                root: self.header.observed_target.clone(),
                                components: album.destination.components.clone(),
                            })?;
                            let at_original = root.reference == original.reference
                                && root.parent_identity == original.parent_identity
                                && root.actual_name == original.actual_name;
                            let at_destination = root.reference == reference(&destination)
                                && root.parent_identity == parent
                                && root.actual_name
                                    == name(
                                        album
                                            .destination
                                            .components
                                            .last()
                                            .ok_or(E::CorruptJournal)?,
                                    );
                            if root.identity != original.identity
                                || root.size.is_some()
                                || !(at_original || at_destination)
                            {
                                return Err(E::CorruptJournal);
                            }
                        }
                        stopped = true;
                    }
                    report.recorded.push(outcome);
                }
                "SinglesPrepared" => {
                    if !header
                        || stopped
                        || pending_directory.is_some()
                        || pending_album.is_some()
                        || singles_prepared
                        || report.recorded.len() != self.header.snapshot.albums.len()
                        || report.recorded.iter().any(|o| o.status != "complete")
                    {
                        return Err(E::CorruptJournal);
                    }
                    let binding = self
                        .header
                        .singles_binding
                        .as_ref()
                        .ok_or(E::CorruptJournal)?;
                    if record.payload
                        != serde_json::json!({"job_id":self.header.singles_job_id,"binding_sha256":crate::hash(binding)?})
                    {
                        return Err(E::CorruptJournal);
                    }
                    singles_prepared = true;
                }
                "SinglesObserved" | "SinglesStopped" => {
                    if !singles_prepared
                        || singles_status.is_some()
                        || record.payload["job_id"]
                            != serde_json::to_value(&self.header.singles_job_id)
                                .map_err(|_| E::Io)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(
                        status,
                        "Complete" | "Partial" | "Cancelled" | "NeedsRecovery"
                    ) || (record.phase == "SinglesStopped" && status == "Complete")
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_singles =
                        Some((status.to_owned(), record.phase == "SinglesObserved"));
                    singles_status = Some(status.to_owned());
                }
                "ParentStopped" => {
                    if !header
                        || stopped
                        || pending_album.is_some()
                        || pending_child.is_some()
                        || singles_prepared
                    {
                        return Err(E::CorruptJournal);
                    }
                    let album = self
                        .header
                        .snapshot
                        .albums
                        .get(report.recorded.len())
                        .ok_or(E::CorruptJournal)?;
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if record.payload["operation"].as_u64() != Some(album.operation as u64)
                        || !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery")
                    {
                        return Err(E::CorruptJournal);
                    }
                    parent_stopped = Some(status.to_owned());
                }
                "Finished" => {
                    if !header
                        || record.payload["child_renames"]
                            != serde_json::to_value(&report.child_renames).map_err(|_| E::Io)?
                        || record.payload["outcomes"]
                            != serde_json::to_value(&report.recorded).map_err(|_| E::Io)?
                        || record.payload["created"]
                            != serde_json::to_value(&created).map_err(|_| E::Io)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    let completed = report
                        .recorded
                        .iter()
                        .filter(|o| o.status == "complete")
                        .count();
                    if record.payload["albums_completed"].as_u64() != Some(completed as u64)
                        || !matches!(
                            record.payload["status"].as_str(),
                            Some("Complete" | "Partial" | "Cancelled" | "NeedsRecovery")
                        )
                    {
                        return Err(E::CorruptJournal);
                    }
                    if record.payload["status"] == "Complete"
                        && (completed != self.header.snapshot.albums.len()
                            || pending_directory.is_some()
                            || pending_album.is_some()
                            || stopped
                            || (self.header.singles_binding.is_some()
                                && singles_status.as_deref() != Some("Complete")))
                    {
                        return Err(E::CorruptJournal);
                    }
                    if singles_status
                        .as_deref()
                        .is_some_and(|status| record.payload["status"] != status)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if report.recorded.iter().any(|o| o.status == "needs_recovery")
                        && record.payload["status"] != "NeedsRecovery"
                    {
                        return Err(E::CorruptJournal);
                    }
                    if report.recorded.iter().any(|o| o.status == "cancelled")
                        && record.payload["status"] != "Cancelled"
                    {
                        return Err(E::CorruptJournal);
                    }
                    if report.recorded.iter().any(|o| o.status == "failed")
                        && record.payload["status"] != "Partial"
                    {
                        return Err(E::CorruptJournal);
                    }
                    let expected = parent_stopped
                        .as_deref()
                        .or(child_stopped.as_deref())
                        .or(singles_status.as_deref())
                        .or_else(|| {
                            report
                                .recorded
                                .last()
                                .and_then(|o| match o.status.as_str() {
                                    "needs_recovery" => Some("NeedsRecovery"),
                                    "cancelled" => Some("Cancelled"),
                                    "failed" => Some("Partial"),
                                    _ => None,
                                })
                        })
                        .unwrap_or("Complete");
                    if record.payload["status"] != expected {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_finished = true;
                }
                _ => return Err(E::CorruptJournal),
            }
        }
        if !header {
            return Err(E::CorruptJournal);
        }
        if let Some((path, ..)) = pending_directory {
            report.pending_directories.push(path);
        }
        if let Some((index, ..)) = pending_album {
            report
                .pending_albums
                .push(self.header.snapshot.albums[index].operation);
        }
        if let Some((index, ..)) = pending_child {
            report
                .pending_albums
                .push(self.header.snapshot.albums[index].operation);
        }
        report.created_directories = created;
        cancelled(cancel)?;
        Ok(report)
    }
    fn next_recorded_child(
        &self,
        album: &AlbumGroup,
        receipts: &[AlbumChildRenameReceipt],
    ) -> Option<usize> {
        album
            .children
            .iter()
            .enumerate()
            .find(|(index, child)| {
                leaf(&child.source).is_ok_and(|source| source != name(&child.destination_name))
                    && !receipts.iter().any(|receipt| {
                        receipt.operation == album.operation && receipt.planner_index == *index
                    })
            })
            .map(|(index, _)| index)
    }
    fn validate_child_receipt(
        &self,
        index: usize,
        receipt: &AlbumChildRenameReceipt,
    ) -> Result<()> {
        let album = &self.header.snapshot.albums[index];
        let child = album
            .children
            .get(receipt.planner_index)
            .ok_or(E::CorruptJournal)?;
        let root = &self.header.observed_sources[index];
        let mut expected = path(&root.reference)?;
        expected.push(&child.destination_name);
        let observed = &receipt.observation;
        if observed.reference != reference(&expected)
            || observed.parent_identity != root.identity
            || Some(&observed.identity) != child.source.identity.as_ref()
            || observed.actual_name != name(&child.destination_name)
            || observed.size != child.source.size
        {
            return Err(E::CorruptJournal);
        }
        Ok(())
    }
    fn expected_parent(
        &self,
        child: &StorageRef,
        created: &[MutationObservation],
    ) -> Result<ObjectIdentity> {
        let child = path(child)?;
        let parent = reference(child.parent().ok_or(E::CorruptJournal)?);
        if parent == self.header.target.reference {
            return self.header.target.identity.clone().ok_or(E::CorruptJournal);
        }
        if let Some(observation) = created
            .iter()
            .find(|observation| observation.reference == parent)
        {
            return Ok(observation.identity.clone());
        }
        self.header
            .snapshot
            .destinations
            .iter()
            .find_map(|requirement| {
                requirement
                    .existing
                    .as_ref()
                    .filter(|existing| existing.reference == parent)
                    .and_then(|existing| existing.identity.clone())
            })
            .ok_or(E::CorruptJournal)
    }
    fn validate_outcome(
        &self,
        album: &AlbumGroup,
        parent: &ObjectIdentity,
        outcome: &WholeAlbumOutcome,
    ) -> Result<()> {
        let root = outcome.root.as_ref().ok_or(E::CorruptJournal)?;
        if outcome.status != "complete"
            || root.size.is_some()
            || outcome.failure.is_some()
            || Some(&root.identity) != album.source.identity.as_ref()
            || &root.parent_identity != parent
            || root.actual_name
                != name(
                    album
                        .destination
                        .components
                        .last()
                        .ok_or(E::CorruptJournal)?,
                )
        {
            return Err(E::CorruptJournal);
        }
        let root_path = path(&root.reference)?;
        let expected_root = destination_path(&Destination {
            root: self.header.observed_target.clone(),
            components: album.destination.components.clone(),
        })?;
        if root_path != expected_root {
            return Err(E::CorruptJournal);
        }
        let source_path = path(&album.source.reference)?;
        let mut expected = Vec::new();
        for directory in &self.header.snapshot.directories {
            if path(&directory.source.reference)?.starts_with(&source_path) {
                expected.extend(directory.children.iter());
            }
        }
        if expected.len() != outcome.descendants.len() {
            return Err(E::CorruptJournal);
        }
        let mut seen = HashSet::new();
        for (relative, observation) in &outcome.descendants {
            let first = relative.first().ok_or(E::CorruptJournal)?;
            let mut source = source_path.clone();
            let mut destination = root_path.clone();
            for (part_index, part) in relative.iter().enumerate() {
                let MutationName::Windows(units) = part else {
                    return Err(E::CorruptJournal);
                };
                if units.is_empty()
                    || units.iter().any(|u| matches!(*u, 0 | 47 | 92 | 58))
                    || units == &[46]
                    || units == &[46, 46]
                {
                    return Err(E::CorruptJournal);
                }
                let part = OsString::from_wide(units);
                if part_index == 0 {
                    let child = album
                        .children
                        .iter()
                        .find(|child| name(&child.destination_name) == *first)
                        .ok_or(E::CorruptJournal)?;
                    source.push(
                        path(&child.source.reference)?
                            .file_name()
                            .ok_or(E::CorruptJournal)?,
                    );
                } else {
                    source.push(&part);
                }
                destination.push(part);
            }
            let source = reference(&source);
            let entry = expected
                .iter()
                .find(|entry| entry.reference == source)
                .ok_or(E::CorruptJournal)?;
            if !seen.insert(source)
                || observation.reference != reference(&destination)
                || Some(&observation.identity) != entry.identity.as_ref()
                || Some(&observation.parent_identity) != entry.parent_identity.as_ref()
                || Some(&observation.actual_name) != relative.last()
                || observation.size != entry.size
            {
                return Err(E::CorruptJournal);
            }
        }
        Ok(())
    }
}
