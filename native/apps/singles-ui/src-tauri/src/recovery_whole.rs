// Whole-Album restart is read-only. Descendant rows are inspections of one
// recorded directory move, never synthesized file-transfer receipts.
const WHOLE_VERIFICATION_SCOPE: &str = "recorded_main_hash_identity_membership";

fn inspect_whole_record(
    record: RetainedRecord,
    source: SelectedSource,
    target: Root,
    control: Option<&InspectionControl>,
    cursor: usize,
) -> Result<RecoveryReport> {
    project_whole_inspection(read_whole_inspection(
        record, source, target, control, cursor,
    )?)
}

fn read_whole_inspection(
    record: RetainedRecord,
    source: SelectedSource,
    target: Root,
    control: Option<&InspectionControl>,
    cursor: usize,
) -> Result<sorter_executor::windows_album::RecordedWholeAlbumInspection> {
    use sorter_executor::windows_album::{
        AlbumFileInspectionGrants, AlbumFileInspectionJournal, AlbumFileInspectionSource,
        recover_recorded_whole_album_job,
    };
    use sorter_io::desktop::windows_mutation::WindowsAlbumInspectionParentGrant;
    use sorter_io::mutation::MutationName;
    let fallback = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
    let control = control.unwrap_or(&fallback);
    let RetainedJournal::Whole {
        manifest,
        nested,
        descriptor,
    } = &record.journal
    else {
        return Err(changed());
    };
    let mut storage = probe(Some(control), || {
        Desktop::new(std::slice::from_ref(&record.directory)).map_err(storage_error)
    })?;
    let reference = storage
        .reference(&record.directory)
        .map_err(storage_error)?;
    let observed = probe(Some(control), || {
        storage.stat(&reference).map_err(storage_error)
    })?;
    if observed.identity != record.root.identity || observed.kind != EntryKind::Directory {
        return Err(changed());
    }
    let mut budget = PAGE_BYTE_BUDGET;
    read_retained(&mut storage, manifest, &mut budget, Some(control))?;
    for (path, evidence) in nested {
        if let Some(evidence) = evidence {
            read_retained(&mut storage, evidence, &mut budget, Some(control))?;
        } else if observation(&mut storage, path, Some(control))?.is_some() {
            return Err(changed());
        }
    }
    drop(storage);
    let journal = |evidence: &JournalEvidence| -> Result<AlbumFileInspectionJournal> {
        Ok(AlbumFileInspectionJournal {
            name: MutationName::Windows(evidence.observed.name.encode_utf16().collect()),
            identity: evidence.observed.identity.clone().ok_or_else(changed)?,
            length: evidence.observed.size.ok_or_else(changed)?,
            sha256: evidence.sha256.clone(),
        })
    };
    let mut journals = vec![journal(manifest)?];
    for (_, evidence) in nested {
        if let Some(evidence) = evidence {
            journals.push(journal(evidence)?);
        }
    }
    let grant = |root: Root| {
        WindowsSelectedRootGrant::new(root.path, root.observed.identity.ok_or_else(changed)?)
            .map_err(storage_error)
    };
    let source = match source {
        SelectedSource::Present(root) => AlbumFileInspectionSource::Present(grant(root)?),
        SelectedSource::Missing(parent) => AlbumFileInspectionSource::Missing(
            WindowsAlbumInspectionParentGrant::new(
                parent.path,
                parent.observed.identity.ok_or_else(changed)?,
            )
            .map_err(storage_error)?,
        ),
    };
    let grants = AlbumFileInspectionGrants {
        source,
        target: grant(target)?,
        journal: WindowsSelectedRootGrant::new(
            record.directory,
            record.root.identity.ok_or_else(changed)?,
        )
        .map_err(storage_error)?,
    };
    recover_recorded_whole_album_job(descriptor, grants, journals, control, cursor)
        .map_err(recorded_error)
}

