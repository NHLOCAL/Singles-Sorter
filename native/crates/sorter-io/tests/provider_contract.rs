//! Conformance examples using an in-memory, fault-injecting provider. This is not
//! an Android implementation or evidence that any real SAF provider is safe.
//! Strong guarantees below are explicit simulated provider primitives, never
//! inferred from DocumentsContract flags or emulated with read-check-delete.

use sha2::{Digest, Sha256};
use sorter_core::discovery::{ObjectIdentity, StorageRef};
use sorter_io::mutation::*;
use std::collections::HashMap;
use std::io::{self, Read};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

const AUTHORITY: &str = "opaque.fixture.provider";
const ROOT_ID: &str = "not/a/path:root%2F..:אב🎵";
const ORIGINAL: &[u8] = b"original recording bytes, not a real music file";
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

fn digest(bytes: &[u8]) -> ContentDigest {
    ContentDigest {
        sha256: Sha256::digest(bytes).into(),
        length: bytes.len() as u64,
    }
}
fn name(value: &str) -> MutationName {
    MutationName::Provider(value.to_owned())
}
fn identity(document_id: &str) -> ObjectIdentity {
    ObjectIdentity::Android {
        authority: AUTHORITY.to_owned(),
        document_id: document_id.to_owned(),
    }
}
fn error(kind: MutationErrorKind) -> MutationError {
    MutationError::new(kind, "injected synthetic provider result")
}
fn applied<T: std::fmt::Debug>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(value) => value,
        other => panic!("expected Applied, got {other:?}"),
    }
}
fn no_effect<T: std::fmt::Debug>(outcome: MutationOutcome<T>, kind: MutationErrorKind) {
    match outcome {
        MutationOutcome::NoEffect(e) => assert_eq!(e.kind, kind),
        other => panic!("expected NoEffect({kind:?}), got {other:?}"),
    }
}

