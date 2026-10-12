//! Whole-object album Move is a distinct scoped protocol, not a batch of
//! invented per-file rename receipts. No external album writer may run during it.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlbumFailure {
    pub kind: String,
    pub detail: String,
    pub observed: Option<MutationObservation>,
}
impl From<MutationError> for AlbumFailure {
    fn from(error: MutationError) -> Self {
        Self {
            kind: format!("{:?}", error.kind),
            detail: error.detail,
            observed: error.observed.map(|value| *value),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WholeAlbumOutcome {
    pub child_renames: Vec<AlbumChildRenameReceipt>,
    pub operation: usize,
    pub status: String,
    pub detail: String,
    pub failure: Option<AlbumFailure>,
    pub root: Option<MutationObservation>,
    pub descendants: Vec<(Vec<MutationName>, MutationObservation)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlbumChildRenameReceipt {
    pub operation: usize,
    pub planner_index: usize,
    pub observation: MutationObservation,
}
#[derive(Clone, Debug)]
pub struct AlbumChildLocationEvidence {
    pub operation: usize,
    pub planner_index: usize,
    pub at_destination: bool,
    pub observed: Option<MutationObservation>,
    pub error: Option<AlbumFailure>,
}
#[derive(Clone, Debug)]
pub struct WholeAlbumRecovery {
    recorded_singles: Option<(String, bool)>,
    pub child_renames: Vec<AlbumChildRenameReceipt>,
    pub child_locations: Vec<AlbumChildLocationEvidence>,
    pub recorded_finished: bool,
    pub incomplete_tail: bool,
    pub recorded: Vec<WholeAlbumOutcome>,
    pub source_evidence: Vec<AlbumDirectoryEvidence>,
    pub destination_evidence: Vec<AlbumDirectoryEvidence>,
    pub files: Vec<AlbumFileEvidence>,
    pub pending_directories: Vec<StorageRef>,
    pub pending_albums: Vec<usize>,
    pub singles: Option<Box<SelectedAlbumRecovery>>,
}
#[derive(Clone, Debug, Serialize)]
struct Header {
    domain: &'static str,
    plan_sha256: String,
    source: EntryObservation,
    target: EntryObservation,
    observed_target: StorageRef,
    observed_sources: Vec<MutationObservation>,
    singles_binding: Option<BindingData>,
    singles_job_id: Option<String>,
    scope: ScopeEvidence,
    snapshot: Snapshot,
}
enum Authority {
    Child {
        roots: WindowsMutationStorage,
        leases: Vec<WindowsAlbumChildRenameScope>,
    },
    Root(Box<WindowsAlbumRootChildRenameSession>),
}
impl Authority {
    fn roots(&self) -> &WindowsMutationStorage {
        match self {
            Self::Child { roots, .. } => roots,
            Self::Root(session) => session.target_and_journal(),
        }
    }
    fn original(&self, index: usize) -> &MutationObservation {
        match self {
            Self::Child { leases, .. } => leases[index].observation(),
            Self::Root(session) => session.source_observation(),
        }
    }
    fn next_child(&self, index: usize) -> Option<(usize, &WindowsAlbumChildRename)> {
        match self {
            Self::Child { leases, .. } => leases[index].next_child_rename(),
            Self::Root(session) => session.next_child_rename(),
        }
    }
    fn rename_child(
        &mut self,
        index: usize,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumChildRenameObservation> {
        match self {
            Self::Child { roots, leases } => {
                roots.rename_next_album_child(&mut leases[index], cancel)
            }
            Self::Root(session) => session.rename_next_album_child(cancel),
        }
    }
    fn inspect_child(
        &self,
        index: usize,
        child: usize,
        destination: bool,
    ) -> std::result::Result<Option<MutationObservation>, MutationError> {
        match self {
            Self::Child { roots, leases } => {
                roots.inspect_album_child_rename_location(&leases[index], child, destination)
            }
            Self::Root(session) => session.inspect_child_location(child, destination),
        }
    }
    fn rename(
        &mut self,
        index: usize,
        parent: &WindowsParent,
        leaf: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumPublication> {
        match self {
            Self::Child { roots, leases } => {
                roots.rename_completed_album_children(&mut leases[index], parent, leaf, cancel)
            }
            Self::Root(session) if index == 0 => {
                session.rename_completed_album_children(parent, leaf, cancel)
            }
            Self::Root(_) => {
                MutationOutcome::NoEffect(failure("root album operation index mismatch"))
            }
        }
    }
}
fn ordered_children(album: &AlbumGroup) -> Result<Vec<WindowsAlbumChildRename>> {
    album
        .children
        .iter()
        .map(|child| {
            Ok(WindowsAlbumChildRename {
                source: leaf(&child.source)?,
                destination: name(&child.destination_name),
                identity: child.source.identity.clone().ok_or(E::Scope)?,
            })
        })
        .collect()
}
fn tree(scope: &AlbumScope, album: &AlbumGroup) -> Result<Vec<WindowsAlbumTreeEntry>> {
    let root = path(&album.source.reference)?;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for directory in &scope.snapshot.directories {
        if !path(&directory.source.reference)?.starts_with(&root) {
            continue;
        }
        for child in &directory.children {
            if !seen.insert(child.reference.clone()) {
                return Err(E::InvalidBinding);
            }
            let child_path = path(&child.reference)?;
            let relative = child_path
                .strip_prefix(&root)
                .map_err(|_| E::Scope)?
                .components()
                .map(|component| {
                    MutationName::Windows(component.as_os_str().encode_wide().collect())
                })
                .collect::<Vec<_>>();
            if relative.is_empty() {
                return Err(E::Scope);
            }
            let digest = if child.kind == EntryKind::File {
                let evidence = scope
                    .execution
                    .inspected_sources
                    .iter()
                    .find(|s| s.observation == *child)
                    .ok_or(E::Scope)?;
                Some(expected(evidence)?.digest)
            } else {
                None
            };
            entries.push(WindowsAlbumTreeEntry {
                relative,
                identity: child.identity.clone().ok_or(E::Scope)?,
                kind: child.kind,
                digest,
            });
        }
    }
    Ok(entries)
}
pub(super) fn eligible(plan: &JobPlanV2, scope: &AlbumScope) -> bool {
    if plan.request.settings.copy_mode {
        return false;
    }
    let mut singles_started = false;
    plan.operations.iter().all(|op| match &op.kind {
        OperationKind::SingleTransfer { .. } => {
            singles_started = true;
            true
        }
        OperationKind::AlbumTransfer {
            source,
            destination,
            children,
            remove_empty_source,
        } => {
            !singles_started
                && *remove_empty_source
                && scope
                    .snapshot
                    .directories
                    .iter()
                    .find(|d| d.source == *source)
                    .is_some_and(|directory| {
                        directory.children.len() == children.len()
                            && children.iter().all(|child| {
                                let mut expected_destination = destination.clone();
                                expected_destination.components.push(
                                    child
                                        .destination
                                        .components
                                        .last()
                                        .cloned()
                                        .unwrap_or_default(),
                                );
                                child.destination == expected_destination
                                    && directory.children.contains(&child.source)
                            })
                    })
        }
        _ => false,
    })
}
/// Separate one-shot execution of the exact planner's eligible album operations.
pub(super) struct WholeAlbumJob {
    authority: Authority,
    header: Header,
    job_id: String,
    attempted: bool,
    locations: Vec<WindowsJournalLocation>,
    directories: Vec<WindowsParent>,
    created: Vec<MutationObservation>,
    publications: Vec<WindowsAlbumPublication>,
    outcomes: Vec<WholeAlbumOutcome>,
    singles: Option<Box<SelectedAlbumFileJob>>,
    child_renames: Vec<AlbumChildRenameReceipt>,
}
impl WholeAlbumJob {
    pub(super) fn prepare(
        source_grant: WindowsSelectedRootGrant,
        target_grant: WindowsSelectedRootGrant,
        journal_grant: WindowsSelectedRootGrant,
        validated: ValidatedAlbumExecution,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let plan = &validated.plan;
        let scope = &validated.scope;
        let resources = scope.resources();
        let source = validated.source.clone();
        let target = validated.target.clone();
        if !eligible(plan, scope) {
            return Err(E::Unsupported);
        }
        cancelled(cancel)?;
        if !matches!((&scope.execution.source_root,&scope.execution.target_root),(ObjectIdentity::Windows{volume:a,..},ObjectIdentity::Windows{volume:b,..})if a==b)
        {
            return Err(E::Unsupported);
        }
        let root_album = scope
            .snapshot
            .albums
            .iter()
            .any(|a| a.source.reference == source.reference);
        let authority = if root_album {
            if scope.snapshot.albums.len() != 1 {
                return Err(E::Unsupported);
            }
            let session = WindowsMutationStorage::for_selected_album_root_child_renames(
                source_grant,
                target_grant,
                journal_grant,
                tree(scope, &scope.snapshot.albums[0])?,
                ordered_children(&scope.snapshot.albums[0])?,
                cancel,
            )
            .map_err(|_| E::Stale)?;
            if session.selected_source_reference() != &source.reference
                || Some(session.selected_source_identity()) != source.identity.as_ref()
            {
                return Err(E::Scope);
            }
            Authority::Root(Box::new(session))
        } else {
            let roots = WindowsMutationStorage::for_selected_roots(
                source_grant,
                target_grant,
                journal_grant,
            )
            .map_err(|_| E::Scope)?;
            if roots.selected_source_reference().map_err(|_| E::Scope)? != source.reference
                || Some(
                    roots
                        .selected_source_root()
                        .map_err(|_| E::Scope)?
                        .identity(),
                ) != source.identity.as_ref()
            {
                return Err(E::Scope);
            }
            let mut leases = Vec::new();
            for album in &scope.snapshot.albums {
                cancelled(cancel)?;
                let parent = roots
                    .parent_reference(&parent_reference(&album.source)?)
                    .map_err(|_| E::Stale)?;
                leases.push(
                    roots
                        .acquire_album_directory_for_child_renames(
                            &parent,
                            &leaf(&album.source)?,
                            album.source.identity.as_ref().ok_or(E::Scope)?,
                            tree(scope, album)?,
                            ordered_children(album)?,
                            cancel,
                        )
                        .map_err(|_| E::Stale)?,
                );
            }
            Authority::Child { roots, leases }
        };
        if authority
            .roots()
            .selected_target_reference()
            .map_err(|_| E::Scope)?
            != target.reference
            || Some(
                authority
                    .roots()
                    .selected_target_root()
                    .map_err(|_| E::Scope)?
                    .identity(),
            ) != target.identity.as_ref()
        {
            return Err(E::Scope);
        }
        let singles = derive_singles_job(&authority, &validated, cancel)?;
        let execution = &scope.execution;
        let header = Header {
            domain: "SelectedWindowsWholeAlbumRenameV1",
            plan_sha256: plan.binding_sha256.clone(),
            source,
            target,
            observed_target: authority
                .roots()
                .selected_target_observed_reference()
                .map_err(|_| E::Stale)?,
            observed_sources: (0..scope.snapshot.albums.len())
                .map(|index| authority.original(index).clone())
                .collect(),
            singles_binding: singles.as_ref().map(|job| job.binding.data.clone()),
            singles_job_id: singles.as_ref().map(|job| job.job_id.clone()),
            scope: ScopeEvidence {
                inspected_plan: execution.inspected_plan.clone(),
                source_root: execution.source_root.clone(),
                target_root: execution.target_root.clone(),
                inspected_sources: execution.inspected_sources.clone(),
                resources: resources.clone(),
                allowed_modes: execution.allowed_modes.clone(),
            },
            snapshot: scope.snapshot.clone(),
        };
        if serde_json::to_vec(&header).map_err(|_| E::Io)?.len() > MAX_MANIFEST - 1024 {
            return Err(E::Unsupported);
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| E::Io)?
            .as_nanos();
        let job_id = format!(
            "album-move-{}-{now}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        Ok(Self {
            authority,
            header,
            job_id,
            attempted: false,
            locations: vec![],
            directories: vec![],
            created: vec![],
            publications: vec![],
            outcomes: vec![],
            singles,
            child_renames: vec![],
        })
    }
    pub(super) fn job_id(&self) -> &str {
        &self.job_id
    }
    pub(super) fn journal_locations(&self) -> &[WindowsJournalLocation] {
        &self.locations
    }
    pub(super) fn binding_sha256(&self) -> Result<String> {
        crate::hash(&self.header)
    }
    pub(super) fn run(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<SelectedAlbumResult> {
        if self.attempted {
            return Err(E::NeedsRecovery);
        }
        self.attempted = true;
        cancelled(cancel)?;
        let parent = self
            .authority
            .roots()
            .selected_journal_root()
            .map_err(|_| E::Scope)?;
        let file = self
            .authority
            .roots()
            .create_journal(&parent, &name(&format!("{}.album-move.jsonl", self.job_id)))
            .map_err(|_| E::Io)?;
        self.locations.push(file.location());
        let mut manifest = Manifest {
            sink: WindowsJournalStore::new(file),
            job_id: self.job_id.clone(),
            sequence: 0,
            previous: String::new(),
            bytes: 0,
            poisoned: false,
        };
        manifest.append("Header", &self.header)?;
        let mut status = JobStatus::Complete;
        let mut detail = String::new();
        for index in 0..self.header.snapshot.albums.len() {
            let album = self.header.snapshot.albums[index].clone();
            if let Err(error) = self.prepare_parent(&album, &mut manifest, cancel) {
                status = error_status(&error);
                detail = format!("album parent preparation stopped: {error}");
                if !manifest.poisoned {
                    manifest.append("ParentStopped",&serde_json::json!({"operation":album.operation,"status":format!("{status:?}")}))?;
                }
                break;
            }
            let destination = destination_path(&album.destination)?;
            let parent = self
                .authority
                .roots()
                .parent_reference(&reference(destination.parent().ok_or(E::Scope)?))
                .map_err(|_| E::Stale)?;
            let destination_name = name(album.destination.components.last().ok_or(E::Scope)?);
            let mut target_reads =
                sorter_io::desktop::DesktopStorage::new(&[path(&self.header.target.reference)?])
                    .map_err(|_| E::Scope)?;
            match target_reads.stat(&reference(&destination)) {
                Err(error) if error.kind == StorageErrorKind::NotFound => {}
                _ => {
                    status = JobStatus::Partial;
                    detail = "album destination changed since preview".into();
                    manifest.append(
                        "ParentStopped",
                        &serde_json::json!({"operation":album.operation,"status":"Partial"}),
                    )?;
                    break;
                }
            }
            drop(target_reads);
            while let Some((planner_index, rename)) = self.authority.next_child(index) {
                let rename = rename.clone();
                manifest.append("ChildRenamePrepared",&serde_json::json!({"operation":album.operation,"planner_index":planner_index,"source":rename.source,"destination":rename.destination,"identity":rename.identity}))?;
                match self.authority.rename_child(index, cancel) {
                    MutationOutcome::Applied(observed) => {
                        let receipt = AlbumChildRenameReceipt {
                            operation: album.operation,
                            planner_index: observed.planner_index,
                            observation: observed.observation,
                        };
                        manifest.append("ChildRenameObserved", &receipt)?;
                        self.child_renames.push(receipt);
                        progress(TransferProgress {
                            operation: album.operation,
                            bytes_written: 0,
                            source_length: 0,
                        });
                    }
                    MutationOutcome::NoEffect(error) => {
                        status = if error.kind == MutationErrorKind::Cancelled {
                            JobStatus::Cancelled
                        } else {
                            JobStatus::Partial
                        };
                        detail = error.detail.clone();
                        manifest.append("ChildRenameStopped",&serde_json::json!({"operation":album.operation,"planner_index":planner_index,"status":format!("{status:?}"),"failure":AlbumFailure::from(error)}))?;
                        break;
                    }
                    MutationOutcome::PendingUnknown(error) => {
                        status = JobStatus::NeedsRecovery;
                        detail = error.detail.clone();
                        manifest.append("ChildRenameUnknown",&serde_json::json!({"operation":album.operation,"planner_index":planner_index,"status":"NeedsRecovery","failure":AlbumFailure::from(error)}))?;
                        break;
                    }
                }
            }
            if status != JobStatus::Complete {
                break;
            }
            manifest.append("AlbumPrepared",&serde_json::json!({"operation":album.operation,"source":album.source,"destination":album.destination,"parent":parent.identity()}))?;
            match self
                .authority
                .rename(index, &parent, &destination_name, cancel)
            {
                MutationOutcome::Applied(publication) => {
                    let outcome = WholeAlbumOutcome {
                        child_renames: self
                            .child_renames
                            .iter()
                            .filter(|item| item.operation == album.operation)
                            .cloned()
                            .collect(),
                        operation: album.operation,
                        status: "complete".into(),
                        detail: String::new(),
                        failure: None,
                        root: Some(publication.observation().clone()),
                        descendants: publication
                            .entries()
                            .iter()
                            .map(|entry| (entry.relative.clone(), entry.observation.clone()))
                            .collect(),
                    };
                    self.publications.push(publication);
                    self.outcomes.push(outcome.clone());
                    manifest.append("AlbumObserved", &outcome)?;
                    progress(TransferProgress {
                        operation: album.operation,
                        bytes_written: 0,
                        source_length: 0,
                    });
                }
                MutationOutcome::NoEffect(error) => {
                    status = if error.kind == MutationErrorKind::Cancelled {
                        JobStatus::Cancelled
                    } else {
                        JobStatus::Partial
                    };
                    detail = error.detail.clone();
                    let outcome = WholeAlbumOutcome {
                        child_renames: self
                            .child_renames
                            .iter()
                            .filter(|item| item.operation == album.operation)
                            .cloned()
                            .collect(),
                        operation: album.operation,
                        status: if status == JobStatus::Cancelled {
                            "cancelled"
                        } else {
                            "failed"
                        }
                        .into(),
                        detail: error.detail.clone(),
                        root: self.root_error_observation(
                            index,
                            &parent,
                            error.observed.as_deref(),
                        ),
                        failure: Some(error.into()),
                        descendants: vec![],
                    };
                    manifest.append("AlbumStopped", &outcome)?;
                    self.outcomes.push(outcome);
                    break;
                }
                MutationOutcome::PendingUnknown(error) => {
                    status = JobStatus::NeedsRecovery;
                    detail = error.detail.clone();
                    let outcome = WholeAlbumOutcome {
                        child_renames: self
                            .child_renames
                            .iter()
                            .filter(|item| item.operation == album.operation)
                            .cloned()
                            .collect(),
                        operation: album.operation,
                        status: "needs_recovery".into(),
                        detail: error.detail.clone(),
                        root: self.root_error_observation(
                            index,
                            &parent,
                            error.observed.as_deref(),
                        ),
                        failure: Some(error.into()),
                        descendants: vec![],
                    };
                    manifest.append("AlbumUnknown", &outcome)?;
                    self.outcomes.push(outcome);
                    break;
                }
            }
        }
        let mut transfers = None;
        let mut nested_created = Vec::new();
        if status == JobStatus::Complete
            && let Some(singles) = &mut self.singles
        {
            // The frozen planner emits every album before deferred singles. The
            // nested binding is a strict subset validated before those effects;
            // no replan is run against the deliberately changed source tree.
            for requirement in &mut singles.snapshot.destinations {
                if requirement.existing.is_none() {
                    let reference = reference(&destination_path(&requirement.destination)?);
                    if let Some(created) = self.created.iter().find(|o| o.reference == reference) {
                        requirement.existing = Some(EntryObservation {
                            reference: created.reference.clone(),
                            name: requirement
                                .destination
                                .components
                                .last()
                                .cloned()
                                .ok_or(E::Scope)?,
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
            }
            manifest.append("SinglesPrepared",&serde_json::json!({"job_id":singles.job_id(),"binding_sha256":singles.binding_sha256()?}))?;
            let result = singles.run(cancel, progress);
            self.locations
                .extend(singles.journal_locations().iter().cloned());
            match result {
                Ok(result) => {
                    status = result.status;
                    detail = result.detail;
                    nested_created = result.created_directories;
                    transfers = result.transfers;
                    manifest.append("SinglesObserved",&serde_json::json!({"status":format!("{status:?}"),"job_id":singles.job_id()}))?;
                }
                Err(error) => {
                    status = if singles.journal_locations().is_empty() {
                        error_status(&error)
                    } else {
                        JobStatus::NeedsRecovery
                    };
                    detail = format!("singles stopped after album transfers: {error}");
                    manifest.append("SinglesStopped",&serde_json::json!({"status":format!("{status:?}"),"job_id":singles.job_id()}))?;
                }
            }
        }
        let albums_completed = self
            .outcomes
            .iter()
            .filter(|o| o.status == "complete")
            .count();
        manifest.append("Finished",&serde_json::json!({"status":format!("{status:?}"),"albums_completed":albums_completed,"outcomes":self.outcomes,"child_renames":self.child_renames,"created":self.created}))?;
        let artist_files = album_artist_counts(
            &self.header.snapshot,
            self.outcomes
                .iter()
                .filter(|o| o.status == "complete")
                .map(|o| o.operation),
        );
        let mut effect_receipts = Vec::new();
        // Each album is processed in order, and deferred singles run afterward.
        // A whole-object rename is one effect, never invented per-file moves.
        for album in &self.header.snapshot.albums {
            effect_receipts.extend(
                self.child_renames
                    .iter()
                    .filter(|receipt| receipt.operation == album.operation)
                    .map(|receipt| crate::results::ObservedEffectReceipt {
                        operation: receipt.operation,
                        kind: "album_child_renamed".into(),
                        identity: receipt.observation.identity.clone(),
                    }),
            );
            if let Some(outcome) = self.outcomes.iter().find(|outcome| {
                outcome.operation == album.operation && outcome.status == "complete"
            }) && let Some(root) = &outcome.root
            {
                effect_receipts.push(crate::results::ObservedEffectReceipt {
                    operation: album.operation,
                    kind: "album_renamed".into(),
                    identity: root.identity.clone(),
                });
            }
        }
        if let Some(result) = &transfers {
            effect_receipts.extend(result.effect_receipts.iter().cloned());
        }
        Ok(SelectedAlbumResult {
            effect_receipts,
            child_renames: self.child_renames.clone(),
            artist_files,
            status,
            transfers,
            albums_completed,
            completed_album_operations: self
                .outcomes
                .iter()
                .filter(|outcome| outcome.status == "complete")
                .map(|outcome| outcome.operation)
                .collect(),
            created_directories: self
                .created
                .iter()
                .chain(&nested_created)
                .cloned()
                .collect(),
            removed_directories: vec![],
            retained_directories: vec![],
            whole_albums: self.outcomes.clone(),
            detail,
        })
    }
    fn prepare_parent(
        &mut self,
        album: &AlbumGroup,
        manifest: &mut Manifest,
        cancel: &AtomicBool,
    ) -> Result<()> {
        for count in 1..album.destination.components.len() {
            cancelled(cancel)?;
            let destination = Destination {
                root: album.destination.root.clone(),
                components: album.destination.components[..count].to_vec(),
            };
            let full = destination_path(&destination)?;
            if self.created.iter().any(|o| o.reference == reference(&full)) {
                continue;
            }
            let expected = self
                .header
                .snapshot
                .destinations
                .iter()
                .find(|d| d.destination == destination)
                .ok_or(E::Scope)?;
            if let Some(existing) = &expected.existing {
                let parent = self
                    .authority
                    .roots()
                    .parent_reference(&existing.reference)
                    .map_err(|_| E::Stale)?;
                if Some(parent.identity()) != existing.identity.as_ref() {
                    return Err(E::Stale);
                }
                self.directories.push(parent);
                continue;
            }
            let parent = self
                .authority
                .roots()
                .parent_reference(&reference(full.parent().ok_or(E::Scope)?))
                .map_err(|_| E::Stale)?;
            let leaf = destination.components.last().ok_or(E::Scope)?;
            manifest.append("DirectoryPrepared",&serde_json::json!({"reference":reference(&full),"parent":parent.identity(),"name":leaf}))?;
            match self
                .authority
                .roots()
                .create_destination_directory(&parent, &name(leaf), cancel)
            {
                MutationOutcome::Applied(directory) => {
                    let observed = MutationObservation {
                        reference: reference(&full),
                        identity: directory.identity().clone(),
                        parent_identity: parent.identity().clone(),
                        actual_name: name(leaf),
                        size: None,
                    };
                    acknowledge_created_directory(
                        manifest,
                        &mut self.directories,
                        &mut self.created,
                        directory,
                        observed,
                    )?;
                }
                MutationOutcome::NoEffect(error) => {
                    return Err(if error.kind == MutationErrorKind::Cancelled {
                        E::Cancelled
                    } else {
                        E::Stale
                    });
                }
                MutationOutcome::PendingUnknown(_) => return Err(E::NeedsRecovery),
            }
        }
        Ok(())
    }
}

impl WholeAlbumJob {
    pub(super) fn inspect_recovery(&mut self, cancel: &AtomicBool) -> Result<WholeAlbumRecovery> {
        cancelled(cancel)?;
        let location = self.locations.first().ok_or(E::Unsupported)?;
        if self
            .authority
            .roots()
            .selected_target_observed_reference()
            .map_err(|_| E::Stale)?
            != self.header.observed_target
        {
            return Err(E::Stale);
        }
        let bytes = self
            .authority
            .roots()
            .read_selected_journal(location.name(), location.identity())
            .map_err(|_| E::Io)?;
        let mut report = self.read_manifest(&bytes, cancel)?;
        // Guard release is not cleanup. The original parent remains privately
        // pinned in the consumed lease/session; only that original leaf is probed.
        self.publications.clear();
        let mut reads =
            sorter_io::desktop::DesktopStorage::new(&[path(&self.header.target.reference)?])
                .map_err(|_| E::Scope)?;
        for (index, album) in self.header.snapshot.albums.iter().enumerate() {
            cancelled(cancel)?;
            let original = match &self.authority {
                Authority::Child { roots, leases } => {
                    roots.inspect_original_album_child_rename_location(&leases[index])
                }
                Authority::Root(session) => session.inspect_original_location(),
            };
            let expected_identity = album.source.identity.clone().ok_or(E::Scope)?;
            let mut source = AlbumDirectoryEvidence {
                reference: album.source.reference.clone(),
                expected_identity: expected_identity.clone(),
                observed_identity: None,
                status: AlbumDirectoryEvidenceStatus::Unverified,
            };
            match original {
                Ok(None) => source.status = AlbumDirectoryEvidenceStatus::Absent,
                Ok(Some(observation)) => {
                    source.status = if observation.identity == expected_identity {
                        AlbumDirectoryEvidenceStatus::SameIdentity
                    } else {
                        AlbumDirectoryEvidenceStatus::Replaced
                    };
                    source.observed_identity = Some(observation.identity);
                }
                Err(_) => {}
            }
            for (child_index, child) in album.children.iter().enumerate() {
                if leaf(&child.source)? != name(&child.destination_name) {
                    for at_destination in [false, true] {
                        let inspected =
                            self.authority
                                .inspect_child(index, child_index, at_destination);
                        let (observed, error) = match inspected {
                            Ok(observed) => (observed, None),
                            Err(error) => (None, Some(error.into())),
                        };
                        report.child_locations.push(AlbumChildLocationEvidence {
                            operation: album.operation,
                            planner_index: child_index,
                            at_destination,
                            observed,
                            error,
                        });
                    }
                }
            }
            report.source_evidence.push(source);
            let destination = reference(&destination_path(&album.destination)?);
            let mut observed = AlbumDirectoryEvidence {
                reference: destination.clone(),
                expected_identity: expected_identity.clone(),
                observed_identity: None,
                status: AlbumDirectoryEvidenceStatus::Unverified,
            };
            match reads.stat(&destination) {
                Ok(entry) => {
                    observed.status = if entry.kind == EntryKind::Directory
                        && entry.identity.as_ref() == Some(&expected_identity)
                    {
                        AlbumDirectoryEvidenceStatus::SameIdentity
                    } else {
                        AlbumDirectoryEvidenceStatus::Replaced
                    };
                    observed.observed_identity = entry.identity;
                }
                Err(error) if error.kind == StorageErrorKind::NotFound => {
                    observed.status = AlbumDirectoryEvidenceStatus::Absent
                }
                Err(_) => {}
            }
            report.destination_evidence.push(observed);
            // Every lookup is reconstructed from the retained planner snapshot,
            // never routed from a serialized observation or recovery renderer.
            for file_index in &album.files {
                let intent = &self.header.snapshot.files[*file_index];
                let source = self
                    .header
                    .scope
                    .inspected_sources
                    .iter()
                    .find(|source| source.observation == intent.source)
                    .ok_or(E::Scope)?;
                let destination = reference(&destination_path(&intent.destination)?);
                let expected_identity = source.observation.identity.clone().ok_or(E::Scope)?;
                let mut evidence = AlbumFileEvidence {
                    operation: album.operation,
                    reference: destination.clone(),
                    expected_identity: expected_identity.clone(),
                    observed_identity: None,
                    status: "unverified".into(),
                };
                match reads.stat(&destination) {
                    Ok(entry) => {
                        evidence.observed_identity = entry.identity.clone();
                        if entry.kind != EntryKind::File
                            || entry.identity.as_ref() != Some(&expected_identity)
                        {
                            evidence.status = "changed".into();
                        } else if let Ok(mut input) = reads.open_read(&destination)
                            && input.binding().is_some_and(|b| {
                                b.identity == expected_identity
                                    && Some(b.size) == source.observation.size
                            })
                        {
                            let mut hash = Sha256::new();
                            let mut length = 0u64;
                            let mut buffer = [0; 65536];
                            let mut read_ok = true;
                            loop {
                                cancelled(cancel)?;
                                match input.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        length = length.checked_add(n as u64).ok_or(E::Io)?;
                                        if Some(length) > source.observation.size {
                                            read_ok = false;
                                            break;
                                        }
                                        hash.update(&buffer[..n]);
                                    }
                                    Err(_) => {
                                        read_ok = false;
                                        break;
                                    }
                                }
                            }
                            if read_ok
                                && Some(length) == source.observation.size
                                && format!("{:x}", hash.finalize()) == source.sha256
                                && reads.stat(&destination).is_ok_and(|again| again == entry)
                            {
                                evidence.status = "verified".into();
                            } else {
                                evidence.status = "changed".into();
                            }
                        }
                    }
                    Err(error) if error.kind == StorageErrorKind::NotFound => {
                        evidence.status = "missing".into()
                    }
                    Err(_) => {}
                }
                report.files.push(evidence);
            }
        }
        if let Some(singles) = &mut self.singles {
            if !singles.journal_locations().is_empty() {
                let nested = singles.inspect_recovery(cancel)?;
                if let Some((status, true)) = &report.recorded_singles
                    && nested.recorded_status.as_ref() != Some(status)
                {
                    return Err(E::CorruptJournal);
                }
                report.singles = Some(Box::new(nested));
            } else if report
                .recorded_singles
                .as_ref()
                .is_some_and(|(_, observed)| *observed)
            {
                return Err(E::CorruptJournal);
            }
        }
        Ok(report)
    }
    fn read_manifest(&self, bytes: &[u8], cancel: &AtomicBool) -> Result<WholeAlbumRecovery> {
        if bytes.len() > MAX_MANIFEST {
            return Err(E::Unsupported);
        }
        let mut report = WholeAlbumRecovery {
            recorded_singles: None,
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
                        || record.payload
                            != serde_json::to_value(&self.header).map_err(|_| E::Io)?
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
    fn root_error_observation(
        &self,
        index: usize,
        parent: &WindowsParent,
        observation: Option<&MutationObservation>,
    ) -> Option<MutationObservation> {
        let observed = observation?;
        let original = &self.header.observed_sources[index];
        if observed.identity != original.identity {
            return None;
        }
        if observed.reference == original.reference
            && observed.actual_name == original.actual_name
            && observed.parent_identity == original.parent_identity
        {
            return Some(observed.clone());
        }
        let album = &self.header.snapshot.albums[index];
        let expected = destination_path(&Destination {
            root: self.header.observed_target.clone(),
            components: album.destination.components.clone(),
        })
        .ok()?;
        if observed.reference == reference(&expected)
            && observed.parent_identity == *parent.identity()
            && observed.actual_name == name(album.destination.components.last()?)
        {
            Some(observed.clone())
        } else {
            None
        }
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
#[derive(Clone, Debug, Serialize)]
pub struct AlbumFileEvidence {
    pub operation: usize,
    pub reference: StorageRef,
    pub expected_identity: ObjectIdentity,
    pub observed_identity: Option<ObjectIdentity>,
    pub status: String,
}

fn derive_singles_job(
    authority: &Authority,
    validated: &ValidatedAlbumExecution,
    cancel: &AtomicBool,
) -> Result<Option<Box<SelectedAlbumFileJob>>> {
    let albums = validated
        .scope
        .snapshot
        .albums
        .iter()
        .map(|album| album.operation)
        .collect::<HashSet<_>>();
    let files = validated
        .scope
        .snapshot
        .files
        .iter()
        .filter(|file| !albums.contains(&file.original_operation))
        .cloned()
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(None);
    }
    if matches!(authority, Authority::Root(_)) {
        return Err(E::Unsupported);
    }
    let destinations = validated
        .scope
        .snapshot
        .destinations
        .iter()
        .filter(|directory| {
            files.iter().any(|file| {
                file.destination
                    .components
                    .starts_with(&directory.destination.components)
                    && file.destination.components.len() > directory.destination.components.len()
            })
        })
        .cloned()
        .collect();
    let mut scope = validated.scope.clone();
    scope
        .execution
        .inspected_sources
        .retain(|source| files.iter().any(|file| file.source == source.observation));
    scope.snapshot = Snapshot {
        copied_directories: vec![],
        directories: vec![],
        destinations,
        files,
        albums: vec![],
    };
    let subset = ValidatedAlbumExecution {
        plan: validated.plan.clone(),
        scope,
        source: validated.source.clone(),
        target: validated.target.clone(),
    };
    let selected = authority.roots();
    // Attenuation only: these exact paths/identities come from the already-held
    // user grants. No renderer data or serialized journal can enter this path.
    let source = WindowsSelectedRootGrant::new(
        path(&selected.selected_source_reference().map_err(|_| E::Scope)?)?,
        selected
            .selected_source_root()
            .map_err(|_| E::Scope)?
            .identity()
            .clone(),
    )
    .map_err(|_| E::Scope)?;
    let target = WindowsSelectedRootGrant::new(
        path(&selected.selected_target_reference().map_err(|_| E::Scope)?)?,
        selected
            .selected_target_root()
            .map_err(|_| E::Scope)?
            .identity()
            .clone(),
    )
    .map_err(|_| E::Scope)?;
    let journal = WindowsSelectedRootGrant::new(
        path(
            &selected
                .selected_journal_reference()
                .map_err(|_| E::Scope)?,
        )?,
        selected
            .selected_journal_root()
            .map_err(|_| E::Scope)?
            .identity()
            .clone(),
    )
    .map_err(|_| E::Scope)?;
    let roots = WindowsMutationStorage::for_selected_roots(source, target, journal)
        .map_err(|_| E::Scope)?;
    Ok(Some(Box::new(SelectedAlbumFileJob::prepare(
        roots, &subset, cancel,
    )?)))
}
