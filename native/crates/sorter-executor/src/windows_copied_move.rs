//! Separate preview-bound ordinary/quiescent local-NTFS copied Move domain.
use super::*;
use crate::transfer::{TransferStorage, source_expected};
use sorter_storage_contracts::*;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// Retained in-process inspection, deliberately not Deserialize.
#[derive(Clone, Debug)]
pub struct CopiedMoveScope {
    execution: ExecutionScope,
    sources: Vec<CopiedMoveSourceEvidence>,
}
impl CopiedMoveScope {
    pub fn execution_scope(&self) -> &ExecutionScope { &self.execution }
    pub fn sources(&self) -> &[CopiedMoveSourceEvidence] { &self.sources }
}

pub fn supported_copied_move_plan(plan: &JobPlanV2) -> Result<()> {
    if plan.request.mode != JobMode::Sort || plan.request.settings.copy_mode || plan.operations.is_empty()
        || plan.blockers.iter().any(|blocker| blocker != "PHYSICAL_APPLY_UNAVAILABLE" && blocker != "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS")
    { return Err(E::Unsupported); }
    for operation in &plan.operations {
        match &operation.kind {
            OperationKind::SingleTransfer { source, transfer, .. } => {
                if source.kind != EntryKind::File || !transfer.predicted_success || !transfer.blockers.is_empty() {
                    return Err(E::Unsupported);
                }
                let source_path = path(&source.reference)?;
                let leaf = source_path.file_name().ok_or(E::Scope)?.encode_wide().collect::<Vec<_>>();
                if String::from_utf16(&leaf).map_err(|_| E::Unsupported)? != source.name { return Err(E::Unsupported); }
            }
            _ => return Err(E::Unsupported),
        }
    }
    Ok(())
}

fn admission(roots: &WindowsMutationStorage, plan: &JobPlanV2) -> Result<()> {
    let source = roots.selected_source_root().map_err(|_| E::Scope)?;
    let target = roots.selected_target_root().map_err(|_| E::Scope)?;
    let (ObjectIdentity::Windows { volume: source_volume, .. }, ObjectIdentity::Windows { volume: target_volume, .. }) =
        (source.identity(), target.identity()) else { return Err(E::Scope); };
    if source_volume == target_volume && plan.operations.iter().any(|operation|
        !matches!(&operation.kind, OperationKind::SingleTransfer { transfer, .. } if transfer.copy)) {
        return Err(E::Unsupported);
    }
    Ok(())
}
fn preparation_error(error: MutationError) -> E {
    match error.kind { MutationErrorKind::Cancelled => E::Cancelled, MutationErrorKind::Unsupported => E::Unsupported, _ => E::Stale }
}

fn native_evidence(source: &CopiedMoveSourceEvidence) -> WindowsMoveSourceEvidence {
    WindowsMoveSourceEvidence {
        identity: source.identity.clone(), attributes: source.attributes,
        streams: source.streams.iter().map(|stream| WindowsMoveStreamEvidence {
            name: stream.name.clone(), length: stream.length, sha256: stream.sha256,
        }).collect(),
    }
}
fn bound_evidence(reference: StorageRef, source: &WindowsMoveSourceEvidence) -> CopiedMoveSourceEvidence {
    CopiedMoveSourceEvidence {
        source: reference, identity: source.identity.clone(), attributes: source.attributes,
        streams: source.streams.iter().map(|stream| CopiedMoveStreamEvidence {
            name: stream.name.clone(), length: stream.length, sha256: stream.sha256,
        }).collect(),
    }
}
fn expected(source: &SourceEvidence) -> Result<ExpectedSource> {
    source_expected(&BoundOperation {
        id: 0, depends_on: Vec::new(), source: source.clone(),
        destination: DestinationEvidence { id: String::new(), root: source.observation.reference.clone(), components: Vec::new() },
        mode: TransferMode::Move, copy_effect: false, duet_cohort: None,
    })
}

/// Caller supplies existing verified root grants; inspection performs no writes.
/// Journal storage is an explicitly selected application root, never created here.
pub fn prepare_copied_move_scope(
    plan: &JobPlanV2, reads: &mut dyn ReadStorage, roots: &WindowsMutationStorage,
    resources: &Resources, cancel: &AtomicBool,
) -> Result<CopiedMoveScope> {
    supported_copied_move_plan(plan)?;
    if roots.domain() != WindowsMutationDomain::SelectedRoots
        || roots.selected_source_reference().map_err(|_| E::Scope)? != plan.request.source
        || Some(roots.selected_target_reference().map_err(|_| E::Scope)?) != plan.request.target
    { return Err(E::Scope); }
    admission(roots, plan)?;
    let execution = prepare_scope(plan, reads, resources, cancel)?;
    if roots.selected_source_root().map_err(|_| E::Scope)?.identity() != &execution.source_root
        || roots.selected_target_root().map_err(|_| E::Scope)?.identity() != &execution.target_root
    { return Err(E::Scope); }
    let mut sources = Vec::new();
    for source in &execution.inspected_sources {
        if cancel.load(Ordering::Acquire) { return Err(E::Cancelled); }
        let source_path = path(&source.observation.reference)?;
        let parent = roots.parent_reference(&reference(source_path.parent().ok_or(E::Scope)?)).map_err(|_| E::Stale)?;
        let evidence = roots.inspect_copied_move_source(&parent, &expected(source)?, cancel).map_err(preparation_error)?;
        sources.push(bound_evidence(source.observation.reference.clone(), &evidence));
    }
    Ok(CopiedMoveScope { execution, sources })
}