#[derive(Clone, Debug)]
struct Parent {
    instance: u64,
    id: String,
}
#[derive(Debug)]
struct Lease {
    instance: u64,
    object: u64,
    generation: u64,
    consumed: bool,
}
#[derive(Debug)]
struct Temp {
    instance: u64,
    object: u64,
    usable: bool,
    observation: MutationObservation,
}
#[derive(Debug)]
struct Published {
    instance: u64,
    object: u64,
    observation: MutationObservation,
}
#[derive(Debug)]
struct Document {
    id: String,
    parent: String,
    name: MutationName,
    bytes: Vec<u8>,
    declared_size: Option<u64>,
    modified: String,
    generation: u64,
    temporary: bool,
    leased: bool,
}
impl Document {
    fn observation(&self) -> MutationObservation {
        MutationObservation {
            reference: StorageRef::AndroidDocument {
                tree_uri: format!("content://{AUTHORITY}/tree/opaque-token%3F%2F"),
                authority: AUTHORITY.to_owned(),
                document_id: self.id.clone(),
            },
            identity: identity(&self.id),
            parent_identity: identity(&self.parent),
            actual_name: self.name.clone(),
            size: self.declared_size,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
enum ReplyFault {
    #[default]
    None,
    NullBeforeEffect,
    NullAfterEffect,
    CrashBeforeEffect,
    CrashAfterEffect,
    CancelAfterEffect,
    CancelAndLoseReply,
}
impl ReplyFault {
    fn suppresses_effect(self) -> bool {
        matches!(self, Self::NullBeforeEffect | Self::CrashBeforeEffect)
    }
    fn ambiguous(self) -> bool {
        matches!(
            self,
            Self::NullBeforeEffect
                | Self::NullAfterEffect
                | Self::CrashBeforeEffect
                | Self::CrashAfterEffect
                | Self::CancelAndLoseReply
        )
    }
}

struct FakeSaf {
    instance: u64,
    docs: HashMap<u64, Document>,
    next_object: u64,
    capabilities: MutationCapabilities,
    grant: bool,
    reply_fault: ReplyFault,
    auto_name_temp: bool,
    auto_name_publish: bool,
    dispatches: usize,
    retirement_calls: usize,
    disposition_only: bool,
}
impl FakeSaf {
    fn validated() -> Self {
        Self {
            instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            docs: HashMap::new(),
            next_object: 1,
            capabilities: MutationCapabilities {
                seek: Capability::Unsupported,
                exclusive_create: Capability::Supported,
                no_replace_publish: Capability::Supported,
                same_volume_no_replace_rename: Capability::Supported,
                content_verification: Capability::Supported,
                durable_flush: Capability::Unsupported,
                leased_source_retirement: Capability::Supported,
                staged_replacement: Capability::Unsupported,
            },
            grant: true,
            reply_fault: ReplyFault::None,
            auto_name_temp: false,
            auto_name_publish: false,
            dispatches: 0,
            retirement_calls: 0,
            disposition_only: false,
        }
    }
    fn parent(&self) -> Parent {
        Parent {
            instance: self.instance,
            id: ROOT_ID.to_owned(),
        }
    }
    fn add(&mut self, display_name: &str, bytes: &[u8], size: Option<u64>) -> u64 {
        let object = self.next_object;
        self.next_object += 1;
        self.docs.insert(
            object,
            Document {
                id: format!("opaque/..%2f:document:{object}:שיר🎵"),
                parent: ROOT_ID.to_owned(),
                name: name(display_name),
                bytes: bytes.to_vec(),
                declared_size: size,
                modified: "version-1".to_owned(),
                generation: 0,
                temporary: false,
                leased: false,
            },
        );
        object
    }
    fn expected(&self, object: u64) -> ExpectedSource {
        let doc = &self.docs[&object];
        ExpectedSource {
            identity: identity(&doc.id),
            parent_identity: identity(&doc.parent),
            name: doc.name.clone(),
            size: doc.declared_size,
            modified: Some(doc.modified.clone()),
            digest: digest(&doc.bytes),
        }
    }
    fn check_parent(&self, parent: &Parent) -> Result<(), MutationError> {
        if !self.grant {
            return Err(error(MutationErrorKind::RevokedGrant));
        }
        if parent.instance != self.instance || parent.id != ROOT_ID {
            return Err(error(MutationErrorKind::StalePlan));
        }
        Ok(())
    }
    fn prepare(
        &self,
        parent: &Parent,
        capability: Capability,
        cancelled: &AtomicBool,
    ) -> Result<(), MutationError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(error(MutationErrorKind::Cancelled));
        }
        self.check_parent(parent)?;
        if !capability.is_supported() {
            return Err(error(MutationErrorKind::Unsupported));
        }
        Ok(())
    }
    fn leaf(value: &MutationName) -> Result<&str, MutationError> {
        match value {
            MutationName::Provider(s) if !s.is_empty() && !s.contains('\0') => Ok(s),
            _ => Err(error(MutationErrorKind::InvalidName)),
        }
    }
    fn take_fault(&mut self) -> ReplyFault {
        self.dispatches += 1;
        std::mem::take(&mut self.reply_fault)
    }
    fn finish<T>(
        fault: ReplyFault,
        value: T,
        observation: MutationObservation,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<T> {
        if matches!(
            fault,
            ReplyFault::CancelAfterEffect | ReplyFault::CancelAndLoseReply
        ) {
            cancelled.store(true, Ordering::Release);
        }
        if fault.ambiguous() {
            MutationOutcome::PendingUnknown(
                error(MutationErrorKind::UnknownOutcome).with_observation(observation),
            )
        } else {
            MutationOutcome::Applied(value)
        }
    }
    /// This is a simulated provider's lease-token check under its own mutation
    /// lock. It does not stat and then invoke deleteDocument on a URI.
    fn leased_object(&self, lease: &Lease) -> Result<&Document, MutationError> {
        if lease.instance != self.instance || lease.consumed {
            return Err(error(MutationErrorKind::StalePlan));
        }
        self.docs
            .get(&lease.object)
            .filter(|d| d.leased && d.generation == lease.generation)
            .ok_or_else(|| error(MutationErrorKind::StalePlan))
    }
    fn occupied(&self, parent: &Parent, target: &MutationName) -> bool {
        self.docs
            .values()
            .any(|d| d.parent == parent.id && d.name == *target)
    }
    fn write_remote(&mut self, temp: &mut Temp, bytes: &[u8]) -> Result<usize, MutationError> {
        if !self.grant {
            return Err(error(MutationErrorKind::RevokedGrant));
        }
        if temp.instance != self.instance || !temp.usable {
            return Err(error(MutationErrorKind::StalePlan));
        }
        let doc = self.docs.get_mut(&temp.object).unwrap();
        // A remote stream may acknowledge short writes. The caller must loop.
        let written = bytes.len().min(3);
        doc.bytes.extend_from_slice(&bytes[..written]);
        Ok(written)
    }
    fn write_all_remote(&mut self, temp: &mut Temp, bytes: &[u8]) {
        let mut offset = 0;
        while offset < bytes.len() {
            offset += self.write_remote(temp, &bytes[offset..]).unwrap();
        }
    }
    fn interrupted_remote_write(&mut self, temp: &mut Temp) -> MutationOutcome<usize> {
        self.docs
            .get_mut(&temp.object)
            .unwrap()
            .bytes
            .extend_from_slice(b"partial");
        temp.usable = false;
        MutationOutcome::PendingUnknown(
            error(MutationErrorKind::UnknownOutcome)
                .with_observation(self.docs[&temp.object].observation()),
        )
    }
}

impl MutationStorage for FakeSaf {
    type Parent = Parent;
    type SourceLease = Lease;
    type OwnedTemp = Temp;
    type Published = Published;

    fn capabilities(&self) -> MutationCapabilities {
        self.capabilities.clone()
    }
    fn acquire_source(
        &mut self,
        parent: &Parent,
        expected: &ExpectedSource,
    ) -> Result<Lease, MutationError> {
        self.check_parent(parent)?;
        if expected.parent_identity != identity(&parent.id) {
            return Err(error(MutationErrorKind::StalePlan));
        }
        let candidates: Vec<u64> = self
            .docs
            .iter()
            .filter_map(|(key, d)| {
                (d.parent == parent.id && d.name == expected.name).then_some(*key)
            })
            .collect();
        if candidates.len() != 1 {
            return Err(error(MutationErrorKind::StalePlan));
        }
        let object = candidates[0];
        let doc = self.docs.get_mut(&object).unwrap();
        if identity(&doc.id) != expected.identity
            || expected
                .size
                .is_some_and(|size| doc.declared_size != Some(size))
            || expected
                .modified
                .as_ref()
                .is_some_and(|modified| *modified != doc.modified)
            || digest(&doc.bytes) != expected.digest
        {
            return Err(error(MutationErrorKind::StalePlan));
        }
        if doc.leased {
            return Err(error(MutationErrorKind::Locked));
        }
        doc.leased = true;
        Ok(Lease {
            instance: self.instance,
            object,
            generation: doc.generation,
            consumed: false,
        })
    }
    fn create_temp(
        &mut self,
        parent: &Parent,
        _job_id: &str,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Temp> {
        if let Err(e) = self.prepare(parent, self.capabilities.exclusive_create, cancelled) {
            return MutationOutcome::NoEffect(e);
        }
        let fault = self.take_fault();
        if fault.suppresses_effect() {
            return MutationOutcome::PendingUnknown(error(MutationErrorKind::UnknownOutcome));
        }
        // Exclusive object allocation is an explicit synthetic provider promise.
        // Existing documents are never selected by a successful create call.
        let actual_name = if self.auto_name_temp {
            format!("provider-chosen ({}).tmp", self.next_object)
        } else {
            format!("private-temp-{}", self.next_object)
        };
        let object = self.add(&actual_name, b"", None);
        self.docs.get_mut(&object).unwrap().temporary = true;
        let observation = self.docs[&object].observation();
        let temp = Temp {
            instance: self.instance,
            object,
            usable: true,
            observation: observation.clone(),
        };
        Self::finish(fault, temp, observation, cancelled)
    }
    fn publish_no_replace(
        &mut self,
        temp: &mut Temp,
        parent: &Parent,
        target: &MutationName,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Published> {
        if let Err(e) = self.prepare(parent, self.capabilities.no_replace_publish, cancelled) {
            return MutationOutcome::NoEffect(e);
        }
        if let Err(e) = Self::leaf(target) {
            return MutationOutcome::NoEffect(e);
        }
        if temp.instance != self.instance
            || !temp.usable
            || !self.docs.get(&temp.object).is_some_and(|d| d.temporary)
        {
            return MutationOutcome::NoEffect(error(MutationErrorKind::StalePlan));
        }
        if self.occupied(parent, target) {
            return MutationOutcome::NoEffect(error(MutationErrorKind::StalePlan));
        }
        let fault = self.take_fault();
        temp.usable = false;
        if fault.suppresses_effect() {
            return MutationOutcome::PendingUnknown(error(MutationErrorKind::UnknownOutcome));
        }
        let doc = self.docs.get_mut(&temp.object).unwrap();
        doc.name = if self.auto_name_publish {
            name("unexpected (1).mp3")
        } else {
            target.clone()
        };
        doc.parent = parent.id.clone();
        doc.id = format!("new-uri-after-publish:{}:opaque%2F", temp.object);
        doc.temporary = false;
        let observation = doc.observation();
        if observation.actual_name != *target {
            return MutationOutcome::PendingUnknown(
                error(MutationErrorKind::UnknownOutcome).with_observation(observation),
            );
        }
        let published = Published {
            instance: self.instance,
            object: temp.object,
            observation: observation.clone(),
        };
        Self::finish(fault, published, observation, cancelled)
    }
    fn rename_no_replace(
        &mut self,
        source: &mut Lease,
        parent: &Parent,
        target: &MutationName,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<Published> {
        if let Err(e) = self.prepare(
            parent,
            self.capabilities.same_volume_no_replace_rename,
            cancelled,
        ) {
            return MutationOutcome::NoEffect(e);
        }
        if let Err(e) = Self::leaf(target).and_then(|_| self.leased_object(source)) {
            return MutationOutcome::NoEffect(e);
        }
        if self.occupied(parent, target) {
            return MutationOutcome::NoEffect(error(MutationErrorKind::StalePlan));
        }
        let fault = self.take_fault();
        source.consumed = true;
        if fault.suppresses_effect() {
            return MutationOutcome::PendingUnknown(error(MutationErrorKind::UnknownOutcome));
        }
        let doc = self.docs.get_mut(&source.object).unwrap();
        doc.id = format!("entirely-unrelated-renamed-id:{}:🥁", source.object);
        doc.name = target.clone();
        doc.parent = parent.id.clone();
        let observation = doc.observation();
        let published = Published {
            instance: self.instance,
            object: source.object,
            observation: observation.clone(),
        };
        Self::finish(fault, published, observation, cancelled)
    }
    fn verify(
        &mut self,
        published: &Published,
        expected: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        if !self.capabilities.content_verification.is_supported() {
            return Err(error(MutationErrorKind::Unsupported));
        }
        let observed = self.inspect(published)?;
        // Deliberately ignores declared size: full stream EOF and digest bind bytes.
        if digest(&self.docs[&published.object].bytes) != *expected {
            return Err(error(MutationErrorKind::VerificationFailed).with_observation(observed));
        }
        Ok(observed)
    }
    fn retire_source(
        &mut self,
        source: &mut Lease,
        cancelled: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        if let Err(e) = self.prepare(
            &self.parent(),
            self.capabilities.leased_source_retirement,
            cancelled,
        ) {
            return MutationOutcome::NoEffect(e);
        }
        let observed = match self.leased_object(source) {
            Ok(doc) => doc.observation(),
            Err(e) => return MutationOutcome::NoEffect(e),
        };
        let fault = self.take_fault();
        self.retirement_calls += 1;
        source.consumed = true;
        if fault.suppresses_effect() {
            return MutationOutcome::PendingUnknown(error(MutationErrorKind::UnknownOutcome));
        }
        let outcome = if self.disposition_only {
            RetirementOutcome::DispositionRequested(observed.clone())
        } else {
            // The synthetic provider consumes its bound lease under the same
            // transaction; no path, display-name lookup or URI reopen is involved.
            self.docs.remove(&source.object).unwrap();
            RetirementOutcome::Retired(observed.clone())
        };
        Self::finish(fault, outcome, observed, cancelled)
    }
    fn inspect(&mut self, published: &Published) -> Result<MutationObservation, MutationError> {
        if !self.grant {
            return Err(error(MutationErrorKind::RevokedGrant));
        }
        if published.instance != self.instance {
            return Err(error(MutationErrorKind::StalePlan));
        }
        let observed = self
            .docs
            .get(&published.object)
            .ok_or_else(|| error(MutationErrorKind::StalePlan))?
            .observation();
        if observed.identity != published.observation.identity
            || observed.parent_identity != published.observation.parent_identity
            || observed.actual_name != published.observation.actual_name
        {
            return Err(error(MutationErrorKind::StalePlan).with_observation(observed));
        }
        Ok(observed)
    }
}

fn source_fixture() -> (FakeSaf, Parent, u64, ExpectedSource) {
    let mut provider = FakeSaf::validated();
    let object = provider.add("שיר 🎵.mp3", ORIGINAL, None);
    let parent = provider.parent();
    let expected = provider.expected(object);
    (provider, parent, object, expected)
}
fn copied_temp(provider: &mut FakeSaf, parent: &Parent) -> Temp {
    let mut temp = applied(provider.create_temp(parent, "synthetic-job", &AtomicBool::new(false)));
    provider.write_all_remote(&mut temp, ORIGINAL);
    temp
}

#[test]
fn unknown_is_not_supported_and_generic_document_flags_do_not_upgrade_it() {
    let mut provider = FakeSaf::validated();
    provider.capabilities = MutationCapabilities::default();
    // A host seeing FLAG_SUPPORTS_WRITE/RENAME/DELETE may not set strong guarantees.
    let _generic_document_flags = (true, true, true);
    let parent = provider.parent();
    assert!(!Capability::Unknown.is_supported());
    assert!(!Capability::Unsupported.is_supported());
    no_effect(
        provider.create_temp(&parent, "job", &AtomicBool::new(false)),
        MutationErrorKind::Unsupported,
    );
    assert_eq!(provider.dispatches, 0);
    assert!(provider.docs.is_empty());
}

#[test]
fn opaque_ids_survive_acquisition_and_changed_uri_after_rename_is_authoritative() {
    let (mut provider, parent, object, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    let target = name("זמר 🥁_3.mp3");
    let published =
        applied(provider.rename_no_replace(&mut lease, &parent, &target, &AtomicBool::new(false)));
    let observed = provider.verify(&published, &expected.digest).unwrap();
    assert_ne!(observed.identity, expected.identity);
    assert_eq!(observed.actual_name, target);
    assert_eq!(observed.parent_identity, identity(ROOT_ID));
    match observed.reference {
        StorageRef::AndroidDocument { document_id, .. } => {
            assert_eq!(document_id, provider.docs[&object].id);
            assert!(document_id.starts_with("entirely-unrelated-renamed-id:"));
        }
        other => panic!("SAF reference became another platform reference: {other:?}"),
    }
    no_effect(
        provider.retire_source(&mut lease, &AtomicBool::new(false)),
        MutationErrorKind::StalePlan,
    );
    assert_eq!(provider.retirement_calls, 0);
}

#[test]
fn full_source_identity_parent_name_and_content_are_all_bound() {
    for changed in 0..7 {
        let (mut provider, parent, object, mut expected) = source_fixture();
        match changed {
            0 => expected.identity = identity("other-document"),
            1 => expected.parent_identity = identity("other-parent"),
            2 => expected.name = name("other-name.mp3"),
            3 => expected.size = Some(123),
            4 => expected.modified = Some("other-version".to_owned()),
            5 => expected.digest = digest(b"different bytes"),
            6 => expected.digest.length += 1,
            _ => unreachable!(),
        }
        assert_eq!(
            provider
                .acquire_source(&parent, &expected)
                .unwrap_err()
                .kind,
            MutationErrorKind::StalePlan
        );
        assert_eq!(provider.docs[&object].bytes, ORIGINAL);
        assert_eq!(provider.dispatches, 0);
    }
}

#[test]
fn duplicate_display_names_are_ambiguous_even_when_one_identity_matches() {
    let (mut provider, parent, object, expected) = source_fixture();
    provider.add("שיר 🎵.mp3", b"unrelated duplicate", None);
    assert_eq!(
        provider
            .acquire_source(&parent, &expected)
            .unwrap_err()
            .kind,
        MutationErrorKind::StalePlan
    );
    assert_eq!(provider.docs[&object].bytes, ORIGINAL);
    assert_eq!(provider.docs.len(), 2);
    assert_eq!(provider.dispatches, 0);
}

#[test]
fn exclusive_temp_records_provider_chosen_name_without_adopting_existing_document() {
    let (mut provider, parent, _, _) = source_fixture();
    let existing = provider.add("private-temp-3", b"unrelated", None);
    provider.auto_name_temp = true;
    let temp =
        applied(provider.create_temp(&parent, "job/../is-not-a-name", &AtomicBool::new(false)));
    assert_eq!(
        temp.observation.actual_name,
        name("provider-chosen (3).tmp")
    );
    assert_ne!(temp.object, existing);
    assert_eq!(provider.docs[&existing].bytes, b"unrelated");
    assert!(provider.docs[&temp.object].temporary);
}

#[test]
fn auto_renamed_publication_is_pending_and_preserves_actual_reference() {
    let (mut provider, parent, source, _) = source_fixture();
    let mut temp = copied_temp(&mut provider, &parent);
    provider.auto_name_publish = true;
    let outcome = provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("planned_3.mp3"),
        &AtomicBool::new(false),
    );
    let MutationOutcome::PendingUnknown(e) = outcome else {
        panic!("must not accept an auto-renamed publication")
    };
    let observed = e.observed.unwrap();
    assert_eq!(observed.actual_name, name("unexpected (1).mp3"));
    assert_eq!(observed.identity, identity(&provider.docs[&temp.object].id));
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
    assert_eq!(provider.retirement_calls, 0);
}

#[test]
fn destination_creation_race_never_overwrites_or_selects_another_suffix() {
    let (mut provider, parent, source, _) = source_fixture();
    let mut temp = copied_temp(&mut provider, &parent);
    let raced = provider.add("planned_3.mp3", b"created after planning", None);
    let before = provider.dispatches;
    no_effect(
        provider.publish_no_replace(
            &mut temp,
            &parent,
            &name("planned_3.mp3"),
            &AtomicBool::new(false),
        ),
        MutationErrorKind::StalePlan,
    );
    assert_eq!(provider.docs[&raced].bytes, b"created after planning");
    assert!(provider.docs[&temp.object].temporary);
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
    assert!(!provider.occupied(&parent, &name("planned_4.mp3")));
    assert_eq!(provider.dispatches, before);
}

#[test]
fn null_or_crash_after_dispatch_is_unknown_with_and_without_storage_effect() {
    for fault in [
        ReplyFault::NullBeforeEffect,
        ReplyFault::NullAfterEffect,
        ReplyFault::CrashBeforeEffect,
        ReplyFault::CrashAfterEffect,
    ] {
        let (mut provider, parent, source, _) = source_fixture();
        let mut temp = copied_temp(&mut provider, &parent);
        provider.reply_fault = fault;
        let outcome = provider.publish_no_replace(
            &mut temp,
            &parent,
            &name("chosen.mp3"),
            &AtomicBool::new(false),
        );
        assert!(
            matches!(outcome, MutationOutcome::PendingUnknown(_)),
            "{fault:?}"
        );
        assert_eq!(
            provider.occupied(&parent, &name("chosen.mp3")),
            !fault.suppresses_effect()
        );
        let before = provider.dispatches;
        no_effect(
            provider.publish_no_replace(
                &mut temp,
                &parent,
                &name("chosen.mp3"),
                &AtomicBool::new(false),
            ),
            MutationErrorKind::StalePlan,
        );
        assert_eq!(provider.dispatches, before);
        assert_eq!(provider.docs[&source].bytes, ORIGINAL);
        assert_eq!(provider.retirement_calls, 0);
    }
}

#[test]
fn lost_creation_response_does_not_authorize_cleanup_or_a_success_count() {
    for fault in [
        ReplyFault::NullBeforeEffect,
        ReplyFault::NullAfterEffect,
        ReplyFault::CrashAfterEffect,
    ] {
        let mut provider = FakeSaf::validated();
        let parent = provider.parent();
        provider.reply_fault = fault;
        assert!(matches!(
            provider.create_temp(&parent, "job", &AtomicBool::new(false)),
            MutationOutcome::PendingUnknown(_)
        ));
        assert_eq!(provider.docs.len(), usize::from(!fault.suppresses_effect()));
        // The test retains any allocated object for explicit reconciliation.
        assert_eq!(provider.dispatches, 1);
    }
}

#[test]
fn revoked_grant_blocks_new_dispatch_and_never_means_missing() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    let mut temp = copied_temp(&mut provider, &parent);
    let published = applied(provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("copy.mp3"),
        &AtomicBool::new(false),
    ));
    let before = provider.dispatches;
    provider.grant = false;
    assert_eq!(
        provider.inspect(&published).unwrap_err().kind,
        MutationErrorKind::RevokedGrant
    );
    assert_eq!(
        provider
            .verify(&published, &expected.digest)
            .unwrap_err()
            .kind,
        MutationErrorKind::RevokedGrant
    );
    no_effect(
        provider.retire_source(&mut lease, &AtomicBool::new(false)),
        MutationErrorKind::RevokedGrant,
    );
    no_effect(
        provider.create_temp(&parent, "job", &AtomicBool::new(false)),
        MutationErrorKind::RevokedGrant,
    );
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
    assert_eq!(provider.dispatches, before);
}

/// A pipe has no Seek implementation and may report no size at all.
struct Pipe {
    input: io::Cursor<Vec<u8>>,
}
impl Read for Pipe {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.input.read(buffer)
    }
}
#[test]
fn unknown_size_and_nonseekable_pipe_are_verified_through_actual_eof() {
    let (mut provider, parent, source, expected) = source_fixture();
    assert_eq!(expected.size, None);
    assert_eq!(provider.capabilities().seek, Capability::Unsupported);
    let _lease = provider.acquire_source(&parent, &expected).unwrap();
    let mut pipe = Pipe {
        input: io::Cursor::new(provider.docs[&source].bytes.clone()),
    };
    let mut temp = applied(provider.create_temp(&parent, "job", &AtomicBool::new(false)));
    let mut buffer = [0; 7];
    loop {
        let n = pipe.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        provider.write_all_remote(&mut temp, &buffer[..n]);
    }
    let published = applied(provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("copy.mp3"),
        &AtomicBool::new(false),
    ));
    assert_eq!(
        provider.verify(&published, &expected.digest).unwrap().size,
        None
    );
    assert_eq!(provider.docs[&published.object].bytes, ORIGINAL);
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
}

#[test]
fn partial_remote_document_cannot_verify_from_claimed_length_or_successful_publish() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut temp = applied(provider.create_temp(&parent, "job", &AtomicBool::new(false)));
    assert_eq!(provider.write_remote(&mut temp, ORIGINAL).unwrap(), 3);
    provider.docs.get_mut(&temp.object).unwrap().declared_size = Some(ORIGINAL.len() as u64);
    let published = applied(provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("partial.mp3"),
        &AtomicBool::new(false),
    ));
    assert_eq!(
        provider
            .verify(&published, &expected.digest)
            .unwrap_err()
            .kind,
        MutationErrorKind::VerificationFailed
    );
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
    assert_eq!(provider.retirement_calls, 0);
}

