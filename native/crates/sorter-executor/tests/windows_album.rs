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

fn cross_volume_album(root_album: bool) -> (Fixture, tempfile::TempDir) {
    let mut fixture = Fixture::new(false, root_album);
    let target_tree = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    fixture.target = target_tree.path().canonicalize().unwrap().join("target");
    std::fs::create_dir(&fixture.target).unwrap();
    std::fs::write(
        fixture.album.join("1-שיר.mp3:notes"),
        b"album auxiliary bytes",
    )
    .unwrap();
    let reads = DesktopStorage::new(std::slice::from_ref(&fixture.target)).unwrap();
    fixture.plan.request.target = Some(reads.reference(&fixture.target).unwrap());
    drop(reads);
    fixture.replan();
    let (source, target, journal) = fixture.grants();
    let roots = WindowsMutationStorage::for_selected_roots(source, target, journal).unwrap();
    let (ObjectIdentity::Windows { volume: a, .. }, ObjectIdentity::Windows { volume: b, .. }) = (
        &fixture.scope.execution_scope().source_root,
        &fixture.scope.execution_scope().target_root,
    ) else {
        panic!("native Windows identities required")
    };
    assert_ne!(
        a, b,
        "this gate requires genuine TEMP/check-out distinct volumes"
    );
    prepare_album_move_streams(&mut fixture.scope, &roots, &AtomicBool::new(false)).unwrap();
    drop(roots);
    (fixture, target_tree)
}
fn verify_cross_volume_album(fixture: &Fixture) {
    for n in 1..=3 {
        assert_eq!(
            std::fs::read(fixture.destination().join(format!("{n}-שיר.mp3"))).unwrap(),
            format!("owned audio {n}").as_bytes()
        );
    }
    assert_eq!(
        std::fs::read(fixture.destination().join("1-שיר.mp3:notes")).unwrap(),
        b"album auxiliary bytes"
    );
    assert_eq!(
        std::fs::read(fixture.destination().join("cover.jpg")).unwrap(),
        b"owned sidecar"
    );
    assert_eq!(
        std::fs::read(fixture.destination().join("booklet/nested/readme.txt")).unwrap(),
        b"nested bytes"
    );
}
#[test]
fn cross_volume_child_album_preserves_streams_and_completes_exact_cleanup() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
    assert_eq!(result.transfers.as_ref().unwrap().committed_destinations, 5);
    assert_eq!(result.transfers.as_ref().unwrap().moved_sources, 5);
    assert!(result.retained_directories.is_empty());
    assert!(!fixture.album.exists());
    assert!(fixture.source.is_dir());
    let recovery = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(recovery.manifest_finished);
    assert!(recovery.pending_cleanup.is_empty());
    assert_eq!(recovery.recorded_status.as_deref(), Some("Complete"));
    assert_eq!(recovery.completed_albums, vec![0]);
    drop(job);
    verify_cross_volume_album(&fixture);
}
#[test]
fn cross_volume_selected_root_album_uses_original_delete_handle() {
    let (fixture, _target) = cross_volume_album(true);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.albums_completed, 1);
    assert!(!fixture.source.exists());
    let manifest_path = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".album.jsonl")
        })
        .unwrap();
    let raw = std::fs::read_to_string(manifest_path).unwrap();
    let header: serde_json::Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
    let payload = &header["body"]["payload"];
    assert_eq!(payload["schema"], 2);
    assert_eq!(
        payload["original_source"]["identity"],
        payload["binding"]["source_root"]["identity"]
    );
    assert_eq!(
        payload["original_source"]["parent_identity"],
        payload["binding"]["source_root"]["parent_identity"]
    );
    assert_eq!(
        payload["original_source"]["actual_name"],
        serde_json::to_value(MutationName::Windows("source".encode_utf16().collect())).unwrap()
    );
    assert!(payload["original_source"]["size"].is_null());
    assert!(
        result
            .removed_directories
            .iter()
            .any(|directory| directory.identity == fixture.scope.execution_scope().source_root)
    );
    let recovery = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(recovery.manifest_finished);
    assert!(recovery.pending_cleanup.is_empty());
    assert!(
        recovery
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    std::fs::create_dir(&fixture.source).unwrap();
    std::fs::write(fixture.source.join("foreign.txt"), b"replacement root").unwrap();
    let recovery = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(!recovery.pending_cleanup.is_empty());
    assert!(
        recovery
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::NeedsRecovery)
    );
    assert_eq!(
        std::fs::read(fixture.source.join("foreign.txt")).unwrap(),
        b"replacement root"
    );
    drop(job);
    verify_cross_volume_album(&fixture);
}
#[test]
fn changed_album_auxiliary_stream_refuses_before_any_effect() {
    let (fixture, _target) = cross_volume_album(false);
    std::fs::write(
        fixture.album.join("1-שיר.mp3:notes"),
        b"changed before apply",
    )
    .unwrap();
    assert!(fixture.prepare().is_err());
    assert_eq!(std::fs::read_dir(&fixture.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&fixture.journal).unwrap().count(), 0);
    assert!(fixture.album.join("1-שיר.mp3").is_file());
}
#[test]
fn cross_volume_album_cancel_keeps_unretired_sources_and_reports_partial() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |_| {
            cancel.store(true, Ordering::Release);
        })
        .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 0);
    assert!(fixture.album.is_dir());
    assert!(fixture.album.join("1-שיר.mp3").exists());
}
#[test]
fn cross_volume_album_late_child_is_never_recursively_removed() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    std::fs::write(
        fixture.album.join("foreign.txt"),
        b"unplanned retained bytes",
    )
    .unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {});
    assert!(
        result.is_err()
            || result
                .as_ref()
                .is_ok_and(|value| value.status != JobStatus::Complete)
    );
    assert_eq!(
        std::fs::read(fixture.album.join("foreign.txt")).unwrap(),
        b"unplanned retained bytes"
    );
    assert!(fixture.album.join("1-שיר.mp3").exists());
}
#[test]
fn cross_volume_album_destination_collision_preserves_both_objects() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    std::fs::create_dir_all(fixture.destination()).unwrap();
    std::fs::write(
        fixture.destination().join("foreign.txt"),
        b"foreign destination",
    )
    .unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.albums_completed, 0);
    assert_eq!(
        std::fs::read(fixture.destination().join("foreign.txt")).unwrap(),
        b"foreign destination"
    );
    assert!(fixture.album.join("1-שיר.mp3").exists());
}

