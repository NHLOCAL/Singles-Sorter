//! Real selected-root album execution uses only generated, owned fixtures.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, binding::*, digest, results::*, windows_album::*,
    windows_copy::WindowsPlanningStorage,
};
use sorter_io::desktop::{DesktopStorage, WindowsDirectoryReadLimits, windows_mutation::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
struct Metadata;
struct Fields(String);
impl MetadataProvider for Metadata {
    fn open(&mut self, entry: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        Ok(Box::new(Fields(entry.name.clone())))
    }
}
impl MetadataFields for Fields {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        let value = match field {
            LegacyField::Artist | LegacyField::AlbumArtist => {
                LegacyScalar::Text("שלום עולם".into())
            }
            LegacyField::Album => LegacyScalar::Text(
                if self.0.contains("second") {
                    "אלבום שני"
                } else {
                    "אלבום"
                }
                .into(),
            ),
            LegacyField::TrackNumber => LegacyScalar::Integer(
                self.0
                    .chars()
                    .next()
                    .filter(char::is_ascii_digit)
                    .unwrap_or('1')
                    .to_string(),
            ),
            _ => LegacyScalar::Text(String::new()),
        };
        Ok(LegacyItem {
            values: vec![value.clone()],
            truthy: !matches!(&value, LegacyScalar::Text(s) if s.is_empty()),
            value: Ok(value),
        })
    }
}
struct Fixture {
    _tree: tempfile::TempDir,
    source: PathBuf,
    album: PathBuf,
    target: PathBuf,
    journal: PathBuf,
    plan: JobPlanV2,
    scope: AlbumScope,
}
impl Fixture {
    fn new(copy: bool, root_album: bool) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("source");
        let target = tree.path().join("target");
        let journal = tree.path().join("journal");
        for path in [&source, &target, &journal] {
            std::fs::create_dir(path).unwrap();
        }
        let album = if root_album {
            source.clone()
        } else {
            let p = source.join("input-album");
            std::fs::create_dir(&p).unwrap();
            p
        };
        for n in 1..=3 {
            std::fs::write(
                album.join(format!("{n}-שיר.mp3")),
                format!("owned audio {n}"),
            )
            .unwrap();
        }
        std::fs::write(album.join("cover.jpg"), b"owned sidecar").unwrap();
        std::fs::create_dir(album.join("booklet")).unwrap();
        std::fs::create_dir(album.join("booklet/nested")).unwrap();
        std::fs::write(album.join("booklet/nested/readme.txt"), b"nested bytes").unwrap();
        let scan_cancel = AtomicBool::new(false);
        let mut reads = WindowsPlanningStorage::for_album_preview(
            &source,
            &target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &scan_cancel,
        )
        .unwrap();
        let request = JobRequestV2 {
            mode: JobMode::Sort,
            source: reads.reference(&source).unwrap(),
            target: Some(reads.reference(&target).unwrap()),
            settings: Settings {
                copy_mode: copy,
                main_folder_only: !root_album,
                singles_folder: false,
                ..Settings::default()
            },
            alias_rows: vec![vec!["שלום עולם".into(), "שם ממופה".into()]],
            resource_ids: vec!["fixture".into()],
            adapter_versions: vec!["fixture".into()],
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(
            &request,
            &mut reads,
            &mut Metadata,
            &mut DisabledRecognition,
        )
        .unwrap();
        assert_eq!(plan.operations.len(), 1, "{plan:?}");
        assert!(matches!(
            plan.operations[0].kind,
            OperationKind::AlbumTransfer { .. }
        ));
        // Frozen v14 handle_album_transfer counts direct files, including cover.jpg,
        // under the analyzed artist. Albums never increment songs_sorted.
        assert_eq!(plan.predicted_legacy_counters.songs_sorted, 0);
        assert_eq!(plan.predicted_legacy_counters.albums_processed, 1);
        assert_eq!(
            plan.predicted_legacy_counters.top_artists,
            vec![("שלום עולם".into(), 4)]
        );
        let resources = Resources {
            resource_ids: request.resource_ids,
            adapter_versions: request.adapter_versions,
            ai_profile: AiProfile::Disabled,
            resource_hashes: vec![digest(b"fixture")],
            adapter_hashes: vec![digest(b"fixture")],
            model_hash: None,
        };
        let scope =
            prepare_album_scope(&plan, &mut reads, &resources, &AtomicBool::new(false)).unwrap();
        Self {
            _tree: tree,
            source,
            album,
            target,
            journal,
            plan,
            scope,
        }
    }
    fn grants(
        &self,
    ) -> (
        WindowsSelectedRootGrant,
        WindowsSelectedRootGrant,
        WindowsSelectedRootGrant,
    ) {
        let mut reads = DesktopStorage::new(&[
            self.source.clone(),
            self.target.clone(),
            self.journal.clone(),
        ])
        .unwrap();
        let mut grant = |p: &PathBuf| {
            let r = reads.reference(p).unwrap();
            WindowsSelectedRootGrant::new(p.clone(), reads.stat(&r).unwrap().identity.unwrap())
                .unwrap()
        };
        (
            grant(&self.source),
            grant(&self.target),
            grant(&self.journal),
        )
    }
    fn prepare(&self) -> sorter_executor::Result<SelectedWindowsAlbumJob> {
        let validated = validate_album_execution(
            &self.plan,
            &mut Metadata,
            &mut DisabledRecognition,
            self.scope.resources(),
            &self.scope,
            &AtomicBool::new(false),
        )?;
        let (source, target, journal) = self.grants();
        SelectedWindowsAlbumJob::prepare(
            source,
            target,
            journal,
            validated,
            &AtomicBool::new(false),
        )
    }
    fn job(&self) -> SelectedWindowsAlbumJob {
        self.prepare().unwrap()
    }
    fn replan(&mut self) {
        let scan_cancel = AtomicBool::new(false);
        let resources = self.scope.resources().clone();
        let mut reads = WindowsPlanningStorage::for_album_preview(
            &self.source,
            &self.target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &scan_cancel,
        )
        .unwrap();
        self.plan = plan_job(
            &self.plan.request,
            &mut reads,
            &mut Metadata,
            &mut DisabledRecognition,
        )
        .unwrap();
        self.scope =
            prepare_album_scope(&self.plan, &mut reads, &resources, &AtomicBool::new(false))
                .unwrap();
    }
    fn destination(&self) -> PathBuf {
        self.target.join("שם ממופה/אלבום")
    }
    fn verify_copy(&self) {
        for n in 1..=3 {
            assert_eq!(
                std::fs::read(self.destination().join(format!("{n}-שיר.mp3"))).unwrap(),
                format!("owned audio {n}").as_bytes()
            );
        }
        assert_eq!(
            std::fs::read(self.destination().join("cover.jpg")).unwrap(),
            b"owned sidecar"
        );
        assert_eq!(
            std::fs::read(self.destination().join("booklet/nested/readme.txt")).unwrap(),
            b"nested bytes"
        );
        assert!(self.album.join("1-שיר.mp3").exists());
    }
}
#[test]
fn album_copy_nested_sidecar_and_original_counter_rules() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    let mut ids = vec![];
    let result = job
        .run(&AtomicBool::new(false), &mut |p| ids.push(p.operation))
        .unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
    assert_eq!(result.transfers.as_ref().unwrap().committed_destinations, 5);
    assert!(ids.iter().all(|id| *id == 0));
    drop(job);
    f.verify_copy();
}
#[test]
fn selected_source_root_album_copy() {
    let f = Fixture::new(true, true);
    let mut job = f.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    f.verify_copy();
}
#[test]
fn album_pre_cancel_and_second_attempt_have_no_effect() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    assert!(matches!(
        job.run(&AtomicBool::new(true), &mut |_| {}),
        Err(E::Cancelled)
    ));
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::NeedsRecovery)
    ));
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
}
#[test]
fn album_late_child_before_run_stops_before_destination_creation() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    std::fs::write(f.album.join("late.txt"), b"preserve late child").unwrap();
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::Stale)
    ));
    assert!(!f.destination().exists());
}
#[test]
fn album_removed_child_before_run_stops_before_effects() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    std::fs::remove_file(f.album.join("cover.jpg")).unwrap();
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::Stale)
    ));
    assert!(!f.destination().exists());
}
#[test]
fn album_content_change_after_preview_is_stale() {
    let f = Fixture::new(true, false);
    std::fs::write(f.album.join("1-שיר.mp3"), b"changed bytes").unwrap();
    assert!(matches!(f.prepare(), Err(E::Stale)));
    assert!(!f.destination().exists());
}
#[test]
fn album_foreign_destination_is_never_merged_or_overwritten() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    std::fs::create_dir_all(f.destination()).unwrap();
    std::fs::write(f.destination().join("foreign"), b"foreign bytes").unwrap();
    assert_ne!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    assert_eq!(
        std::fs::read(f.destination().join("foreign")).unwrap(),
        b"foreign bytes"
    );
    assert!(!f.destination().join("1-שיר.mp3").exists());
}
#[test]
fn album_cancel_during_copy_retains_every_original() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |_| cancel.store(true, Ordering::Release))
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.albums_completed, 0);
    drop(job);
    for n in 1..=3 {
        assert!(f.album.join(format!("{n}-שיר.mp3")).exists());
    }
}
#[test]
fn album_late_child_during_staging_prevents_complete_receipt() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    let mut inserted = false;
    let result = job
        .run(&AtomicBool::new(false), &mut |_| {
            if !inserted {
                std::fs::write(f.album.join("late.txt"), b"late child").unwrap();
                inserted = true;
            }
        })
        .unwrap();
    assert!(inserted);
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 0);
    drop(job);
    assert_eq!(
        std::fs::read(f.album.join("late.txt")).unwrap(),
        b"late child"
    );
}
#[test]
fn album_copy_recovery_is_read_only_and_tracks_each_nested_file() {
    let f = Fixture::new(true, false);
    let mut job = f.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    let before = std::fs::read_dir(&f.journal).unwrap().count();
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(report.manifest_finished);
    assert!(!report.incomplete_tail);
    assert!(report.pending_directories.is_empty());
    let transfers = report.transfers.unwrap();
    assert_eq!(transfers.operations.len(), 5);
    assert!(
        transfers.operations.iter().all(|o| o.operation == 0
            && o.status == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), before);
}