#[test]
fn interrupted_remote_write_keeps_partial_object_and_source_for_reconciliation() {
    let (mut provider, parent, source, _) = source_fixture();
    let mut temp = applied(provider.create_temp(&parent, "job", &AtomicBool::new(false)));
    assert!(matches!(
        provider.interrupted_remote_write(&mut temp),
        MutationOutcome::PendingUnknown(_)
    ));
    let before = provider.dispatches;
    no_effect(
        provider.publish_no_replace(
            &mut temp,
            &parent,
            &name("copy.mp3"),
            &AtomicBool::new(false),
        ),
        MutationErrorKind::StalePlan,
    );
    assert_eq!(provider.docs[&temp.object].bytes, b"partial");
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
    assert_eq!(provider.dispatches, before);
    assert_eq!(provider.retirement_calls, 0);
}

#[test]
fn cancellation_before_dispatch_proves_no_effect() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    let mut temp = copied_temp(&mut provider, &parent);
    let cancelled = AtomicBool::new(true);
    let before = provider.dispatches;
    no_effect(
        provider.create_temp(&parent, "job", &cancelled),
        MutationErrorKind::Cancelled,
    );
    no_effect(
        provider.publish_no_replace(&mut temp, &parent, &name("copy.mp3"), &cancelled),
        MutationErrorKind::Cancelled,
    );
    no_effect(
        provider.rename_no_replace(&mut lease, &parent, &name("renamed.mp3"), &cancelled),
        MutationErrorKind::Cancelled,
    );
    no_effect(
        provider.retire_source(&mut lease, &cancelled),
        MutationErrorKind::Cancelled,
    );
    assert_eq!(provider.dispatches, before);
    assert_eq!(provider.docs[&source].bytes, ORIGINAL);
}