#[test]
fn copied_album_cleanup_rejects_hash_valid_missing_metadata_or_wrong_location() {
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
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    assert!(
        job.inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .manifest_finished
    );
    let journal = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".album.jsonl")
        })
        .unwrap();
    let original = std::fs::read(&journal).unwrap();
    for variant in 0..5 {
        let mut rewritten = Vec::new();
        let mut previous = String::new();
        let mut sequence = 0;
        let mut reached = false;
        for line in original
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
            if variant == 0
                && matches!(
                    envelope.body.phase.as_str(),
                    "DirectoryMetadataPrepared" | "DirectoryMetadataObserved"
                )
            {
                continue;
            }
            if variant == 3 && envelope.body.phase == "AlbumTransfersObserved" {
                continue;
            }
            if variant == 4 && envelope.body.phase == "AlbumTransfersObserved" {
                envelope.body.payload["events"][0]["observed_identity"] =
                    serde_json::to_value(&fixture.scope.execution_scope().source_root).unwrap();
            }
            if matches!(variant, 1 | 2) && envelope.body.phase == "CleanupObserved" {
                if variant == 1 {
                    envelope.body.payload["size"] = serde_json::json!(17);
                } else {
                    envelope.body.payload["reference"] =
                        serde_json::to_value(&fixture.plan.request.target).unwrap();
                }
            }
            let stop = envelope.body.phase
                == match variant {
                    0 => "CleanupPrepared",
                    3 => "DirectoryMetadataPrepared",
                    4 => "AlbumTransfersObserved",
                    _ => "CleanupObserved",
                };
            envelope.body.sequence = sequence;
            sequence += 1;
            envelope.body.previous = previous;
            envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
            previous = envelope.sha256.clone();
            rewritten.extend(serde_json::to_vec(&envelope).unwrap());
            rewritten.push(b'\n');
            if stop {
                reached = true;
                break;
            }
        }
        assert!(reached);
        std::fs::write(&journal, rewritten).unwrap();
        assert!(matches!(
            job.inspect_recovery(&AtomicBool::new(false)),
            Err(E::CorruptJournal)
        ));
    }
    std::fs::write(&journal, original).unwrap();
    assert!(
        job.inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .manifest_finished
    );
}