impl SelectedWindowsJob {
    /// All source stream leases are acquired before any journal/artist directory
    /// effects. The generic selected Copy/rename constructor remains unchanged.
    pub fn prepare_copied_move(
        roots: WindowsMutationStorage, plan: &JobPlanV2,
        metadata: &mut dyn MetadataProvider, recognition: &mut dyn RecognitionProvider,
        resources: &Resources, scope: &CopiedMoveScope, cancel: &AtomicBool,
    ) -> Result<Self> {
        supported_copied_move_plan(plan)?;
        if roots.domain() != WindowsMutationDomain::SelectedRoots
            || roots.selected_source_reference().map_err(|_| E::Scope)? != plan.request.source
            || Some(roots.selected_target_reference().map_err(|_| E::Scope)?) != plan.request.target
        { return Err(E::Scope); }
        admission(&roots, plan)?;
        let mut reads = OwnedCopyReadStorage::new(&roots, &plan.request.source, plan.request.target.as_ref().ok_or(E::Scope)?)?;
        let mut binding = validate_execution_domain(plan, &mut reads, metadata, recognition, resources, &scope.execution, ExecutionDomain::SelectedWindowsCopiedMove)?;
        binding.data.copied_move_sources = Some(scope.sources.clone());
        if !binding.data.valid_domain() { return Err(E::InvalidBinding); }
        if serde_json::to_vec(&binding.data).map_err(|_| E::InvalidBinding)?.len() > 8 * 1024 * 1024 {
            return Err(E::Unsupported);
        }
        drop(reads);
        let mut held = Vec::new();
        for source in &scope.sources {
            if cancel.load(Ordering::Acquire) { return Err(E::Cancelled); }
            let operation = binding.operations().iter().find(|op| op.source.observation.reference == source.source).ok_or(E::Scope)?;
            let source_path = path(&source.source)?;
            let parent = roots.parent_reference(&reference(source_path.parent().ok_or(E::Scope)?)).map_err(|_| E::Stale)?;
            let required = binding.required_destinations(&source.source).len();
            let lease = roots.acquire_copied_move_source(&parent, &source_expected(operation)?, &native_evidence(source), required, cancel).map_err(preparation_error)?;
            held.push((source.source.clone(), lease));
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| E::Io)?.as_nanos();
        Ok(Self {
            storage: Storage::CopiedMove(CopiedMoveStorage { roots, held, sources: scope.sources.clone() }),
            binding, job_id: format!("sorter-{}-{nanos}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)),
            attempted: false, created_directories: Vec::new(), journal_locations: Vec::new(), directory_observations: Vec::new(),
        })
    }
}

pub(super) struct CopiedMoveStorage {
    pub(super) roots: WindowsMutationStorage,
    held: Vec<(StorageRef, WindowsCopiedMoveSource)>,
    sources: Vec<CopiedMoveSourceEvidence>,
}
impl MutationStorage for CopiedMoveStorage {
    type Parent = WindowsParent;
    type SourceLease = WindowsCopiedMoveSource;
    type OwnedTemp = WindowsCopiedMoveTemp;
    type Published = WindowsCopiedMovePublished;
    fn capabilities(&self) -> MutationCapabilities {
        MutationCapabilities {
            seek: Capability::Supported, exclusive_create: Capability::Supported,
            no_replace_publish: Capability::Supported, content_verification: Capability::Supported,
            same_volume_no_replace_rename: Capability::Unsupported,
            durable_flush: Capability::Unknown, leased_source_retirement: Capability::Unknown,
            staged_replacement: Capability::Unsupported,
        }
    }
    fn acquire_source(&mut self, parent: &WindowsParent, expected: &ExpectedSource) -> std::result::Result<Self::SourceLease, MutationError> {
        let index = self.held.iter().position(|(_, source)| source.observation().identity == expected.identity
            && source.observation().parent_identity == *parent.identity()
            && source.observation().actual_name == expected.name
        ).ok_or_else(|| MutationError::new(MutationErrorKind::StalePlan, "copied Move source was not preleased"))?;
        Ok(self.held.remove(index).1)
    }
    fn create_temp(&mut self, parent: &WindowsParent, job_id: &str, cancel: &AtomicBool) -> MutationOutcome<Self::OwnedTemp> {
        self.roots.create_copied_move_temp(parent, job_id, cancel)
    }
    fn publish_no_replace(&mut self, temp: &mut Self::OwnedTemp, parent: &WindowsParent, name: &MutationName, cancel: &AtomicBool) -> MutationOutcome<Self::Published> {
        self.roots.publish_copied_move_no_replace(temp, parent, name, cancel)
    }
    fn rename_no_replace(&mut self, _: &mut Self::SourceLease, _: &WindowsParent, _: &MutationName, _: &AtomicBool) -> MutationOutcome<Self::Published> {
        MutationOutcome::NoEffect(MutationError::new(MutationErrorKind::Unsupported, "copied Move never selects rename fallback"))
    }
    fn verify(&mut self, published: &Self::Published, digest: &ContentDigest) -> std::result::Result<MutationObservation, MutationError> {
        let _ = (published, digest);
        Err(MutationError::new(MutationErrorKind::Unsupported, "copied Move verification requires execution cancellation"))
    }
    fn retire_source(&mut self, _: &mut Self::SourceLease, _: &AtomicBool) -> MutationOutcome<RetirementOutcome> {
        MutationOutcome::NoEffect(MutationError::new(MutationErrorKind::Unsupported, "copied Move requires complete bound cohort hook"))
    }
    fn inspect(&mut self, published: &Self::Published) -> std::result::Result<MutationObservation, MutationError> {
        self.roots.inspect_copied_move_publication(published)
    }
}
impl TransferStorage for CopiedMoveStorage {
    fn source_parent(&mut self, source: &SourceEvidence, root: &ObjectIdentity) -> std::result::Result<WindowsParent, MutationError> {
        self.roots.source_parent(source, root)
    }
    fn destination_parent(&mut self, destination: &DestinationEvidence, root: &ObjectIdentity) -> std::result::Result<WindowsParent, MutationError> {
        self.roots.destination_parent(destination, root)
    }
    fn parent_identity(&self, parent: &WindowsParent) -> ObjectIdentity { parent.identity().clone() }
    fn temp_observation(&self, temp: &Self::OwnedTemp) -> MutationObservation { temp.observation().clone() }
    fn rewind_source(&mut self, source: &mut Self::SourceLease) -> std::result::Result<(), MutationError> {
        source.seek(SeekFrom::Start(0)).map(|_| ()).map_err(|error| MutationError::new(MutationErrorKind::Io, error.to_string()))
    }
    fn read_source(&mut self, source: &mut Self::SourceLease, bytes: &mut [u8]) -> io::Result<usize> { source.read(bytes) }
    fn write_temp(&mut self, temp: &mut Self::OwnedTemp, bytes: &[u8]) -> io::Result<usize> { temp.write(bytes) }
    fn sync_temp(&mut self, temp: &mut Self::OwnedTemp) -> std::result::Result<(), MutationError> { temp.sync_data() }
    fn verify_temp(&mut self, temp: &Self::OwnedTemp, digest: &ContentDigest) -> std::result::Result<MutationObservation, MutationError> { temp.verify_content(digest) }
    fn rename_eligible(&self, _: &Self::SourceLease, _: &WindowsParent) -> bool { false }
    fn selected_copied_move_contract(&self) -> bool { true }
    fn verify_selected_copied_move(&mut self, published: &Self::Published, digest: &ContentDigest, cancel: &AtomicBool) -> std::result::Result<MutationObservation, MutationError> {
        self.roots.verify_copied_move_publication(published, digest, cancel)
    }