#[test]
fn cancellation_after_dispatch_cannot_disguise_applied_or_unknown_effects() {
    for fault in [
        ReplyFault::CancelAfterEffect,
        ReplyFault::CancelAndLoseReply,
    ] {
        let (mut provider, parent, source, _) = source_fixture();
        let mut temp = copied_temp(&mut provider, &parent);
        provider.reply_fault = fault;
        let cancelled = AtomicBool::new(false);
        let outcome =
            provider.publish_no_replace(&mut temp, &parent, &name("copy.mp3"), &cancelled);
        assert!(cancelled.load(Ordering::Acquire));
        match fault {
            ReplyFault::CancelAfterEffect => {
                assert!(matches!(outcome, MutationOutcome::Applied(_)))
            }
            ReplyFault::CancelAndLoseReply => {
                assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)))
            }
            _ => unreachable!(),
        }
        assert!(provider.occupied(&parent, &name("copy.mp3")));
        assert_eq!(provider.docs[&source].bytes, ORIGINAL);
        assert_eq!(provider.retirement_calls, 0);
    }
}

#[test]
fn unsupported_source_retirement_never_dispatches_read_check_delete_emulation() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    for capability in [Capability::Unknown, Capability::Unsupported] {
        provider.capabilities.leased_source_retirement = capability;
        no_effect(
            provider.retire_source(&mut lease, &AtomicBool::new(false)),
            MutationErrorKind::Unsupported,
        );
        assert_eq!(provider.docs[&source].bytes, ORIGINAL);
        assert_eq!(provider.retirement_calls, 0);
    }
}