#[test]
fn later_cross_volume_cancellation_keeps_earlier_album_cleanup_and_credit() {
    let (mut fixture, _target) = cross_volume_album(false);
    let second = fixture.source.join("second-album");
    std::fs::create_dir(&second).unwrap();
    for n in 1..=3 {
        std::fs::write(
            second.join(format!("{n}-second.mp3")),
            format!("second {n}"),
        )
        .unwrap();
    }
    fixture.replan();
    assert_eq!(fixture.plan.operations.len(), 2);
    let (source, target, journal) = fixture.grants();
    let roots = WindowsMutationStorage::for_selected_roots(source, target, journal).unwrap();
    prepare_album_move_streams(&mut fixture.scope, &roots, &AtomicBool::new(false)).unwrap();
    drop(roots);
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let mut saw_second = false;
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 1 {
                // Completion and source-directory cleanup must precede the next album.
                assert!(!fixture.album.exists());
                saw_second = true;
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert!(saw_second);
    assert_eq!(result.status, JobStatus::Cancelled, "{result:?}");
    assert_eq!(result.completed_album_operations, vec![0]);
    assert_eq!(result.albums_completed, 1);
    assert_eq!(result.artist_files, vec![("שלום עולם".into(), 4)]);
    assert!(!fixture.album.exists());
    assert!(second.join("1-second.mp3").is_file());
    let recovery = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(recovery.manifest_finished);
    assert!(recovery.pending_cleanup.is_empty());
    assert_eq!(recovery.recorded_status.as_deref(), Some("Cancelled"));
    assert_eq!(recovery.completed_albums, vec![0]);
    drop(job);
    verify_cross_volume_album(&fixture);
}

#[test]
fn copied_album_controlled_recovery_cancels_chunks_with_or_without_source_root() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::{Arc, atomic::AtomicU64};
    for root_album in [false, true] {
        let (mut fixture, _target) = cross_volume_album(root_album);
        std::fs::write(fixture.album.join("1-שיר.mp3"), vec![7u8; 256 * 1024]).unwrap();
        fixture.replan();
        let (source, target, journal) = fixture.grants();
        let roots = WindowsMutationStorage::for_selected_roots(source, target, journal).unwrap();
        prepare_album_move_streams(&mut fixture.scope, &roots, &AtomicBool::new(false)).unwrap();
        drop(roots);
        let mut job = fixture.job();
        assert_eq!(
            job.run(&AtomicBool::new(false), &mut |_| {})
                .unwrap()
                .status,
            JobStatus::Complete
        );
        let before = std::fs::read_dir(&fixture.journal)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let journal_cancel = Arc::new(AtomicBool::new(false));
        let flag = journal_cancel.clone();
        let journal_control = InspectionControl::with_progress_interval(
            journal_cancel,
            std::time::Duration::ZERO,
            move |progress| {
                if progress.stage == "album_journals" && progress.bytes_read > 0 {
                    flag.store(true, Ordering::Release);
                }
            },
        );
        assert!(matches!(
            job.inspect_file_recovery_controlled(&journal_control),
            Err(E::Cancelled)
        ));
        assert!(journal_control.snapshot().unwrap().bytes_read > 0);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let baseline = Arc::new(AtomicU64::new(0));
        let observed = baseline.clone();
        let control = InspectionControl::with_progress_interval(
            cancel,
            std::time::Duration::ZERO,
            move |progress| {
                if progress.stage == "metadata" {
                    observed.store(progress.bytes_read, Ordering::Release);
                }
                if progress.stage == "files"
                    && progress.bytes_read > observed.load(Ordering::Acquire)
                {
                    flag.store(true, Ordering::Release);
                }
            },
        );
        assert!(matches!(
            job.inspect_file_recovery_controlled(&control),
            Err(E::Cancelled)
        ));
        let count = control.snapshot().unwrap().bytes_read;
        assert!(count > baseline.load(Ordering::Acquire));
        assert!(count <= baseline.load(Ordering::Acquire) + 65536);
        for (path, bytes) in &before {
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }
        let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
        let report = job.inspect_file_recovery_controlled(&control).unwrap();
        assert!(report.manifest_finished);
        assert!(
            report
                .transfers
                .unwrap()
                .operations
                .iter()
                .all(|operation| operation.status
                    == sorter_executor::recovery::RecoveryStatus::Committed)
        );
        assert_eq!(
            std::fs::read(fixture.destination().join("1-שיר.mp3")).unwrap(),
            vec![7u8; 256 * 1024]
        );
        for (path, bytes) in before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
fn cancellation_after_last_retirement_stops_album_without_rewriting_file_terminal() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let mut progress_events = 0;
    let result = job
        .run(&cancel, &mut |_| {
            progress_events += 1;
            // Five one-chunk files each report copied bytes and the acknowledged
            // retirement handoff. Stop at the last handoff, before directory metadata.
            if progress_events == 10 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(progress_events, 10);
    assert_eq!(result.status, JobStatus::Cancelled, "{result:?}");
    assert_eq!(result.albums_completed, 0);
    assert!(result.completed_album_operations.is_empty());
    assert!(fixture.album.is_dir());
    let transfers = result.transfers.as_ref().unwrap();
    assert_eq!(transfers.committed_destinations, 5);
    assert_eq!(transfers.moved_sources, 5);
    let journals = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let physical = journals
        .iter()
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".transfers.jsonl")
        })
        .unwrap();
    let lines = std::fs::read_to_string(physical).unwrap();
    let last: serde_json::Value = serde_json::from_str(lines.lines().last().unwrap()).unwrap();
    assert_eq!(last["body"]["event"]["phase"], "SourceRetired");
    let manifest = journals
        .iter()
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".album.jsonl")
        })
        .unwrap();
    assert!(
        std::fs::read_to_string(manifest)
            .unwrap()
            .contains("AlbumCompletionStopped")
    );
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(report.manifest_finished);
    assert!(!report.incomplete_tail);
    assert_eq!(report.recorded_status.as_deref(), Some("Cancelled"));
    assert!(report.completed_albums.is_empty());
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed)
    );
}

