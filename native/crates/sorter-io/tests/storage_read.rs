#[cfg(any(windows, target_os = "linux"))]
mod support;
use sorter_io::storage::*;
use std::io::{Read, Seek, SeekFrom};

fn test_ref(id: &str) -> StorageRef {
    StorageRef::TestOpaque { id: id.into() }
}
fn entry(id: &str, parent: Option<&str>, name: &str, kind: EntryKind) -> EntryObservation {
    EntryObservation {
        reference: test_ref(id),
        identity: Some(ObjectIdentity::Test { id: id.into() }),
        parent_identity: parent.map(|p| ObjectIdentity::Test { id: p.into() }),
        name: name.into(),
        kind,
        size: None,
        modified: None,
        capabilities: Capabilities {
            read: Some(true),
            list: Some(kind == EntryKind::Directory),
            seek: None,
        },
    }
}
#[test]
fn ordered_memory_listing_keeps_unicode_case_and_changing_entries() {
    let root = entry("root", None, "root", EntryKind::Directory);
    let a = entry("a", Some("root"), "שיר𐐀.mp3", EntryKind::File);
    let b = entry("b", Some("root"), "A.mp3", EntryKind::File);
    let c = entry("c", Some("root"), "a.mp3", EntryKind::File);
    let mut fs = MemoryStorage::default();
    for e in [&root, &a, &b, &c] {
        fs.insert(e.clone(), vec![1, 2, 3]);
    }
    fs.set_children(
        &root.reference,
        vec![
            c.reference.clone(),
            a.reference.clone(),
            b.reference.clone(),
        ],
    );
    assert_eq!(
        fs.children(&root.reference).unwrap(),
        vec![c.clone(), a.clone(), b.clone()]
    );
    fs.remove(&a.reference);
    assert_eq!(
        fs.children(&root.reference).unwrap_err().kind,
        StorageErrorKind::NotFound
    );
    fs.set_children(
        &root.reference,
        vec![b.reference.clone(), c.reference.clone()],
    );
    assert_eq!(fs.children(&root.reference).unwrap(), vec![b, c]);
}
#[test]
fn fake_saf_keeps_opaque_ids_duplicate_names_unknown_size_and_revocation() {
    let mut fs = MemoryStorage::default();
    let make = |id: &str| StorageRef::AndroidDocument {
        tree_uri: "content://provider/tree/grant".into(),
        authority: "provider".into(),
        document_id: id.into(),
    };
    let mut root = entry("r", None, "root", EntryKind::Directory);
    root.reference = make("tree:/../opaque");
    let mut a = entry("a", Some("r"), "same.mp3", EntryKind::File);
    a.reference = make("not/a/path%2f1");
    a.identity = Some(ObjectIdentity::Android {
        authority: "provider".into(),
        document_id: "not/a/path%2f1".into(),
    });
    let mut b = a.clone();
    b.reference = make("not/a/path%2f2");
    b.identity = Some(ObjectIdentity::Android {
        authority: "provider".into(),
        document_id: "not/a/path%2f2".into(),
    });
    for e in [&root, &a, &b] {
        fs.insert(e.clone(), vec![7]);
    }
    fs.set_children(
        &root.reference,
        vec![a.reference.clone(), b.reference.clone()],
    );
    let children = fs.children(&root.reference).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].size, None);
    assert_ne!(children[0].identity, children[1].identity);
    fs.fail(&b.reference, StorageErrorKind::RevokedGrant);
    assert_eq!(
        fs.stat(&b.reference).unwrap_err().kind,
        StorageErrorKind::RevokedGrant
    );
    assert_eq!(
        fs.open_read(&b.reference).err().unwrap().kind,
        StorageErrorKind::RevokedGrant
    );
    assert_eq!(
        fs.children(&root.reference).unwrap_err().kind,
        StorageErrorKind::RevokedGrant
    );
}
#[test]
fn utf16_is_lossless_and_never_replaced_by_display_text() {
    let units = vec![67, 58, 92, 0x05e9, 0xd801, 0xdc00, 0xd800];
    let r = StorageRef::WindowsPath {
        utf16: units.clone(),
    };
    assert_eq!(r, StorageRef::WindowsPath { utf16: units });
    assert_ne!(
        r,
        StorageRef::WindowsPath {
            utf16: "C:\\ש𐐀�".encode_utf16().collect()
        }
    );
}
#[test]
fn identity_relationships_use_parent_ids_not_names_and_fail_closed() {
    let root = entry("root", None, "root", EntryKind::Directory);
    let child = entry("child", Some("root"), "../opaque", EntryKind::Directory);
    let grandchild = entry("grandchild", Some("child"), "root", EntryKind::File);
    let mut alias = grandchild.clone();
    alias.reference = test_ref("hardlink");
    alias.name = "OTHER.MP3".into();
    let observations = vec![
        root.clone(),
        child.clone(),
        grandchild.clone(),
        alias.clone(),
    ];
    assert_eq!(
        relationship(&grandchild, &alias, &observations),
        Relationship::Same
    );
    assert_eq!(
        relationship(&root, &grandchild, &observations),
        Relationship::Ancestor
    );
    assert_eq!(
        relationship(&grandchild, &root, &observations),
        Relationship::Descendant
    );
    assert_eq!(relationship(&root, &grandchild, &[]), Relationship::Unknown);
    alias.identity = None;
    assert_eq!(
        relationship(&grandchild, &alias, &observations),
        Relationship::Unknown
    );
}
#[test]
fn missing_locked_reparse_and_unknown_capabilities_are_explicit() {
    let mut fs = MemoryStorage::default();
    assert_eq!(
        fs.stat(&test_ref("missing")).unwrap_err().kind,
        StorageErrorKind::NotFound
    );
    let mut e = entry("locked", None, "locked", EntryKind::File);
    fs.insert(e.clone(), vec![]);
    fs.fail(&e.reference, StorageErrorKind::Locked);
    assert_eq!(
        fs.open_read(&e.reference).err().unwrap().kind,
        StorageErrorKind::Locked
    );
    e = entry("link", None, "link", EntryKind::LinkOrReparse);
    fs.insert(e.clone(), vec![]);
    assert_eq!(
        fs.open_read(&e.reference).err().unwrap().kind,
        StorageErrorKind::UnsafeLink
    );
    assert!(e.apply_blockers().contains(&ApplyBlocker::LinkOrReparse));
    e.kind = EntryKind::File;
    e.identity = None;
    e.capabilities.read = None;
    assert!(e.apply_blockers().contains(&ApplyBlocker::MissingIdentity));
    assert!(
        e.apply_blockers()
            .contains(&ApplyBlocker::UnknownCapability)
    );
    assert!(
        e.apply_blockers()
            .contains(&ApplyBlocker::PhysicalApplyUnavailable)
    );
}
#[test]
fn seekable_reads_cannot_escape_observed_length() {
    let mut input = ReadInput::seekable(std::io::Cursor::new(vec![1, 2, 3, 4, 5]), 3);
    let mut bytes = vec![];
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, vec![1, 2, 3]);
    assert!(input.seek(SeekFrom::Start(4)).is_err());
    assert!(input.seek(SeekFrom::Current(-4)).is_err());
    input.seek(SeekFrom::End(-1)).unwrap();
    bytes.clear();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, vec![3]);
}