#[test]
fn invalidated_source_lease_cannot_retire_a_replacement() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    // Simulate the provider invalidating an outstanding transaction token during
    // replacement. Its own atomic primitive rejects the invalid token.
    let replacement = provider.docs.get_mut(&source).unwrap();
    replacement.generation += 1;
    replacement.id = "replacement-with-reused-display-name".to_owned();
    replacement.bytes = b"unrelated replacement".to_vec();
    no_effect(
        provider.retire_source(&mut lease, &AtomicBool::new(false)),
        MutationErrorKind::StalePlan,
    );
    assert_eq!(provider.docs[&source].bytes, b"unrelated replacement");
    assert_eq!(provider.retirement_calls, 0);
}

#[test]
fn retirement_uses_same_lease_and_a_disposition_is_not_retired() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    let mut temp = copied_temp(&mut provider, &parent);
    let published = applied(provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("copy.mp3"),
        &AtomicBool::new(false),
    ));
    provider.verify(&published, &expected.digest).unwrap();
    provider.disposition_only = true;
    let result = applied(provider.retire_source(&mut lease, &AtomicBool::new(false)));
    assert!(matches!(result, RetirementOutcome::DispositionRequested(_)));
    assert!(provider.docs.contains_key(&source));
    assert_eq!(provider.retirement_calls, 1);
    no_effect(
        provider.retire_source(&mut lease, &AtomicBool::new(false)),
        MutationErrorKind::StalePlan,
    );
}