#[test]
fn copied_album_original_plan_revalidation_preserves_cancellation() {
    struct CancellingMetadata<'a>(&'a AtomicBool);
    impl MetadataProvider for CancellingMetadata<'_> {
        fn open(
            &mut self,
            entry: &EntryObservation,
        ) -> Result<Box<dyn MetadataFields>, MetadataError> {
            self.0.store(true, Ordering::Release);
            Ok(Box::new(Fields(entry.name.clone())))
        }
    }
    let (fixture, _target) = cross_volume_album(false);
    let cancel = AtomicBool::new(false);
    let result = validate_album_execution(
        &fixture.plan,
        &mut CancellingMetadata(&cancel),
        &mut DisabledRecognition,
        fixture.scope.resources(),
        &fixture.scope,
        &cancel,
    );
    assert!(cancel.load(Ordering::Acquire));
    assert_eq!(result.err(), Some(E::Cancelled));
    assert_eq!(std::fs::read_dir(&fixture.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&fixture.journal).unwrap().count(), 0);
    assert!(fixture.album.join("1-שיר.mp3").is_file());
}

#[test]
fn late_child_after_one_copied_retirement_stops_remaining_album_effects() {
    let (fixture, _target) = cross_volume_album(false);
    let mut job = fixture.job();
    let mut progress_events = 0;
    let foreign = fixture.album.join("late-foreign.txt");
    let result = job
        .run(&AtomicBool::new(false), &mut |_| {
            progress_events += 1;
            if progress_events == 2 {
                std::fs::write(&foreign, b"late external child").unwrap();
            }
        })
        .unwrap();
    assert_eq!(progress_events, 2);
    assert_eq!(result.status, JobStatus::Partial, "{result:?}");
    assert_eq!(result.albums_completed, 0);
    assert!(result.removed_directories.is_empty());
    let transfers = result.transfers.as_ref().unwrap();
    assert_eq!(transfers.committed_destinations, 1);
    assert_eq!(transfers.moved_sources, 1);
    let original = [
        "1-שיר.mp3",
        "2-שיר.mp3",
        "3-שיר.mp3",
        "cover.jpg",
        "booklet/nested/readme.txt",
    ];
    assert_eq!(
        original
            .iter()
            .filter(|name| fixture.album.join(name).is_file())
            .count(),
        4
    );
    assert_eq!(std::fs::read(&foreign).unwrap(), b"late external child");
    assert!(
        job.inspect_recovery(&AtomicBool::new(false))
            .unwrap()
            .manifest_finished
    );
    assert_eq!(std::fs::read(&foreign).unwrap(), b"late external child");
}