fn whole_destination_reference(
    destination: &sorter_core::job_plan::Destination,
    relative: &[sorter_io::mutation::MutationName],
) -> Result<StorageRef> {
    use sorter_io::mutation::MutationName;
    let StorageRef::WindowsPath { utf16 } = &destination.root else {
        return Err(changed());
    };
    let mut units = utf16.clone();
    let parts = destination
        .components
        .iter()
        .map(|part| part.encode_utf16().collect::<Vec<_>>())
        .chain(relative.iter().map(|part| match part {
            MutationName::Windows(value) => value.clone(),
            _ => Vec::new(),
        }));
    for part in parts {
        if part.is_empty()
            || part == [46]
            || part == [46, 46]
            || part.iter().any(|unit| matches!(*unit, 0 | 47 | 58 | 92))
        {
            return Err(changed());
        }
        if !units.last().is_some_and(|unit| matches!(*unit, 47 | 92)) {
            units.push(92);
        }
        units.extend(part);
    }
    Ok(StorageRef::WindowsPath { utf16: units })
}

fn whole_directory_status(
    item: Option<&sorter_executor::windows_album::AlbumDirectoryEvidence>,
    identity: &sorter_core::discovery::ObjectIdentity,
    absent_expected: bool,
) -> &'static str {
    use sorter_executor::windows_album::AlbumDirectoryEvidenceStatus as Status;
    let Some(item) = item else {
        return "Unverified";
    };
    if &item.expected_identity != identity {
        return "Changed";
    }
    match item.status {
        Status::Absent if item.observed_identity.is_none() && absent_expected => "AbsentExpected",
        Status::Absent if item.observed_identity.is_none() => "Missing",
        Status::SameIdentity
            if !absent_expected && item.observed_identity.as_ref() == Some(identity) =>
        {
            "Verified"
        }
        Status::Unverified => "Unverified",
        _ => "Changed",
    }
}