#[test]
fn successful_retirement_does_not_follow_new_document_at_old_display_name() {
    let (mut provider, parent, source, expected) = source_fixture();
    let mut lease = provider.acquire_source(&parent, &expected).unwrap();
    let mut temp = copied_temp(&mut provider, &parent);
    let published = applied(provider.publish_no_replace(
        &mut temp,
        &parent,
        &name("copy.mp3"),
        &AtomicBool::new(false),
    ));
    provider.verify(&published, &expected.digest).unwrap();
    // Duplicate name appears after acquisition; the provider retires the held
    // object transactionally, never whichever object a name search finds first.
    let other = provider.add("שיר 🎵.mp3", b"unrelated newcomer", None);
    let result = applied(provider.retire_source(&mut lease, &AtomicBool::new(false)));
    assert!(matches!(result, RetirementOutcome::Retired(ref o) if o.identity == expected.identity));
    assert!(!provider.docs.contains_key(&source));
    assert_eq!(provider.docs[&other].bytes, b"unrelated newcomer");
    assert_eq!(
        provider
            .verify(&published, &expected.digest)
            .unwrap()
            .actual_name,
        name("copy.mp3")
    );
}

#[test]
fn foreign_guards_and_windows_names_are_rejected_before_dispatch() {
    let (mut first, first_parent, _, expected) = source_fixture();
    let mut lease = first.acquire_source(&first_parent, &expected).unwrap();
    let mut temp = copied_temp(&mut first, &first_parent);
    let mut second = FakeSaf::validated();
    let parent = second.parent();
    no_effect(
        second.publish_no_replace(
            &mut temp,
            &parent,
            &name("copy.mp3"),
            &AtomicBool::new(false),
        ),
        MutationErrorKind::StalePlan,
    );
    no_effect(
        second.retire_source(&mut lease, &AtomicBool::new(false)),
        MutationErrorKind::StalePlan,
    );
    assert_eq!(
        second
            .acquire_source(&first_parent, &expected)
            .unwrap_err()
            .kind,
        MutationErrorKind::StalePlan
    );
    no_effect(
        first.publish_no_replace(
            &mut temp,
            &first_parent,
            &MutationName::Windows(vec![0xd800]),
            &AtomicBool::new(false),
        ),
        MutationErrorKind::InvalidName,
    );
    assert_eq!(second.dispatches, 0);
}