#[test]
fn selected_root_short_leaf_alias_recovers_against_native_long_leaf() {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetShortPathNameW"]
        fn get_short_path_name(input: *const u16, output: *mut u16, capacity: u32) -> u32;
    }
    let (mut fixture, _target) = cross_volume_album(true);
    let long = fixture
        .source
        .parent()
        .unwrap()
        .join("Owned Album Source With Long Leaf");
    std::fs::rename(&fixture.source, &long).unwrap();
    let input = long
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut output = vec![0u16; 32768];
    // SAFETY: input is NUL-terminated; output is a writable buffer of capacity
    // passed in UTF-16 units, as documented by GetShortPathNameW.
    let length =
        unsafe { get_short_path_name(input.as_ptr(), output.as_mut_ptr(), output.len() as u32) }
            as usize;
    assert!(
        length > 0 && length < output.len(),
        "GetShortPathNameW: {}",
        std::io::Error::last_os_error()
    );
    let short = PathBuf::from(std::ffi::OsString::from_wide(&output[..length]));
    assert_ne!(
        short.file_name(),
        long.file_name(),
        "this fixture requires a genuine NTFS short leaf alias"
    );
    fixture.source = short.clone();
    fixture.album = short.clone();
    fixture.plan.request.source = StorageRef::WindowsPath {
        utf16: short.as_os_str().encode_wide().collect(),
    };
    fixture.replan();
    let (source, target, journal) = fixture.grants();
    let roots = WindowsMutationStorage::for_selected_roots(source, target, journal).unwrap();
    prepare_album_move_streams(&mut fixture.scope, &roots, &AtomicBool::new(false)).unwrap();
    drop(roots);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    assert!(!long.exists());
    assert!(!short.exists());
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert_eq!(report.recorded_status.as_deref(), Some("Complete"));
    assert!(
        report
            .removed_directory_references
            .iter()
            .any(
                |(reference, identity)| reference == &fixture.plan.request.source
                    && identity == &fixture.scope.execution_scope().source_root
            )
    );
    assert!(report.manifest_finished);
    assert!(report.pending_cleanup.is_empty());
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    drop(job);
    verify_cross_volume_album(&fixture);
}