#[test]
fn whole_album_move_preserves_all_objects_and_removes_original_container_path() {
    let f = Fixture::new(false, false);
    let mut reads = DesktopStorage::new(&[f.source.clone(), f.target.clone()]).unwrap();
    let original_ref = reads.reference(&f.album).unwrap();
    let original = reads.stat(&original_ref).unwrap();
    let mut job = f.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.whole_albums.len(), 1);
    assert_eq!(result.effect_receipts.len(), 1);
    assert_eq!(result.effect_receipts[0].kind, "album_renamed");
    assert_eq!(result.effect_receipts[0].operation, 0);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
    assert_eq!(
        result.whole_albums[0].root.as_ref().map(|o| &o.identity),
        original.identity.as_ref()
    );
    assert_eq!(result.whole_albums[0].descendants.len(), 7);
    assert!(result.transfers.is_none());
    drop(job);
    assert!(!f.album.exists());
    assert_eq!(
        std::fs::read(f.destination().join("booklet/nested/readme.txt")).unwrap(),
        b"nested bytes"
    );
    assert_eq!(
        std::fs::read(f.destination().join("cover.jpg")).unwrap(),
        b"owned sidecar"
    );
}
#[test]
fn selected_source_root_album_move_uses_dedicated_scope() {
    let f = Fixture::new(false, true);
    let mut job = f.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 1);
    assert!(result.retained_directories.is_empty());
    drop(job);
    assert!(!f.source.exists());
    assert!(f.destination().join("1-שיר.mp3").exists());
    assert!(std::fs::read_dir(&f.journal).unwrap().count() > 0);
}
#[test]
fn whole_album_late_destination_collision_preserves_original_tree() {
    let f = Fixture::new(false, false);
    let mut job = f.job();
    std::fs::create_dir_all(f.destination()).unwrap();
    std::fs::write(f.destination().join("foreign"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 0);
    drop(job);
    assert!(f.album.join("1-שיר.mp3").exists());
    assert_eq!(
        std::fs::read(f.destination().join("foreign")).unwrap(),
        b"foreign"
    );
}
#[test]
fn moved_root_album_recovery_reads_missing_original_without_ancestor_grant() {
    let f = Fixture::new(false, true);
    let mut job = f.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    let whole = report.whole.unwrap();
    assert!(whole.recorded_finished);
    assert!(whole.pending_albums.is_empty());
    assert!(matches!(
        whole.source_evidence[0].status,
        AlbumDirectoryEvidenceStatus::Absent
    ));
    assert!(matches!(
        whole.destination_evidence[0].status,
        AlbumDirectoryEvidenceStatus::SameIdentity
    ));
    assert_eq!(whole.files.len(), 5);
    assert!(whole.files.iter().all(|file| file.status == "verified"));
}
#[test]
fn whole_album_cancel_before_dispatch_preserves_original_root() {
    let f = Fixture::new(false, true);
    let mut job = f.job();
    assert!(matches!(
        job.run(&AtomicBool::new(true), &mut |_| {}),
        Err(E::Cancelled)
    ));
    assert!(f.source.exists());
    assert!(!f.destination().exists());
}
#[test]
fn real_metadata_root_album_validation_releases_all_read_pins_before_move() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    fn sync(size: usize) -> [u8; 4] {
        [
            ((size >> 21) & 127) as u8,
            ((size >> 14) & 127) as u8,
            ((size >> 7) & 127) as u8,
            (size & 127) as u8,
        ]
    }
    fn audio(track: usize) -> Vec<u8> {
        let mut body = vec![];
        for (id, value) in [
            (b"TPE1", "שלום עולם".to_owned()),
            (b"TPE2", "שלום עולם".to_owned()),
            (b"TALB", "אלבום".to_owned()),
            (b"TRCK", track.to_string()),
        ] {
            let mut payload = vec![3];
            payload.extend(value.as_bytes());
            body.extend(id);
            body.extend(sync(payload.len()));
            body.extend([0, 0]);
            body.extend(payload);
        }
        let mut file = b"ID3\x04\x00\x00".to_vec();
        file.extend(sync(body.len()));
        file.extend(body);
        file.extend(include_bytes!("../../../tests/fixtures/media/blank.mp3"));
        file
    }
    let f = Fixture::new(false, true);
    for track in 1..=3 {
        std::fs::write(f.source.join(format!("{track}-שיר.mp3")), audio(track)).unwrap();
    }
    let resources = f.scope.resources().clone();
    let validated = {
        let scan_cancel = AtomicBool::new(false);
        let mut reads = WindowsPlanningStorage::for_album_preview(
            &f.source,
            &f.target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &scan_cancel,
        )
        .unwrap();
        let mut metadata_reads = WindowsPlanningStorage::for_album_preview(
            &f.source,
            &f.target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &scan_cancel,
        )
        .unwrap();
        let mut metadata = StorageMetadataProvider {
            storage: &mut metadata_reads,
            reader: MetadataReader,
        };
        let plan = plan_job(
            &f.plan.request,
            &mut reads,
            &mut metadata,
            &mut DisabledRecognition,
        )
        .unwrap();
        assert_eq!(plan.operations.len(), 1);
        let scope =
            prepare_album_scope(&plan, &mut reads, &resources, &AtomicBool::new(false)).unwrap();
        validate_album_execution(
            &plan,
            &mut metadata,
            &mut DisabledRecognition,
            &resources,
            &scope,
            &AtomicBool::new(false),
        )
        .unwrap()
        // Both independent root-pinning read adapters and metadata provider drop
        // at this block boundary. The returned value has no native authority.
    };
    let (source, target, journal) = f.grants();
    let mut job = SelectedWindowsAlbumJob::prepare(
        source,
        target,
        journal,
        validated,
        &AtomicBool::new(false),
    )
    .unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
    let recovery = job
        .inspect_recovery(&AtomicBool::new(false))
        .unwrap()
        .whole
        .unwrap();
    assert!(recovery.files.iter().all(|file| file.status == "verified"));
    drop(job);
    assert!(!f.source.exists());
}