    fn copy_selected_move_streams(&mut self, source: &Self::SourceLease, temp: &mut Self::OwnedTemp, cancel: &AtomicBool) -> MutationOutcome<CopiedMoveSourceEvidence> {
        let Some(expected) = self.sources.iter().find(|bound| bound.identity == source.evidence().identity) else {
            return MutationOutcome::NoEffect(MutationError::new(MutationErrorKind::StalePlan, "copied source not in binding"));
        };
        match self.roots.copy_bound_move_streams(source, temp, cancel) {
            MutationOutcome::Applied(evidence) => MutationOutcome::Applied(bound_evidence(expected.source.clone(), &evidence)),
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }
    fn finalize_selected_copy_metadata(&mut self, source: &Self::SourceLease, temp: &mut Self::OwnedTemp, cancel: &AtomicBool) -> MutationOutcome<crate::physical_journal::CopyMetadataEvidence> {
        match self.roots.finalize_copied_move_metadata(source, temp, cancel) {
            MutationOutcome::Applied(evidence) => MutationOutcome::Applied(crate::physical_journal::CopyMetadataEvidence {
                source_identity: evidence.source_identity, destination_identity: evidence.destination_identity,
                access_time: evidence.access_time, write_time: evidence.write_time, readonly: evidence.readonly,
                retained_creation_time: evidence.retained_creation_time, retained_attributes: evidence.retained_attributes,
            }),
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }
    fn sync_owned_windows_publication(&mut self, published: &mut Self::Published, digest: &ContentDigest, cancel: &AtomicBool) -> std::result::Result<MutationObservation, MutationError> {
        self.roots.sync_copied_move_publication(published, digest, cancel)
    }
    fn retire_selected_copied_move(&mut self, source: &mut Self::SourceLease, destinations: &[&Self::Published], cancel: &AtomicBool) -> MutationOutcome<RetirementOutcome> {
        self.roots.retire_copied_move_source(source, destinations, cancel)
    }
}