#[test]
fn lossless_windows_names_are_not_forced_through_utf8() {
    let utf16 = vec![
        b'x' as u16,
        0xd800,
        b'.' as u16,
        b'm' as u16,
        b'p' as u16,
        b'3' as u16,
    ];
    assert_eq!(
        MutationName::Windows(utf16.clone()),
        MutationName::Windows(utf16)
    );
    assert_ne!(
        MutationName::Windows(vec![0xd800]),
        MutationName::Windows(vec![0xfffd])
    );
}

#[test]
fn lost_rename_reply_does_not_claim_source_absence_or_authorize_a_retry() {
    for fault in [
        ReplyFault::NullBeforeEffect,
        ReplyFault::NullAfterEffect,
        ReplyFault::CrashBeforeEffect,
        ReplyFault::CrashAfterEffect,
    ] {
        let (mut provider, parent, source, expected) = source_fixture();
        let mut lease = provider.acquire_source(&parent, &expected).unwrap();
        provider.reply_fault = fault;
        let outcome = provider.rename_no_replace(
            &mut lease,
            &parent,
            &name("renamed.mp3"),
            &AtomicBool::new(false),
        );
        assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)));
        assert_eq!(
            provider.occupied(&parent, &name("renamed.mp3")),
            !fault.suppresses_effect()
        );
        assert_eq!(provider.docs[&source].bytes, ORIGINAL);
        let before = provider.dispatches;
        no_effect(
            provider.rename_no_replace(
                &mut lease,
                &parent,
                &name("renamed.mp3"),
                &AtomicBool::new(false),
            ),
            MutationErrorKind::StalePlan,
        );
        no_effect(
            provider.retire_source(&mut lease, &AtomicBool::new(false)),
            MutationErrorKind::StalePlan,
        );
        assert_eq!(provider.dispatches, before);
        assert_eq!(provider.retirement_calls, 0);
    }
}

