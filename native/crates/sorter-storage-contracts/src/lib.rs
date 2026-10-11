//! Identity-bound, capability-restricted mutation contracts.
//!
//! This is deliberately separate from the read-only storage contract. A readable
//! document, a writable flag, or generic Android `DocumentsContract` flags do not
//! grant any of these guarantees. In particular, read-check-delete is never a
//! substitute for conditional or leased retirement. This contract is not a host
//! authorization token, a shipping apply entry point, or Android implementation.

use sorter_core::discovery::{ObjectIdentity, StorageRef};
use std::sync::atomic::AtomicBool;

/// Unknown capabilities fail closed, just like explicitly unsupported ones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Capability {
    #[default]
    Unknown,
    Unsupported,
    Supported,
}
impl Capability {
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

/// Evidence about a particular validated storage/provider, not generic API flags.
/// Native platform acceptance and application authorization remain separate gates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutationCapabilities {
    pub seek: Capability,
    pub exclusive_create: Capability,
    pub no_replace_publish: Capability,
    pub same_volume_no_replace_rename: Capability,
    pub content_verification: Capability,
    pub durable_flush: Capability,
    pub leased_source_retirement: Capability,
    pub staged_replacement: Capability,
}

/// Names are never references or evidence of identity. Windows names preserve
/// unpaired UTF-16 units; provider names are opaque display-name strings. An
/// adapter must reject the other platform's variant, invalid leaf names and
/// unsupported names rather than normalize, reinterpret, or silently suffix them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationName {
    Windows(Vec<u16>),
    Provider(String),
}

/// Digest and actual byte count read from the same held object/snapshot.
/// Unknown declared size is permitted; unknown verified content is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentDigest {
    pub sha256: [u8; 32],
    pub length: u64,
}

/// Serialized planning facts alone cannot construct an associated SourceLease.
/// Acquisition compares every supplied precondition and reads `digest` from the
/// leased object. Size is an optional provider observation, not an EOF boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedSource {
    pub identity: ObjectIdentity,
    pub parent_identity: ObjectIdentity,
    pub name: MutationName,
    pub size: Option<u64>,
    pub modified: Option<String>,
    pub digest: ContentDigest,
}

/// Observed references are evidence only, not leases or permission to mutate.
/// A provider may change its URI/document ID on rename; the returned reference,
/// identity, parent and actual name must be observed anew from that operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationObservation {
    pub reference: StorageRef,
    pub identity: ObjectIdentity,
    pub parent_identity: ObjectIdentity,
    pub actual_name: MutationName,
    pub size: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationErrorKind {
    Io,
    PermissionDenied,
    RevokedGrant,
    DiskFull,
    Locked,
    StalePlan,
    UnsafeReparse,
    Unsupported,
    InvalidName,
    Cancelled,
    VerificationFailed,
    UnknownOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationError {
    pub kind: MutationErrorKind,
    pub detail: String,
    /// Preserve any actual returned reference for reconciliation, even when its
    /// name is unexpected. Never reconstruct this from the requested name.
    pub observed: Option<Box<MutationObservation>>,
}
impl MutationError {
    pub fn new(kind: MutationErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            observed: None,
        }
    }
    pub fn with_observation(mut self, observed: MutationObservation) -> Self {
        self.observed = Some(Box::new(observed));
        self
    }
}
impl std::fmt::Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}
impl std::error::Error for MutationError {}

/// An error after dispatch does not prove absence of an effect.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "storage outcomes must be journaled; pending outcomes forbid automatic retry"]
pub enum MutationOutcome<T> {
    /// The adapter proves this invocation made no storage change and cannot
    /// complete later. `Cancelled` is valid only before mutation dispatch (or
    /// after definitive provider confirmation of no effect).
    NoEffect(MutationError),
    /// The returned effect is positively observed. Cancellation raised during
    /// dispatch cannot turn this into NoEffect or discard the returned evidence.
    Applied(T),
    /// The request may still complete, a response was lost, or an unexpected
    /// effect needs reconciliation. Preserve evidence; do not retry, count a
    /// success, clean up by guessed name, or retire a source automatically.
    PendingUnknown(MutationError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RetirementOutcome {
    /// A platform accepted a delete disposition on the held object. This does
    /// not prove namespace removal (other handles/mappings may defer it), and
    /// must not increment completed moves or be journaled as SourceRetired.
    DispositionRequested(MutationObservation),
    /// The held object is positively known to have been retired. This guarantee
    /// must come from provider semantics, never a read-check-delete sequence.
    Retired(MutationObservation),
}

/// Platform guards are opaque, non-forgeable, non-serializable adapter-owned
/// objects. Keep relevant ancestor guards and the source lease alive through the
/// complete operation. The adapter validates that guards belong to itself.
/// Dropping any guard must not delete a source or an ambiguous/unowned object.
///
/// Implementations must check their relevant capabilities at dispatch, not rely
/// on callers having inspected `capabilities()` earlier. Capability availability
/// can change after a permission/grant revocation. Cancellation stops new work;
/// it does not cancel an already-dispatched provider request by assertion.
pub trait MutationStorage {
    type Parent;
    type SourceLease;
    type OwnedTemp;
    type Published;

    fn capabilities(&self) -> MutationCapabilities;

    /// Acquire the specific expected object under the held expected parent,
    /// comparing identity, exact name, optional observations and required digest.
    /// Ambiguous duplicate names, replaced content, or inability to hold a real
    /// identity/content lease fail closed. No mutation takes place.
    fn acquire_source(
        &mut self,
        parent: &Self::Parent,
        expected: &ExpectedSource,
    ) -> Result<Self::SourceLease, MutationError>;

    /// Allocate a newly owned object exclusively, never adopt an existing one.
    /// `job_id` is correlation, not a leaf name or authorization. Observe the
    /// actual reference/name even if the provider chooses another temporary name.
    /// Reject generic providers that cannot prove exclusive ownership.
    fn create_temp(
        &mut self,
        parent: &Self::Parent,
        job_id: &str,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Self::OwnedTemp>;

    /// Publish this exact owned object at the exact planned name without replace.
    /// A collision is StalePlan, never an invitation to choose another suffix.
    /// A wrong returned name/identity is an ambiguous effect, not success. No
    /// copy/delete or check-then-rename emulation can strengthen provider promises.
    fn publish_no_replace(
        &mut self,
        temp: &mut Self::OwnedTemp,
        parent: &Self::Parent,
        name: &MutationName,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Self::Published>;

    /// Atomically rename the leased original without replacement on validated
    /// same-volume storage. Source removal is intrinsic to this operation and
    /// requires its own intent/observation journal phases, not later retirement.
    fn rename_no_replace(
        &mut self,
        source: &mut Self::SourceLease,
        parent: &Self::Parent,
        name: &MutationName,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Self::Published>;

    /// Verify actual EOF, full digest and identity from the published guard.
    /// Do not trust a cached successful write, declared size, or guessed URI.
    fn verify(
        &mut self,
        published: &Self::Published,
        digest: &ContentDigest,
    ) -> Result<MutationObservation, MutationError>;

    /// Use the very same lease that established source identity/content, never
    /// reopen by name/URI or perform an optimistic read-check-delete. The host
    /// must first establish every required distinct destination's verification.
    fn retire_source(
        &mut self,
        source: &mut Self::SourceLease,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome>;

    /// Read current evidence through the published guard. This must not turn a
    /// name lookup into ownership, infer absence from a provider error, or parse
    /// an Android document ID/tree URI as a filesystem path.
    fn inspect(
        &mut self,
        published: &Self::Published,
    ) -> Result<MutationObservation, MutationError>;
}