#[cfg(any(windows, target_os = "linux"))]
mod desktop_tests {
    use super::*;
    use crate::support::*;
    use std::fs;
    #[test]
    fn desktop_observes_unicode_hardlinks_and_bounded_reads() {
        let tree = synthetic_tree();
        let root = tree.path().join("source");
        fs::create_dir(&root).unwrap();
        let song = root.join("שיר𐐀.mp3");
        fs::write(&song, b"synthetic audio").unwrap();
        let alias = root.join("Other.MP3");
        fs::hard_link(&song, &alias).unwrap();
        let mut storage = TestDesktop::new(std::slice::from_ref(&root)).unwrap();
        let r = reference(&mut storage, &root);
        let s = reference(&mut storage, &song);
        let a = reference(&mut storage, &alias);
        let observed = storage.stat(&s).unwrap();
        let other = storage.stat(&a).unwrap();
        assert_eq!(observed.name, "שיר𐐀.mp3");
        assert_eq!(observed.identity, other.identity);
        assert_eq!(observed.parent_identity, storage.stat(&r).unwrap().identity);
        assert_eq!(relationship(&observed, &other, &[]), Relationship::Same);
        let raw_order: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        let children = storage.children(&r).unwrap();
        assert_eq!(
            children.iter().map(|e| e.name.clone()).collect::<Vec<_>>(),
            raw_order
        );
        let mut input = storage.open_read(&s).unwrap();
        let mut bytes = vec![];
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"synthetic audio");
        drop(input);
        fs::remove_file(&song).unwrap();
        assert_eq!(
            storage.stat(&s).unwrap_err().kind,
            StorageErrorKind::NotFound
        );
        assert_eq!(storage.children(&r).unwrap().len(), 1);
        let outside = tree.path().join("outside");
        fs::write(&outside, b"private synthetic").unwrap();
        assert_eq!(
            storage.reference(&outside).unwrap_err().kind,
            StorageErrorKind::OutsideRoots
        );
    }
    #[test]
    fn desktop_root_equality_and_ancestry_are_handle_observations() {
        let tree = synthetic_tree();
        let source = tree.path().join("source");
        let target = source.join("target");
        fs::create_dir_all(&target).unwrap();
        let mut storage = TestDesktop::new(&[source.clone(), target.clone()]).unwrap();
        let s = reference(&mut storage, &source);
        let t = reference(&mut storage, &target);
        let source = storage.stat(&s).unwrap();
        let target = storage.stat(&t).unwrap();
        assert_eq!(relationship(&source, &source, &[]), Relationship::Same);
        assert_eq!(relationship(&source, &target, &[]), Relationship::Ancestor);
        assert_eq!(storage.children(&t).unwrap(), vec![]);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_is_explicitly_test_only_and_preserves_case_collisions() {
        let tree = synthetic_tree();
        fs::write(tree.path().join("A.mp3"), b"a").unwrap();
        fs::write(tree.path().join("a.mp3"), b"b").unwrap();
        let mut storage = TestDesktop::new(&[tree.path().into()]).unwrap();
        let r = reference(&mut storage, tree.path());
        let entries = storage.children(&r).unwrap();
        assert_eq!(entries.len(), 2);
        assert_ne!(entries[0].identity, entries[1].identity);
        assert!(entries.iter().all(|e| matches!(&e.identity, Some(ObjectIdentity::Test { id }) if id.starts_with("linux-test:"))));
        assert!(
            entries
                .iter()
                .all(|e| matches!(&e.reference, StorageRef::TestOpaque { .. }))
        );
        assert_eq!(
            storage
                .stat(&StorageRef::TestOpaque {
                    id: tree.path().display().to_string()
                })
                .unwrap_err()
                .kind,
            StorageErrorKind::UnsupportedReference
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_links_are_observed_but_never_read_or_traversed() {
        use std::os::unix::fs::symlink;
        let tree = synthetic_tree();
        let source = tree.path().join("source");
        let outside = tree.path().join("outside");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret"), b"outside").unwrap();
        symlink(&outside, source.join("link")).unwrap();
        let mut storage = TestDesktop::new(std::slice::from_ref(&source)).unwrap();
        let r = reference(&mut storage, &source);
        let link = storage.children(&r).unwrap().remove(0);
        assert_eq!(link.kind, EntryKind::LinkOrReparse);
        assert_eq!(
            storage.open_read(&link.reference).err().unwrap().kind,
            StorageErrorKind::UnsafeLink
        );
        assert_eq!(
            storage.children(&link.reference).unwrap_err().kind,
            StorageErrorKind::UnsafeLink
        );
        assert_eq!(
            storage
                .reference(&source.join("link/secret"))
                .unwrap_err()
                .kind,
            StorageErrorKind::UnsafeLink
        );
        assert_eq!(
            TestDesktop::new(&[source.join("link")]).err().unwrap().kind,
            StorageErrorKind::UnsafeLink
        );
    }
    #[cfg(windows)]
    #[test]
    fn windows_code_units_and_sharing_violation() {
        use std::os::windows::{ffi::OsStringExt, fs::OpenOptionsExt};
        let tree = synthetic_tree();
        let name = std::ffi::OsString::from_wide(&[0x05e9, 0xd800, 46, 109, 112, 51]);
        let path = tree.path().join(name);
        fs::write(&path, b"synthetic").unwrap();
        let mut storage = TestDesktop::new(&[tree.path().into()]).unwrap();
        let r = reference(&mut storage, &path);
        let StorageRef::WindowsPath { utf16 } = &r else {
            panic!("not a wide reference")
        };
        assert!(utf16.contains(&0xd800));
        assert!(matches!(
            storage.stat(&r).unwrap().identity,
            Some(ObjectIdentity::Windows { .. })
        ));
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert_eq!(
            storage.open_read(&r).err().unwrap().kind,
            StorageErrorKind::Locked
        );
        drop(lock);
        assert!(storage.open_read(&r).is_ok());
    }
    #[cfg(windows)]
    #[test]
    fn windows_junction_is_observed_but_not_followed() {
        let tree = synthetic_tree();
        let source = tree.path().join("source");
        let outside = tree.path().join("outside");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret"), b"synthetic").unwrap();
        let link = source.join("junction");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .status()
            .unwrap();
        assert!(status.success(), "junction fixture creation failed");
        let mut storage = TestDesktop::new(std::slice::from_ref(&source)).unwrap();
        let r = reference(&mut storage, &source);
        let observation = storage.children(&r).unwrap().remove(0);
        assert_eq!(observation.kind, EntryKind::LinkOrReparse);
        assert_eq!(
            storage.children(&observation.reference).unwrap_err().kind,
            StorageErrorKind::UnsafeLink
        );
        assert_eq!(
            storage
                .open_read(&observation.reference)
                .err()
                .unwrap()
                .kind,
            StorageErrorKind::UnsafeLink
        );
        assert_eq!(
            storage.reference(&link.join("secret")).unwrap_err().kind,
            StorageErrorKind::UnsafeLink
        );
        drop(storage);
        fs::remove_dir(&link).unwrap();
    }
}

#[test]
fn bounded_seekable_initial_offset_is_reset_and_truncation_is_an_error() {
    let mut cursor = std::io::Cursor::new(vec![1, 2, 3, 4, 5]);
    cursor.set_position(4);
    let mut input = ReadInput::seekable(cursor, 3);
    let mut bytes = vec![];
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, vec![1, 2, 3]);
    let mut input = ReadInput::seekable(std::io::Cursor::new(vec![1]), 3);
    let mut bytes = vec![];
    assert_eq!(
        input.read_to_end(&mut bytes).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
#[test]
fn provider_capabilities_distinguish_denied_unknown_and_streaming() {
    let mut fs = MemoryStorage::default();
    let mut root = entry("r", None, "root", EntryKind::Directory);
    root.capabilities.list = Some(false);
    fs.insert(root.clone(), vec![]);
    assert_eq!(
        fs.children(&root.reference).unwrap_err().kind,
        StorageErrorKind::PermissionDenied
    );
    let mut song = entry("s", Some("r"), "song", EntryKind::File);
    song.capabilities.read = None;
    song.capabilities.seek = Some(false);
    fs.insert(song.clone(), vec![8]);
    let mut input = fs.open_read(&song.reference).unwrap();
    assert!(!input.is_seekable());
    assert!(input.seek(SeekFrom::Start(0)).is_err());
    let mut bytes = vec![];
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, vec![8]);
}
#[test]
fn stream_overflow_is_sticky_instead_of_later_becoming_successful_eof() {
    let mut input = ReadInput::streaming(std::io::Cursor::new(vec![1, 2]), 1);
    let mut buffer = [0; 2];
    assert_eq!(input.read(&mut buffer).unwrap(), 1);
    assert!(input.read(&mut buffer).is_err());
    assert!(input.read(&mut buffer).is_err());
}

#[cfg(windows)]
#[test]
fn windows_case_aliases_use_file_identity_and_invalid_references_fail() {
    use crate::support::*;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    let tree = synthetic_tree();
    let root = tree.path().join("SelectedRoot");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("SONG.MP3"), b"synthetic").unwrap();
    let mut storage = TestDesktop::new(std::slice::from_ref(&root)).unwrap();
    let a = reference(&mut storage, &root.join("SONG.MP3"));
    let b = reference(&mut storage, &root.join("song.mp3"));
    assert_ne!(a, b);
    assert_eq!(
        storage.stat(&a).unwrap().identity,
        storage.stat(&b).unwrap().identity
    );
    let case_root = tree.path().join("selectedroot");
    assert_eq!(
        sorter_io::desktop::ScratchSpace::new(&case_root, std::slice::from_ref(&root))
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::OutsideRoots
    );
    let nul = StorageRef::WindowsPath {
        utf16: vec![67, 58, 92, 0, 120],
    };
    assert_eq!(
        storage.stat(&nul).unwrap_err().kind,
        StorageErrorKind::InvalidReference
    );
    let ads = StorageRef::WindowsPath {
        utf16: root
            .join("SONG.MP3:hidden")
            .as_os_str()
            .encode_wide()
            .collect(),
    };
    assert_eq!(
        storage.open_read(&ads).err().unwrap().kind,
        StorageErrorKind::InvalidReference
    );
}

#[test]
fn streaming_spool_capability_never_promotes_test_support_to_product_support() {
    #[cfg(target_os = "linux")]
    assert_eq!(
        streaming_spool_capability(),
        StreamingSpoolCapability::LinuxTestOnly
    );
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        streaming_spool_capability(),
        StreamingSpoolCapability::Unsupported
    );
}