fn project_whole_inspection(
    inspected: sorter_executor::windows_album::RecordedWholeAlbumInspection,
) -> Result<RecoveryReport> {
    use sorter_core::discovery::ObjectIdentity;
    use sorter_executor::recovery::{CopyMetadataRecoveryStatus, RecoveryStatus};
    use sorter_executor::windows_album::{AlbumDirectoryEvidence, AlbumInspectionSourceState};
    use std::collections::{HashMap, HashSet};
    let recorded = &inspected.recorded;
    let report = &inspected.recovery;
    if inspected.content_scope != WHOLE_VERIFICATION_SCOPE
        || recorded.files.is_empty()
        || inspected.files.len() != recorded.files.len()
    {
        return Err(changed());
    }
    let albums = recorded
        .albums
        .iter()
        .map(|album| (album.operation, album))
        .collect::<HashMap<_, _>>();
    if albums.is_empty() || albums.len() != recorded.albums.len() {
        return Err(changed());
    }
    let mut outcomes = HashMap::new();
    for outcome in &report.recorded {
        if !albums.contains_key(&outcome.operation)
            || outcomes.insert(outcome.operation, outcome).is_some()
        {
            return Err(changed());
        }
    }
    let mut evidence = Vec::new();
    let mut completed = HashSet::new();
    let mut published_files = HashMap::new();
    for album in &recorded.albums {
        let destination = whole_destination_reference(&album.destination, &[])?;
        let valid = outcomes.get(&album.operation).is_some_and(|outcome| {
            outcome.status == "complete"
                && outcome.failure.is_none()
                && outcome.root.as_ref().is_some_and(|root| {
                    root.size.is_none() && Some(&root.identity) == album.source.identity.as_ref()
                })
        });
        evidence.push(RecoveryEvidence {
            role: "whole_album".into(),
            path: display(&destination),
            status: if valid { "Verified" } else { "Unverified" }.into(),
        });
        if valid {
            completed.insert(album.operation);
            for (relative, observed) in &outcomes[&album.operation].descendants {
                if observed.size.is_some() {
                    let reference = whole_destination_reference(&album.destination, relative)?;
                    if published_files
                        .insert(reference, observed.identity.clone())
                        .is_some()
                    {
                        return Err(changed());
                    }
                }
            }
        }
    }
    let expected_published = recorded
        .files
        .iter()
        .filter(|file| completed.contains(&file.operation))
        .map(|file| file.destination.clone())
        .collect::<HashSet<_>>();
    if published_files.keys().cloned().collect::<HashSet<_>>() != expected_published {
        return Err(changed());
    }
    let expected_renames = recorded
        .child_locations
        .iter()
        .map(|child| (child.operation, child.planner_index))
        .collect::<HashSet<_>>();
    if expected_renames.len() != recorded.child_locations.len() {
        return Err(changed());
    }
    let mut rename_receipts = HashMap::new();
    for receipt in &report.child_renames {
        let key = (receipt.operation, receipt.planner_index);
        if !expected_renames.contains(&key) || rename_receipts.insert(key, receipt).is_some() {
            return Err(changed());
        }
    }
    for outcome in &report.recorded {
        let mut mirrored = HashMap::new();
        for receipt in &outcome.child_renames {
            let key = (receipt.operation, receipt.planner_index);
            if receipt.operation != outcome.operation
                || !expected_renames.contains(&key)
                || mirrored.insert(key, receipt).is_some()
                || rename_receipts.get(&key) != Some(&receipt)
            {
                return Err(changed());
            }
        }
        if completed.contains(&outcome.operation) {
            let expected = expected_renames
                .iter()
                .filter(|(operation, _)| *operation == outcome.operation)
                .copied()
                .collect::<HashSet<_>>();
            if mirrored.keys().copied().collect::<HashSet<_>>() != expected {
                return Err(changed());
            }
        }
    }
    let source_identity = recorded.source.identity.as_ref().ok_or_else(changed)?;
    let root_moved = recorded.albums.iter().any(|album| {
        album.source.reference == recorded.source.reference && completed.contains(&album.operation)
    });
    let source_status = match &inspected.source_state {
        AlbumInspectionSourceState::Present { observed } => {
            if root_moved || &observed.identity != source_identity {
                "Changed"
            } else {
                "Verified"
            }
        }
        AlbumInspectionSourceState::Absent { recorded: original } => {
            if root_moved
                && &original.identity == source_identity
                && recorded.original_source.as_ref() == Some(original)
            {
                "AbsentExpected"
            } else {
                "Missing"
            }
        }
    };
    evidence.push(RecoveryEvidence {
        role: "source_root".into(),
        path: display(&recorded.source.reference),
        status: source_status.into(),
    });
    evidence.push(RecoveryEvidence {
        role: "target_root".into(),
        path: display(&recorded.target.reference),
        status: "Verified".into(),
    });
    // Source and destination root observations are distinct from journal claims.
    for (role, rows, source) in [
        ("album_source", &report.source_evidence, true),
        ("album_destination", &report.destination_evidence, false),
    ] {
        let mut indexed = HashMap::new();
        for row in rows {
            if indexed.insert(&row.reference, row).is_some() {
                return Err(changed());
            }
        }
        for album in &recorded.albums {
            let reference = if source {
                album.source.reference.clone()
            } else {
                whole_destination_reference(&album.destination, &[])?
            };
            let identity = album.source.identity.as_ref().ok_or_else(changed)?;
            let status = whole_directory_status(
                indexed.remove(&reference),
                identity,
                source && completed.contains(&album.operation),
            );
            evidence.push(RecoveryEvidence {
                role: role.into(),
                path: display(&reference),
                status: status.into(),
            });
        }
        if !indexed.is_empty() {
            return Err(changed());
        }
    }
    // Exact descriptor directory references bound by the recorded parser decide
    // which source absences are expected. No arbitrary Absent row is accepted.
    let mut expected = HashMap::<StorageRef, (ObjectIdentity, bool)>::new();
    let mut membership_expected = HashSet::new();
    let mut membership_allowed = HashSet::new();
    for directory in &recorded.directories {
        if !albums.contains_key(&directory.operation) {
            return Err(changed());
        }
        let absent = completed.contains(&directory.operation);
        for (reference, is_source) in [
            (&directory.source, true),
            (&directory.prepared_source, true),
            (&directory.destination, false),
        ] {
            membership_allowed.insert(reference.clone());
            if is_source && reference == &recorded.source.reference {
                continue;
            }
            let value = (directory.identity.clone(), is_source && absent);
            if let Some(prior) = expected.insert(reference.clone(), value.clone())
                && prior != value
            {
                return Err(changed());
            }
        }
        if absent {
            membership_expected.insert(directory.destination.clone());
        }
    }
    let mut current_directories = HashMap::<StorageRef, &AlbumDirectoryEvidence>::new();
    for row in &inspected.directories {
        if current_directories
            .insert(row.reference.clone(), row)
            .is_some()
        {
            return Err(changed());
        }
    }
    let mut verified_directory_ids = HashSet::new();
    for (reference, (identity, absent)) in expected {
        let status =
            whole_directory_status(current_directories.remove(&reference), &identity, absent);
        if status == "Verified" {
            verified_directory_ids.insert(identity);
        }
        evidence.push(RecoveryEvidence {
            role: "directory".into(),
            path: display(&reference),
            status: status.into(),
        });
    }
    // Additional target ancestors remain verified through the fixed native ledger.
    // They may never use the source-absence exception.
    for (reference, row) in current_directories {
        let status = whole_directory_status(Some(row), &row.expected_identity, false);
        if status == "Verified" {
            verified_directory_ids.insert(row.expected_identity.clone());
        }
        evidence.push(RecoveryEvidence {
            role: "directory".into(),
            path: display(&reference),
            status: status.into(),
        });
    }
    for created in &report.created_directories {
        if !verified_directory_ids.contains(&created.identity) {
            evidence.push(RecoveryEvidence {
                role: "directory".into(),
                path: display(&created.reference),
                status: "Unverified".into(),
            });
        }
    }
    let mut membership = HashMap::new();
    for row in &inspected.membership {
        if !membership_allowed.contains(&row.reference)
            || membership.insert(row.reference.clone(), row).is_some()
        {
            return Err(changed());
        }
    }
    for reference in membership_expected {
        let status = match membership.remove(&reference).map(|row| row.status.as_str()) {
            Some("verified") => "Verified",
            Some("changed") => "Changed",
            _ => "Unverified",
        };
        evidence.push(RecoveryEvidence {
            role: "membership".into(),
            path: display(&reference),
            status: status.into(),
        });
    }
    for (reference, row) in membership {
        let status = match row.status.as_str() {
            "verified" => "Verified",
            "changed" => "Changed",
            _ => "Unverified",
        };
        evidence.push(RecoveryEvidence {
            role: "membership".into(),
            path: display(&reference),
            status: status.into(),
        });
    }
    let mut child_locations = HashMap::new();
    for row in &report.child_locations {
        if child_locations
            .insert((row.operation, row.planner_index, row.at_destination), row)
            .is_some()
        {
            return Err(changed());
        }
    }
    let mut child_keys = HashSet::new();
    for child in &recorded.child_locations {
        if !albums.contains_key(&child.operation) {
            return Err(changed());
        }
        for (at_destination, reference) in [(false, &child.original), (true, &child.prepared)] {
            let key = (child.operation, child.planner_index, at_destination);
            if !child_keys.insert(key) {
                return Err(changed());
            }
            let status = match child_locations.remove(&key) {
                Some(row)
                    if row.error.is_none()
                        && row.observed.is_none()
                        && completed.contains(&child.operation) =>
                {
                    "AbsentExpected"
                }
                Some(row) if row.error.is_none() && row.observed.is_some() => "Changed",
                _ => "Unverified",
            };
            evidence.push(RecoveryEvidence {
                role: "child_location".into(),
                path: display(reference),
                status: status.into(),
            });
        }
    }
    if !child_locations.is_empty() {
        return Err(changed());
    }
    let mut operations = Vec::with_capacity(recorded.files.len());
    let mut deferred = HashMap::new();
    let mut main_evidence = HashMap::new();
    for item in &report.files {
        if main_evidence
            .insert((item.operation, &item.reference), item)
            .is_some()
        {
            return Err(changed());
        }
    }
    for (index, item) in inspected.files.iter().enumerate() {
        if item.file != recorded.files[index] || item.file.inspection_index != index {
            return Err(changed());
        }
        let mut details = Vec::new();
        let status = if albums.contains_key(&item.file.operation) {
            if item.transfer.is_some() {
                return Err(changed());
            }
            match &item.main {
                Some(main) => {
                    if main.operation != item.file.operation
                        || main.reference != item.file.destination
                    {
                        return Err(changed());
                    }
                    let mirrored = main_evidence
                        .remove(&(main.operation, &main.reference))
                        .ok_or_else(changed)?;
                    if mirrored.expected_identity != main.expected_identity
                        || mirrored.observed_identity != main.observed_identity
                        || mirrored.status != main.status
                    {
                        return Err(changed());
                    }
                    if main.status == "verified"
                        && main.observed_identity.as_ref() == Some(&main.expected_identity)
                        && published_files.get(&main.reference) == Some(&main.expected_identity)
                    {
                        "Verified"
                    } else if main.status == "changed" {
                        "Changed"
                    } else {
                        "Unverified"
                    }
                }
                None => "Unverified",
            }
        } else {
            if item.main.is_some() {
                return Err(changed());
            }
            match &item.transfer {
                Some(transfer) => {
                    if transfer.operation != item.file.operation
                        || deferred.insert(transfer.operation, transfer).is_some()
                    {
                        return Err(changed());
                    }
                    if transfer.copy_metadata != CopyMetadataRecoveryStatus::NotRequested {
                        details.push(RecoveryEvidence {
                            role: "file_metadata".into(),
                            path: display(&item.file.destination),
                            status: match transfer.copy_metadata {
                                CopyMetadataRecoveryStatus::Verified => "Verified",
                                CopyMetadataRecoveryStatus::Mismatch => "Changed",
                                _ => "Unverified",
                            }
                            .into(),
                        });
                    }
                    if transfer.status == RecoveryStatus::Committed
                        && matches!(
                            transfer.copy_metadata,
                            CopyMetadataRecoveryStatus::Verified
                                | CopyMetadataRecoveryStatus::NotRequested
                        )
                    {
                        "Verified"
                    } else {
                        "Unverified"
                    }
                }
                None => "Unverified",
            }
        };
        operations.push(RecoveryOperation {
            operation: index,
            source: display(&item.file.source),
            destination: display(&item.file.destination),
            status: status.into(),
            evidence: details,
        });
    }
    if !main_evidence.is_empty() {
        return Err(changed());
    }
    let expected_deferred = recorded
        .files
        .iter()
        .filter(|file| !albums.contains_key(&file.operation))
        .map(|file| file.operation)
        .collect::<HashSet<_>>();
    let mut pending = report
        .pending_directories
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let nested_complete = match (&report.singles, expected_deferred.is_empty()) {
        (None, true) => recorded.singles_job_id.is_none(),
        (Some(nested), false) => {
            pending.extend(
                nested
                    .pending_directories
                    .iter()
                    .chain(&nested.pending_cleanup)
                    .chain(&nested.pending_metadata)
                    .cloned(),
            );
            for row in &nested.directories {
                evidence.push(RecoveryEvidence {
                    role: "directory".into(),
                    path: display(&row.reference),
                    status: whole_directory_status(Some(row), &row.expected_identity, false).into(),
                });
            }
            let mut metadata = HashMap::new();
            for (reference, status) in &nested.directory_metadata_status {
                if metadata.insert(reference, status).is_some() {
                    return Err(changed());
                }
            }
            for receipt in &nested.directory_metadata {
                let status = match metadata.remove(&receipt.destination).map(String::as_str) {
                    Some("verified") => "Verified",
                    Some("changed" | "mismatch") => "Changed",
                    _ => "Unverified",
                };
                evidence.push(RecoveryEvidence {
                    role: "directory_metadata".into(),
                    path: display(&receipt.destination),
                    status: status.into(),
                });
            }
            if !metadata.is_empty() {
                return Err(changed());
            }
            let physical_complete = nested.transfers.as_ref().is_some_and(|transfers| {
                let ids = transfers
                    .operations
                    .iter()
                    .map(|item| item.operation)
                    .collect::<HashSet<_>>();
                !transfers.append_blocked
                    && ids == expected_deferred
                    && ids.len() == transfers.operations.len()
                    && ids.len() == deferred.len()
                    && transfers.operations.iter().all(|item| {
                        deferred
                            .get(&item.operation)
                            .is_some_and(|actual| *actual == item)
                    })
            });
            nested.manifest_finished
                && nested.recorded_status.as_deref() == Some("Complete")
                && !nested.incomplete_tail
                && nested.whole.is_none()
                && nested.completed_albums.is_empty()
                && nested.removed_directory_references.is_empty()
                && physical_complete
        }
        _ => false,
    };
    let incomplete_tail = report.incomplete_tail
        || report
            .singles
            .as_ref()
            .is_some_and(|nested| nested.incomplete_tail);
    let terminal = report.recorded_finished
        && !incomplete_tail
        && completed.len() == albums.len()
        && report.pending_albums.is_empty()
        && nested_complete;
    evidence.push(RecoveryEvidence {
        role: "journal_completion".into(),
        path: format!("{}.album-move.jsonl", recorded.job_id),
        status: if terminal { "Verified" } else { "Unverified" }.into(),
    });
    for reference in &pending {
        evidence.push(RecoveryEvidence {
            role: "pending_directory".into(),
            path: display(reference),
            status: "Unverified".into(),
        });
    }
    let directory_roles = [
        "source_root",
        "target_root",
        "album_source",
        "album_destination",
        "directory",
    ];
    let observed_directories = evidence
        .iter()
        .filter(|row| {
            directory_roles.contains(&row.role.as_str())
                && matches!(row.status.as_str(), "Verified" | "AbsentExpected")
        })
        .count();
    let rejected_directories = evidence
        .iter()
        .filter(|row| {
            directory_roles.contains(&row.role.as_str())
                && matches!(row.status.as_str(), "Changed" | "Missing")
        })
        .count();
    let unverified_directories = evidence
        .iter()
        .filter(|row| directory_roles.contains(&row.role.as_str()) && row.status == "Unverified")
        .count();
    let consistent = terminal
        && pending.is_empty()
        && operations.iter().all(|item| item.status == "Verified")
        && evidence
            .iter()
            .all(|row| matches!(row.status.as_str(), "Verified" | "AbsentExpected"));
    // Evidence maps are lookup indexes, not display order. Keep repeated
    // inspections stable while preserving descriptor order for file rows.
    evidence.sort_by(|a, b| (&a.role, &a.path, &a.status).cmp(&(&b.role, &b.path, &b.status)));
    Ok(RecoveryReport {
        verification_scope: Some(WHOLE_VERIFICATION_SCOPE),
        job_id: recorded.job_id.clone(),
        state: if consistent {
            "consistent"
        } else {
            "needs_attention"
        },
        observed_directories,
        pending_directories: pending.len(),
        rejected_directories,
        incomplete_tail,
        unverified_directories,
        observation_limit_reached: false,
        operations,
        evidence,
    })
}
