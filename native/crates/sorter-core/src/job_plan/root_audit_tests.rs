use super::*;

fn entry(scope: &str, document: &str, name: &str) -> EntryObservation {
    EntryObservation {
        reference: StorageRef::AndroidDocument {
            tree_uri: format!("content://com.android.externalstorage.documents/tree/{scope}"),
            authority: "com.android.externalstorage.documents".into(),
            document_id: document.into(),
        },
        identity: None,
        parent_identity: None,
        name: name.into(),
        kind: EntryKind::Directory,
        size: None,
        modified: None,
        capabilities: Capabilities::default(),
    }
}

#[test]
fn distinct_selected_android_roots_are_not_siblings_by_display_name() {
    let source = entry("primary:one/artist", "primary:one/artist", "אברהם פריד");
    let target = entry("primary:two/artist", "primary:two/artist", "אברהם פריד");
    let observations = vec![source.clone(), target.clone()];
    let blockers = audit_observations(&observations, &source.reference, Some(&target.reference));
    assert!(!blockers.iter().any(|b| b == "AMBIGUOUS_DISPLAY_NAME"));
    assert!(blockers.iter().any(|b| b == "MISSING_IDENTITY"));
    assert!(blockers.iter().any(|b| b == "UNKNOWN_CAPABILITY"));
    // Missing identities still do not establish disjoint ancestry.
    assert!(source.identity.is_none() && target.identity.is_none());
}

#[test]
fn distinct_root_case_labels_do_not_bypass_real_child_collisions() {
    let source = entry("primary:one/artist", "primary:one/artist", "Artist");
    let target = entry("primary:two/artist", "primary:two/artist", "artist");
    let mut first = entry("primary:one/artist", "primary:one/artist/a", "same.wav");
    first.parent_identity = Some(ObjectIdentity::Test {
        id: "parent".into(),
    });
    let mut second = first.clone();
    second.reference = entry("primary:one/artist", "primary:one/artist/b", "same.wav").reference;
    let observations = vec![
        source.clone(),
        target.clone(),
        first.clone(),
        second.clone(),
    ];
    let blockers = audit_observations(&observations, &source.reference, Some(&target.reference));
    assert!(blockers.iter().any(|b| b == "AMBIGUOUS_DISPLAY_NAME"));
    assert!(
        !blockers
            .iter()
            .any(|b| b == "CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION")
    );
    second.name = "SAME.WAV".into();
    let observations = vec![source.clone(), target.clone(), first, second];
    let blockers = audit_observations(&observations, &source.reference, Some(&target.reference));
    assert!(
        blockers
            .iter()
            .any(|b| b == "CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION")
    );
}

#[test]
fn selected_root_identity_and_ancestry_overlap_checks_remain() {
    let mut source = entry("primary:one", "primary:one", "same");
    source.identity = Some(ObjectIdentity::Test { id: "one".into() });
    let mut target = entry("primary:alias", "primary:alias", "same");
    target.identity = source.identity.clone();
    let observations = vec![source.clone(), target.clone()];
    assert!(observed_overlap(
        &observations,
        &source.reference,
        Some(&target.reference)
    ));
    assert!(observed_overlap(
        &observations,
        &source.reference,
        Some(&source.reference)
    ));
    target.identity = Some(ObjectIdentity::Test { id: "child".into() });
    target.parent_identity = source.identity.clone();
    let observations = vec![source.clone(), target.clone()];
    assert!(observed_overlap(
        &observations,
        &source.reference,
        Some(&target.reference)
    ));
    assert!(observed_overlap(
        &observations,
        &target.reference,
        Some(&source.reference)
    ));
    source.kind = EntryKind::LinkOrReparse;
    let observations = vec![source.clone(), target.clone()];
    let blockers = audit_observations(&observations, &source.reference, Some(&target.reference));
    assert!(blockers.iter().any(|b| b == "LINK_OR_REPARSE"));
}

#[test]
fn root_overlap_requires_reference_identity_or_observed_ancestry() {
    let mut source = entry("primary:one", "primary:one", "same");
    let mut target = entry("primary:two", "primary:two", "same");
    let observations = vec![source.clone(), target.clone()];
    assert!(!observed_overlap(
        &observations,
        &source.reference,
        Some(&target.reference)
    ));
    assert!(observed_overlap(
        &observations,
        &source.reference,
        Some(&source.reference)
    ));
    assert!(!observed_overlap(&observations, &source.reference, None));
    source.identity = Some(ObjectIdentity::Test { id: "one".into() });
    target.identity = Some(ObjectIdentity::Test { id: "two".into() });
    let observations = vec![source.clone(), target.clone()];
    assert!(!observed_overlap(
        &observations,
        &source.reference,
        Some(&target.reference)
    ));
}