#[test]
fn second_album_content_change_is_child_failure_not_root_evidence() {
    let mut f = Fixture::new(false, false);
    let second = f.source.join("second-album");
    std::fs::create_dir(&second).unwrap();
    for n in 1..=3 {
        std::fs::write(
            second.join(format!("{n}-second.mp3")),
            format!("second audio {n}"),
        )
        .unwrap();
    }
    f.replan();
    assert_eq!(f.plan.operations.len(), 2);
    let mut job = f.job();
    let mut changed = false;
    let result = job
        .run(&AtomicBool::new(false), &mut |_| {
            if !changed {
                std::fs::write(second.join("1-second.mp3"), b"external changed bytes").unwrap();
                changed = true;
            }
        })
        .unwrap();
    assert!(changed);
    assert_eq!(result.status, JobStatus::Partial);
    assert_eq!(result.albums_completed, 1);
    assert!(result.whole_albums[1].root.is_none());
    assert!(
        result.whole_albums[1]
            .failure
            .as_ref()
            .unwrap()
            .observed
            .is_some()
    );
    drop(job);
    assert_eq!(
        std::fs::read(second.join("1-second.mp3")).unwrap(),
        b"external changed bytes"
    );
}
#[test]
fn album_recovery_rejects_rehashed_directory_and_terminal_substitution() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Body {
        sequence: u64,
        previous: String,
        job_id: String,
        phase: String,
        payload: serde_json::Value,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Envelope {
        body: Body,
        sha256: String,
    }
    fn rewrite(bytes: &[u8], mut edit: impl FnMut(&mut Body)) -> Vec<u8> {
        let mut previous = String::new();
        let mut result = vec![];
        for line in bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
            edit(&mut envelope.body);
            envelope.body.previous = previous;
            envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
            previous = envelope.sha256.clone();
            result.extend(serde_json::to_vec(&envelope).unwrap());
            result.push(b'\n');
        }
        result
    }
    for copy in [true, false] {
        let f = Fixture::new(copy, false);
        let mut job = f.job();
        job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        let path = std::fs::read_dir(&f.journal)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(if copy {
                        ".album.jsonl"
                    } else {
                        ".album-move.jsonl"
                    })
            })
            .unwrap();
        let original = std::fs::read(&path).unwrap();
        let forged = rewrite(&original, |body| {
            if body.phase == "Finished" {
                body.payload["albums_completed"] = serde_json::json!(999);
            }
        });
        std::fs::write(&path, forged).unwrap();
        assert!(matches!(
            job.inspect_recovery(&AtomicBool::new(false)),
            Err(E::CorruptJournal)
        ));
        let id = serde_json::to_value(f.scope.execution_scope().source_root.clone()).unwrap();
        let forged = rewrite(&original, |body| {
            if body.phase == "DirectoryObserved" {
                body.payload["identity"] = id.clone();
            }
            if body.phase == "Finished" {
                body.payload["created"][0]["identity"] = id.clone();
            }
        });
        std::fs::write(&path, forged).unwrap();
        assert!(matches!(
            job.inspect_recovery(&AtomicBool::new(false)),
            Err(E::CorruptJournal)
        ));
        let forged = rewrite(&original, |body| {
            if body.phase == "Finished" {
                body.payload["status"] = serde_json::json!("Partial");
                body.payload["albums_completed"] = serde_json::json!(0);
            }
        });
        std::fs::write(&path, forged).unwrap();
        assert!(matches!(
            job.inspect_recovery(&AtomicBool::new(false)),
            Err(E::CorruptJournal)
        ));
        let forged = rewrite(&original, |body| {
            if body.phase == "DirectoryPrepared" {
                body.payload["parent"] = id.clone();
            }
        });
        std::fs::write(&path, forged).unwrap();
        assert!(matches!(
            job.inspect_recovery(&AtomicBool::new(false)),
            Err(E::CorruptJournal)
        ));
        if copy {
            // A missing metadata outcome cannot be hidden by rehashing a partial
            // terminal receipt; the prepared effect remains unresolved.
            let lines = original
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            let last_intent = lines
                .iter()
                .rposition(|line| {
                    serde_json::from_slice::<Envelope>(line).unwrap().body.phase
                        == "DirectoryMetadataPrepared"
                })
                .unwrap();
            let missing = lines
                .iter()
                .enumerate()
                .filter(|(index, line)| {
                    *index <= last_intent
                        || matches!(
                            serde_json::from_slice::<Envelope>(line)
                                .unwrap()
                                .body
                                .phase
                                .as_str(),
                            "TransfersObserved" | "Finished"
                        )
                })
                .flat_map(|(_, line)| line.iter().copied().chain(std::iter::once(b'\n')))
                .collect::<Vec<_>>();
            let mut sequence = 0;
            let forged = rewrite(&missing, |body| {
                body.sequence = sequence;
                sequence += 1;
                if matches!(body.phase.as_str(), "TransfersObserved" | "Finished") {
                    body.payload["status"] = serde_json::json!("Partial");
                }
                if body.phase == "Finished" {
                    body.payload["albums_completed"] = serde_json::json!(0);
                }
            });
            std::fs::write(&path, forged).unwrap();
            assert!(matches!(
                job.inspect_recovery(&AtomicBool::new(false)),
                Err(E::CorruptJournal)
            ));
            let forged = rewrite(&original, |body| {
                if body.phase == "DirectoryMetadataObserved" {
                    body.payload["evidence"]["source_identity"] = id.clone();
                }
            });
            std::fs::write(&path, forged).unwrap();
            assert!(matches!(
                job.inspect_recovery(&AtomicBool::new(false)),
                Err(E::CorruptJournal)
            ));
            // Forge a self-consistent outer partial receipt; the inner physical
            // journal still proves five commits and must reject the zero claim.
            let forged = rewrite(&original, |body| {
                if body.phase == "TransfersObserved" {
                    body.payload["status"] = serde_json::json!("Partial");
                    body.payload["committed_destinations"] = serde_json::json!(0);
                }
                if body.phase == "Finished" {
                    body.payload["status"] = serde_json::json!("Partial");
                    body.payload["albums_completed"] = serde_json::json!(0);
                }
            });
            std::fs::write(&path, forged).unwrap();
            assert!(matches!(
                job.inspect_recovery(&AtomicBool::new(false)),
                Err(E::CorruptJournal)
            ));
        }
        std::fs::write(&path, &original).unwrap();
        assert!(
            job.inspect_recovery(&AtomicBool::new(false))
                .unwrap()
                .manifest_finished
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

#[test]
fn mixed_album_then_single_move_uses_separate_real_receipts() {
    let mut f = Fixture::new(false, false);
    std::fs::write(f.source.join("loose.mp3"), b"owned loose single").unwrap();
    f.replan();
    assert_eq!(f.plan.operations.len(), 2);
    assert!(matches!(
        f.plan.operations[0].kind,
        OperationKind::AlbumTransfer { .. }
    ));
    assert!(matches!(
        f.plan.operations[1].kind,
        OperationKind::SingleTransfer { .. }
    ));
    let mut job = f.job();
    let mut order = vec![];
    let result = job
        .run(&AtomicBool::new(false), &mut |progress| {
            order.push(progress.operation)
        })
        .unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.whole_albums[0].operation, 0);
    let singles = result.transfers.as_ref().unwrap();
    assert_eq!(singles.moved_sources, 1);
    assert_eq!(singles.items[0].operation, 1);
    assert_eq!(singles.committed_operations, vec![1]);
    assert_eq!(
        result
            .effect_receipts
            .iter()
            .map(|receipt| (receipt.operation, receipt.kind.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "album_renamed"), (1, "file_renamed")]
    );
    assert_eq!(order.first(), Some(&0));
    let recovery = job
        .inspect_recovery(&AtomicBool::new(false))
        .unwrap()
        .whole
        .unwrap();
    assert!(recovery.recorded_finished);
    assert!(recovery.singles.unwrap().manifest_finished);
    drop(job);
    assert!(!f.source.join("loose.mp3").exists());
    assert_eq!(
        std::fs::read(f.target.join("שם ממופה/loose.mp3")).unwrap(),
        b"owned loose single"
    );
}
#[test]
fn cancellation_after_album_before_singles_preserves_unprocessed_single() {
    let mut f = Fixture::new(false, false);
    std::fs::write(f.source.join("loose.mp3"), b"owned loose single").unwrap();
    f.replan();
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 0 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.albums_completed, 1);
    assert!(result.transfers.is_none());
    drop(job);
    assert_eq!(
        std::fs::read(f.source.join("loose.mp3")).unwrap(),
        b"owned loose single"
    );
    assert!(!f.target.join("שם ממופה/loose.mp3").exists());
}