#[test]
fn lost_retirement_reply_stays_unknown_whether_source_remains_or_not() {
    for fault in [
        ReplyFault::NullBeforeEffect,
        ReplyFault::NullAfterEffect,
        ReplyFault::CrashBeforeEffect,
        ReplyFault::CrashAfterEffect,
    ] {
        let (mut provider, parent, source, expected) = source_fixture();
        let mut lease = provider.acquire_source(&parent, &expected).unwrap();
        let mut temp = copied_temp(&mut provider, &parent);
        let published = applied(provider.publish_no_replace(
            &mut temp,
            &parent,
            &name("verified-copy.mp3"),
            &AtomicBool::new(false),
        ));
        provider.verify(&published, &expected.digest).unwrap();
        provider.reply_fault = fault;
        let outcome = provider.retire_source(&mut lease, &AtomicBool::new(false));
        assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)));
        assert_eq!(
            provider.docs.contains_key(&source),
            fault.suppresses_effect()
        );
        // No guessed successful-move count, and verified copies are retained.
        provider.verify(&published, &expected.digest).unwrap();
        no_effect(
            provider.retire_source(&mut lease, &AtomicBool::new(false)),
            MutationErrorKind::StalePlan,
        );
        assert_eq!(provider.retirement_calls, 1);
    }
}
