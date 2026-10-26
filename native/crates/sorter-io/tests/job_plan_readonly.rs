#![cfg(any(windows, target_os = "linux"))]
mod support;
use sorter_core::Settings;
use sorter_core::job_plan::*;
use sorter_io::{
    metadata::{MetadataReader, StorageMetadataProvider},
    storage::*,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use support::*;
fn manifest(root: &Path) -> Vec<(PathBuf, Vec<u8>, u64, SystemTime)> {
    fn walk(root: &Path, path: &Path, rows: &mut Vec<(PathBuf, Vec<u8>, u64, SystemTime)>) {
        for e in fs::read_dir(path).unwrap() {
            let p = e.unwrap().path();
            let m = fs::symlink_metadata(&p).unwrap();
            rows.push((
                p.strip_prefix(root).unwrap().into(),
                if m.is_file() {
                    fs::read(&p).unwrap()
                } else {
                    vec![]
                },
                m.len(),
                m.modified().unwrap(),
            ));
            if m.is_dir() {
                walk(root, &p, rows);
            }
        }
    }
    let mut rows = vec![];
    walk(root, root, &mut rows);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}
#[test]
fn real_adapter_sort_and_fix_previews_do_not_change_any_tree_bytes_names_or_mtimes() {
    let temp = synthetic_tree();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(
        source.join("אברהם_פריד.mp3"),
        b"owned synthetic invalid audio; no recording",
    )
    .unwrap();
    fs::write(source.join("artwork.txt"), b"unchanged").unwrap();
    let before = manifest(temp.path());
    for mode in [JobMode::Sort, JobMode::FixNames] {
        let mut desktop = TestDesktop::new(&[source.clone(), target.clone()]).unwrap();
        let source_ref = reference(&mut desktop, &source);
        let target_ref = reference(&mut desktop, &target);
        let mut discovery = SharedReadStorage::new(desktop);
        let mut metadata_storage = discovery.clone();
        let mut metadata = StorageMetadataProvider {
            storage: &mut metadata_storage,
            reader: MetadataReader,
        };
        let request = JobRequestV2 {
            mode,
            source: source_ref,
            target: Some(target_ref),
            settings: Settings::default(),
            alias_rows: vec![vec!["אברהם פריד".into(), "אברהם פריד".into()]],
            resource_ids: vec!["synthetic-alias".into()],
            adapter_versions: vec!["native-read-only".into()],
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(
            &request,
            &mut discovery,
            &mut metadata,
            &mut DisabledRecognition,
        )
        .unwrap();
        assert!(!plan.executable);
        assert_eq!(plan.operations.len(), 1);
        assert!(!plan.metadata_observations.is_empty());
        assert_eq!(manifest(temp.path()), before);
    }
}