#[test]
fn ordered_direct_child_sanitization_preserves_nested_names_for_root_and_child_albums() {
    for root in [false, true] {
        let mut f = Fixture::new(false, root);
        std::fs::rename(f.album.join("1-שיר.mp3"), f.album.join("1-  שיר.mp3")).unwrap();
        std::fs::rename(f.album.join("booklet"), f.album.join("booklet  notes")).unwrap();
        std::fs::rename(
            f.album.join("booklet  notes/nested/readme.txt"),
            f.album.join("booklet  notes/nested/keep  spaces.txt"),
        )
        .unwrap();
        f.replan();
        let OperationKind::AlbumTransfer { children, .. } = &f.plan.operations[0].kind else {
            panic!("album required")
        };
        let expected = children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.source.name != *child.destination.components.last().unwrap())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), 2);
        let mut job = f.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, JobStatus::Complete);
        assert_eq!(result.albums_completed, 1);
        assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
        assert_eq!(
            result
                .child_renames
                .iter()
                .map(|receipt| receipt.planner_index)
                .collect::<Vec<_>>(),
            expected
        );
        let recovery = job
            .inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .whole
            .unwrap();
        assert!(recovery.recorded_finished);
        assert_eq!(recovery.child_renames.len(), 2);
        assert!(recovery.files.iter().all(|file| file.status == "verified"));
        drop(job);
        assert_eq!(
            std::fs::read(
                f.destination()
                    .join("booklet notes/nested/keep  spaces.txt")
            )
            .unwrap(),
            b"nested bytes"
        );
        assert!(f.destination().join("1- שיר.mp3").exists());
        assert!(!f.album.exists());
    }
}
#[test]
fn cancellation_after_child_sanitization_retains_whole_original_container() {
    let mut f = Fixture::new(false, true);
    std::fs::rename(f.album.join("1-שיר.mp3"), f.album.join("1-  שיר.mp3")).unwrap();
    f.replan();
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |_| cancel.store(true, Ordering::Release))
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.albums_completed, 0);
    assert_eq!(result.child_renames.len(), 1);
    let recovery = job
        .inspect_recovery(&AtomicBool::new(false))
        .unwrap()
        .whole
        .unwrap();
    assert!(recovery.recorded_finished);
    assert_eq!(recovery.child_renames.len(), 1);
    assert!(
        recovery
            .child_locations
            .iter()
            .any(|e| e.at_destination && e.observed.is_some())
    );
    drop(job);
    assert!(f.album.exists());
    assert!(f.album.join("1- שיר.mp3").exists());
    assert!(!f.destination().exists());
}
#[test]
fn late_album_collision_does_not_sanitize_original_children() {
    let mut f = Fixture::new(false, false);
    std::fs::rename(f.album.join("1-שיר.mp3"), f.album.join("1-  שיר.mp3")).unwrap();
    f.replan();
    let mut job = f.job();
    std::fs::create_dir_all(f.destination()).unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Partial);
    assert!(result.child_renames.is_empty());
    assert!(
        job.inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .manifest_finished
    );
    drop(job);
    assert!(f.album.join("1-  שיר.mp3").exists());
    assert!(!f.album.join("1- שיר.mp3").exists());
}
#[test]
fn later_copy_cancellation_preserves_the_first_completed_album_receipt() {
    let mut f = Fixture::new(true, false);
    let second = f.source.join("second-album");
    std::fs::create_dir(&second).unwrap();
    for n in 1..=3 {
        std::fs::write(
            second.join(format!("{n}-second.mp3")),
            format!("second audio {n}"),
        )
        .unwrap();
    }
    f.replan();
    assert_eq!(f.plan.operations.len(), 2);
    let OperationKind::AlbumTransfer { children, .. } = &f.plan.operations[0].kind else {
        panic!("album required")
    };
    let first_files = children
        .iter()
        .filter(|child| child.source.kind == EntryKind::File)
        .count();
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 1 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), first_files)]);
    assert!(
        job.inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .manifest_finished
    );
    drop(job);
    assert!(f.album.join("1-שיר.mp3").exists());
    assert!(second.join("1-second.mp3").exists());
}
#[test]
fn copied_files_and_nested_directories_preserve_write_time_and_readonly() {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::time::{Duration, UNIX_EPOCH};
    let mut f = Fixture::new(true, false);
    let stamp = UNIX_EPOCH + Duration::from_secs(946684800);
    let file = f.album.join("cover.jpg");
    let directory = f.album.join("booklet/nested");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_accessed(stamp)
                .set_modified(stamp),
        )
        .unwrap();
    std::fs::OpenOptions::new()
        .access_mode(0x100)
        .custom_flags(0x02000000)
        .open(&directory)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_accessed(stamp)
                .set_modified(stamp),
        )
        .unwrap();
    let original_file_permissions = std::fs::metadata(&file).unwrap().permissions();
    let original_directory_permissions = std::fs::metadata(&directory).unwrap().permissions();
    for path in [&file, &directory] {
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(path, permissions).unwrap();
    }
    f.replan();
    let expected_file = std::fs::metadata(&file).unwrap().last_write_time();
    let expected_directory = std::fs::metadata(&directory).unwrap().last_write_time();
    let mut job = f.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 1);
    let recovery = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert_eq!(recovery.directory_metadata.len(), 2);
    assert!(
        recovery
            .directory_metadata_status
            .iter()
            .all(|(_, status)| status == "verified")
    );
    assert!(
        recovery
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.copy_metadata
                == sorter_executor::recovery::CopyMetadataRecoveryStatus::Verified)
    );
    drop(job);
    let copied_file = f.destination().join("cover.jpg");
    let copied_directory = f.destination().join("booklet/nested");
    assert_eq!(
        std::fs::metadata(&copied_file).unwrap().last_write_time(),
        expected_file
    );
    assert_eq!(
        std::fs::metadata(&copied_directory)
            .unwrap()
            .last_write_time(),
        expected_directory
    );
    for (path, permissions) in [
        (&file, &original_file_permissions),
        (&directory, &original_directory_permissions),
        (&copied_file, &original_file_permissions),
        (&copied_directory, &original_directory_permissions),
    ] {
        assert!(std::fs::metadata(path).unwrap().permissions().readonly());
        std::fs::set_permissions(path, permissions.clone()).unwrap();
    }
    let journal = std::fs::read_dir(&f.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".transfers.jsonl")
        })
        .unwrap();
    let bytes = std::fs::read(journal).unwrap();
    let records = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records[0]["body"]["binding"]["copy_metadata_version"], 1);
    assert_eq!(
        records
            .iter()
            .filter(|record| record["body"]["event"]["phase"] == "CopyMetadataObserved")
            .count(),
        5
    );
}
