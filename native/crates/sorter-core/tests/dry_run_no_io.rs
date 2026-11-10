use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use sorter_core::*;

struct SentinelTree(PathBuf);
impl Drop for SentinelTree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

// The test owns every file operation; the production planner receives only DTOs.
fn manifest(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                entries.insert(path.strip_prefix(root).unwrap().into(), None);
                visit(root, &path, entries);
            } else {
                entries.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(fs::read(&path).unwrap()),
                );
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

#[test]
fn planning_and_binding_leave_source_target_sentinel_manifest_byte_identical() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("singles-native-sentinel-{}-{nonce}", std::process::id());
    let tree = SentinelTree(std::env::temp_dir().join(name));
    fs::create_dir_all(tree.0.join("source/child")).unwrap();
    fs::create_dir_all(tree.0.join("target/אמן/סינגלים")).unwrap();
    fs::write(
        tree.0.join("source/אמן.mp3"),
        b"not audio: disposable source\0\xff",
    )
    .unwrap();
    fs::write(tree.0.join("source/child/keep.txt"), b"child sentinel").unwrap();
    fs::write(
        tree.0.join("target/אמן/סינגלים/אמן.mp3"),
        b"destination sentinel",
    )
    .unwrap();
    fs::write(tree.0.join("target/.hidden"), b"target hidden sentinel").unwrap();
    let before = manifest(&tree.0);
    let hashes_before: BTreeMap<_, _> = before
        .iter()
        .map(|(path, bytes)| {
            (
                path,
                bytes.as_ref().map(|b| format!("{:x}", Sha256::digest(b))),
            )
        })
        .collect();
    let mut request = PlanRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings::default(),
            aliases: vec![Alias {
                source: "אמן".into(),
                target: "אמן".into(),
            }],
            recognition: RecognitionFacts::Disabled,
        },
        songs: vec![SongFacts {
            source_id: tree.0.join("source/אמן.mp3").to_string_lossy().into_owned(),
            filename: "אמן.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: "".into(),
                title: "".into(),
            },
        }],
        target: TargetSnapshot {
            root_id: tree.0.join("target").to_string_lossy().into_owned(),
            existing_directories: vec![vec!["אמן".into(), "סינגלים".into()]],
            existing_files: vec![vec!["אמן".into(), "סינגלים".into(), "אמן.mp3".into()]],
        },
    };
    for copy_mode in [false, true] {
        request.context.settings.copy_mode = copy_mode;
        let result = plan(&request).unwrap();
        assert_eq!(
            result.items[0].destinations[0].components.last().unwrap(),
            "אמן_1.mp3"
        );
        verify_plan_binding(&result, &request).unwrap();
        assert!(!result.executable);
        assert_eq!(result.provenance, Provenance::ProvidedFacts);
        let mut tampered = result;
        tampered.executable = true;
        assert_eq!(
            verify_plan_binding(&tampered, &request).unwrap_err().code,
            ErrorCode::StalePlan
        );
    }
    let after = manifest(&tree.0);
    assert_eq!(before, after);
    let hashes_after: BTreeMap<_, _> = after
        .iter()
        .map(|(path, bytes)| {
            (
                path,
                bytes.as_ref().map(|b| format!("{:x}", Sha256::digest(b))),
            )
        })
        .collect();
    assert_eq!(hashes_before, hashes_after);
}

#[test]
fn planner_interface_accepts_only_supplied_dto_not_file_opening_adapter() {
    let _: fn(&PlanRequest) -> Result<Plan, EngineError> = plan;
    let _: fn(&Plan, &PlanRequest) -> Result<(), EngineError> = verify_plan_binding;
}
