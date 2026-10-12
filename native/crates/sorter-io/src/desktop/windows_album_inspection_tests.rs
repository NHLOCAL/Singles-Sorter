use super::*;
use std::os::windows::fs::MetadataExt;

mod reparse_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/windows_reparse.rs"
    ));
}

fn no_progress(_: WindowsInspectionEvent) -> Result<(), MutationError> {
    Ok(())
}
fn id(path: &Path) -> ObjectIdentity {
    identity(chain(path).unwrap().last().unwrap()).unwrap()
}
fn grant(path: &Path) -> WindowsAlbumInspectionParentGrant {
    WindowsAlbumInspectionParentGrant::new(path.to_path_buf(), id(path)).unwrap()
}
fn recorded(path: &Path) -> MutationObservation {
    let files = chain(path).unwrap();
    read_observation(
        files.last().unwrap(),
        &id(path.parent().unwrap()),
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap()
}
struct Fixture {
    tree: tempfile::TempDir,
    parent: PathBuf,
    source: PathBuf,
    original: MutationObservation,
}
impl Fixture {
    fn new() -> Self {
        let tree = tempfile::tempdir().unwrap();
        let parent = tree.path().join("selected containing directory");
        let source = parent.join("אלבום ארוך 𐐀");
        std::fs::create_dir_all(source.join("empty directory")).unwrap();
        std::fs::write(source.join("song.mp3"), b"owned media content").unwrap();
        std::fs::write(source.join("song.mp3:notes"), b"owned named stream").unwrap();
        std::fs::write(
            parent.join("unselected sibling.txt"),
            b"sibling must not change",
        )
        .unwrap();
        let original = recorded(&source);
        Self {
            tree,
            parent,
            source,
            original,
        }
    }
    fn open(&self) -> WindowsAlbumOriginalLocationInspection {
        WindowsAlbumOriginalLocationInspection::open_observed(
            grant(&self.parent),
            self.original.clone(),
            &AtomicBool::new(false),
            &mut no_progress,
        )
        .unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Node {
    path: Vec<u16>,
    identity: ObjectIdentity,
    directory: bool,
    readonly: bool,
    attributes: u32,
    size: u64,
    created: u64,
    modified: u64,
    main: Option<Vec<u8>>,
    notes: Option<Vec<u8>>,
}
fn snapshot(root: &Path) -> Vec<Node> {
    fn visit(root: &Path, path: &Path, result: &mut Vec<Node>) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let digest = |path: &Path| {
            std::fs::read(path)
                .ok()
                .map(|bytes| Sha256::digest(bytes).to_vec())
        };
        result.push(Node {
            path: path
                .strip_prefix(root)
                .unwrap()
                .as_os_str()
                .encode_wide()
                .collect(),
            identity: id(path),
            directory: metadata.is_dir(),
            readonly: metadata.permissions().readonly(),
            attributes: metadata.file_attributes(),
            size: metadata.file_size(),
            created: metadata.creation_time(),
            modified: metadata.last_write_time(),
            main: metadata.is_file().then(|| digest(path).unwrap()),
            notes: if metadata.is_file() {
                let mut named = path.as_os_str().to_os_string();
                named.push(":notes");
                digest(Path::new(&named))
            } else {
                None
            },
        });
        if metadata.is_dir() {
            let mut children = std::fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(root, &child, result);
            }
        }
    }
    let mut result = Vec::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn fresh_parent_reads_original_unicode_leaf_without_owned_tree_changes() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let reader = fixture.open();
    let actual = reader
        .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    assert_eq!(
        actual,
        WindowsAlbumOriginalLocation::OriginalPresent(fixture.original.clone())
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
    drop(reader);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn missing_foreign_and_original_reappearance_remain_distinct() {
    let fixture = Fixture::new();
    let reader = fixture.open();
    let retired = fixture.parent.join("retired original");
    std::fs::rename(&fixture.source, &retired).unwrap();
    let before = snapshot(fixture.tree.path());
    assert_eq!(
        reader
            .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::Absent
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
    std::fs::create_dir(&fixture.source).unwrap();
    std::fs::write(
        fixture.source.join("foreign.txt"),
        b"retain foreign content",
    )
    .unwrap();
    let before = snapshot(fixture.tree.path());
    let actual = reader
        .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    assert!(
        matches!(actual, WindowsAlbumOriginalLocation::ForeignPresent(ref observed) if observed.identity != fixture.original.identity)
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
    std::fs::rename(&fixture.source, fixture.parent.join("foreign retained")).unwrap();
    std::fs::rename(&retired, &fixture.source).unwrap();
    let before = snapshot(fixture.tree.path());
    assert_eq!(
        reader
            .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::OriginalPresent(fixture.original.clone())
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn a_file_at_the_original_directory_leaf_is_foreign_not_absent() {
    let fixture = Fixture::new();
    let reader = fixture.open();
    std::fs::rename(&fixture.source, fixture.parent.join("retired")).unwrap();
    std::fs::write(&fixture.source, b"foreign file").unwrap();
    let before = snapshot(fixture.tree.path());
    let actual = reader
        .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    assert!(
        matches!(actual, WindowsAlbumOriginalLocation::ForeignPresent(ref observed) if observed.size == Some(12))
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn wrong_parent_and_replaced_selected_parent_are_refused() {
    let fixture = Fixture::new();
    let wrong = fixture.tree.path().join("wrong parent");
    std::fs::create_dir(&wrong).unwrap();
    let before = snapshot(fixture.tree.path());
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        grant(&wrong),
        fixture.original.clone(),
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(snapshot(fixture.tree.path()), before);
    let selected = grant(&fixture.parent);
    std::fs::rename(
        &fixture.parent,
        fixture.tree.path().join("old parent retained"),
    )
    .unwrap();
    std::fs::create_dir(&fixture.parent).unwrap();
    let before = snapshot(fixture.tree.path());
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        selected,
        fixture.original.clone(),
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn same_parent_identity_at_a_different_native_path_is_not_silently_rebound() {
    let fixture = Fixture::new();
    let moved = fixture.tree.path().join("different containing path");
    std::fs::rename(&fixture.parent, &moved).unwrap();
    let before = snapshot(fixture.tree.path());
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        grant(&moved),
        fixture.original.clone(),
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn selected_alias_cannot_replace_the_exact_recorded_native_leaf() {
    let fixture = Fixture::new();
    std::fs::rename(&fixture.source, fixture.parent.join("retired native root")).unwrap();
    let mut forged = fixture.original.clone();
    forged.actual_name = MutationName::Windows("ALBUM~1".encode_utf16().collect());
    let before = snapshot(fixture.tree.path());
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        grant(&fixture.parent),
        forged,
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn malformed_leaf_and_outside_native_reference_are_refused_before_lookup() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    for leaf in [
        "",
        ".",
        "..",
        "sibling\\child",
        "sibling/child",
        "source:notes",
        "NUL",
        "end.",
        "end ",
    ] {
        let mut forged = fixture.original.clone();
        forged.actual_name = MutationName::Windows(leaf.encode_utf16().collect());
        let mut observations = 0;
        let result = WindowsAlbumOriginalLocationInspection::open_observed(
            grant(&fixture.parent),
            forged,
            &AtomicBool::new(false),
            &mut |_| {
                observations += 1;
                Ok(())
            },
        );
        assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::InvalidName));
        assert_eq!(observations, 0);
    }
    let mut forged = fixture.original.clone();
    forged.reference = StorageRef::WindowsPath {
        utf16: fixture
            .tree
            .path()
            .join("outside")
            .as_os_str()
            .encode_wide()
            .collect(),
    };
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        grant(&fixture.parent),
        forged,
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn cancellation_and_probe_failure_never_become_absence() {
    let fixture = Fixture::new();
    let cancelled_flag = AtomicBool::new(false);
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        grant(&fixture.parent),
        fixture.original.clone(),
        &cancelled_flag,
        &mut |_| {
            cancelled_flag.store(true, Ordering::Release);
            Ok(())
        },
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    let reader = fixture.open();
    std::fs::rename(&fixture.source, fixture.parent.join("retired")).unwrap();
    let before = snapshot(fixture.tree.path());
    let result = reader.inspect_original_observed(&AtomicBool::new(false), &mut |_| {
        Err(err(
            MutationErrorKind::PermissionDenied,
            "owned injected probe refusal",
        ))
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::PermissionDenied));
    let flag = AtomicBool::new(false);
    let result = reader.inspect_original_observed(&flag, &mut |_| {
        flag.store(true, Ordering::Release);
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(snapshot(fixture.tree.path()), before);
    drop(reader);
    std::fs::rename(
        &fixture.parent,
        fixture.tree.path().join("released after cancellation"),
    )
    .unwrap();
}

#[test]
fn parent_relocation_is_refused_or_detected_before_reading_an_absent_leaf() {
    let fixture = Fixture::new();
    let reader = fixture.open();
    let before = snapshot(fixture.tree.path());
    let moved = fixture.tree.path().join("moved parent");
    match std::fs::rename(&fixture.parent, &moved) {
        Err(error) => {
            assert!(
                matches!(error.raw_os_error(), Some(32 | 33)),
                "unexpected relocation refusal: {error}"
            );
            assert_eq!(snapshot(fixture.tree.path()), before);
        }
        Ok(()) => {
            let result =
                reader.inspect_original_observed(&AtomicBool::new(false), &mut no_progress);
            assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
        }
    }
    drop(reader);
    if fixture.parent.exists() {
        std::fs::rename(&fixture.parent, &moved).unwrap();
    }
}

#[test]
fn last_parent_boundary_relocation_never_returns_rebound_absence_or_presence() {
    for absent in [true, false] {
        let fixture = Fixture::new();
        let reader = fixture.open();
        if absent {
            std::fs::rename(&fixture.source, fixture.parent.join("retired original")).unwrap();
        }
        let parent_id = id(&fixture.parent);
        let before = snapshot(fixture.tree.path());
        let moved = fixture.tree.path().join("relocated at final boundary");
        let final_probe = reader.ancestors.len() + if absent { 4 } else { 7 };
        let mut probes = 0;
        let mut relocation = None;
        let result = reader.inspect_original_observed(&AtomicBool::new(false), &mut |_| {
            probes += 1;
            if probes == final_probe {
                relocation = Some(std::fs::rename(&fixture.parent, &moved));
            }
            Ok(())
        });
        match relocation.expect("fixture must reach the final parent validation") {
            Ok(()) => {
                assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan))
            }
            Err(error) => {
                assert!(
                    matches!(error.raw_os_error(), Some(5 | 32 | 33)),
                    "unexpected relocation refusal: {error}"
                );
                assert!(result.is_ok());
                assert_eq!(snapshot(fixture.tree.path()), before);
                assert!(!moved.exists());
                drop(reader);
                std::fs::rename(&fixture.parent, &moved).unwrap();
                assert_eq!(id(&moved), parent_id);
            }
        }
    }
}

#[test]
fn cancellation_at_final_parent_validation_never_reports_absence() {
    let fixture = Fixture::new();
    let reader = fixture.open();
    std::fs::rename(&fixture.source, fixture.parent.join("retired original")).unwrap();
    let before = snapshot(fixture.tree.path());
    let cancel = AtomicBool::new(false);
    let final_probe = reader.ancestors.len() + 4;
    let mut probes = 0;
    let result = reader.inspect_original_observed(&cancel, &mut |_| {
        probes += 1;
        if probes == final_probe {
            cancel.store(true, Ordering::Release);
        }
        Ok(())
    });
    assert_eq!(probes, final_probe);
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn reparse_original_leaf_and_reparse_parent_are_not_absence() {
    let fixture = Fixture::new();
    let reader = fixture.open();
    let retired = fixture.parent.join("retired");
    std::fs::rename(&fixture.source, &retired).unwrap();
    std::fs::create_dir(&fixture.source).unwrap();
    reparse_fixture::set_junction(&fixture.source, &retired);
    let result = reader.inspect_original_observed(&AtomicBool::new(false), &mut no_progress);
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::UnsafeReparse));
    assert_eq!(
        std::fs::read(retired.join("song.mp3")).unwrap(),
        b"owned media content"
    );
    assert_eq!(
        std::fs::read(retired.join("song.mp3:notes")).unwrap(),
        b"owned named stream"
    );
    drop(reader);
    std::fs::remove_dir(&fixture.source).unwrap();
    let selected = grant(&fixture.parent);
    let old = fixture.tree.path().join("retained parent");
    std::fs::rename(&fixture.parent, &old).unwrap();
    std::fs::create_dir(&fixture.parent).unwrap();
    reparse_fixture::set_junction(&fixture.parent, &old);
    let result = WindowsAlbumOriginalLocationInspection::open_observed(
        selected,
        fixture.original.clone(),
        &AtomicBool::new(false),
        &mut no_progress,
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::UnsafeReparse));
    assert_eq!(
        std::fs::read(old.join("unselected sibling.txt")).unwrap(),
        b"sibling must not change"
    );
    std::fs::remove_dir(&fixture.parent).unwrap();
}

#[test]
fn dedicated_read_only_parent_grant_accepts_a_local_drive_anchor() {
    let fixture = Fixture::new();
    let anchor = fixture.parent.ancestors().last().unwrap();
    assert!(anchor.file_name().is_none());
    let handles = chain(anchor).unwrap();
    let parent = handles.last().unwrap();
    let parent_identity = identity(parent).unwrap();
    assert!(WindowsSelectedRootGrant::new(anchor.to_path_buf(), parent_identity.clone()).is_err());
    let ObjectIdentity::Windows { volume, .. } = parent_identity else {
        unreachable!()
    };
    let name = MutationName::Windows(
        format!(
            "singles-readonly-{}-{}",
            std::process::id(),
            fixture.tree.path().file_name().unwrap().to_string_lossy()
        )
        .encode_utf16()
        .collect(),
    );
    let reference = child_reference(
        &observed_reference(parent, &AtomicBool::new(false), &mut no_progress).unwrap(),
        &name,
    )
    .unwrap();
    let original = MutationObservation {
        reference,
        actual_name: name,
        identity: ObjectIdentity::Windows {
            volume,
            file_id: [0xa5; 16],
        },
        parent_identity: parent_identity.clone(),
        size: None,
    };
    // No fixture is created in the drive root. This tests only a fresh drive
    // parent and one unique absent leaf, not a completed Album job.
    let before = snapshot(fixture.tree.path());
    let reader = WindowsAlbumOriginalLocationInspection::open_observed(
        WindowsAlbumInspectionParentGrant::new(anchor.to_path_buf(), parent_identity).unwrap(),
        original,
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(
        reader
            .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::Absent
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
}
