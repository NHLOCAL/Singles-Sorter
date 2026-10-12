//! Owned selected-root product fixtures for the exact shared Ogg transformer.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[path = "../../../../tests/staged_ogg/fixture_support.rs"]
mod media;

fn ogg(opus: bool, duplicates: bool, unchanged: bool) -> Vec<u8> {
    let mut values = vec!["x-Custom=one".to_owned(), media::PICTURE.to_owned(), "X-custom=two".to_owned(), "TRACKNUMBER=03".to_owned(), "ALBUM ARTIST=keep alias".to_owned()];
    for key in ["ARTIST", "ALBUMARTIST", "TITLE", "ALBUM", "GENRE"] {
        values.push(format!("{key}={}", if unchanged { "שלום" } else { "à" }));
        if duplicates { values.push(format!("{}=á", key.to_ascii_lowercase())); }
    }
    let refs = values.iter().map(String::as_str).collect::<Vec<_>>();
    media::build(opus, &refs, if opus { b"\x01opaque\0\xff" } else { b"\x81\0padding" }, 255)
}
fn name(opus: bool) -> &'static str { if opus { "song__demo.opus" } else { "song__demo.ogg" } }
fn journal(fixture: &Fixture) -> (PathBuf, Vec<u8>) {
    let path = std::fs::read_dir(&fixture.journal).unwrap().next().unwrap().unwrap().path();
    let bytes = std::fs::read(&path).unwrap(); (path, bytes)
}
fn backup(fixture: &Fixture) -> PathBuf {
    std::fs::read_dir(&fixture.source).unwrap().map(|e| e.unwrap().path()).find(|p| p.extension().is_some_and(|e| e == "backup")).unwrap()
}
#[derive(Serialize, Deserialize)]
struct Body { sequence: u64, previous: String, job_id: String, operation: Option<usize>, phase: String, payload: Value }
#[derive(Serialize, Deserialize)]
struct Envelope { body: Body, sha256: String }
fn rewrite(bytes: &[u8], mut edit: impl FnMut(&mut Body)) -> Vec<u8> {
    let mut previous = String::new(); let mut out = Vec::new();
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let mut e: Envelope = serde_json::from_slice(line).unwrap(); edit(&mut e.body); e.body.previous = previous;
        e.sha256 = digest(&serde_json::to_vec(&e.body).unwrap()); previous = e.sha256.clone();
        out.extend(serde_json::to_vec(&e).unwrap()); out.push(b'\n');
    }
    out
}
fn through(bytes: &[u8], phase: &str) -> Vec<u8> {
    let mut out = Vec::new(); let mut found = false;
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let e: Envelope = serde_json::from_slice(line).unwrap(); out.extend(line); out.push(b'\n');
        if e.body.phase == phase { found = true; break; }
    }
    assert!(found); out
}
fn verify_role(report: &RepairRecoveryReport, role: &str) {
    assert!(report.operations[0].evidence.iter().any(|e| e.role == role && matches!(e.status, RepairEvidenceStatus::Verified)), "{report:?}");
}
fn assert_tags(before: &[u8], after: &[u8], duplicate: bool) {
    media::assert_preserved(before, after);
    let (vendor, values, suffix) = media::comments(after); assert_eq!(vendor, media::VENDOR); assert_eq!(suffix, media::comments(before).2);
    let mut expected = vec!["x-Custom=one".to_owned(), media::PICTURE.to_owned(), "X-custom=two".to_owned(), "TRACKNUMBER=03".to_owned(), "ALBUM ARTIST=keep alias".to_owned()];
    for key in ["artist", "albumartist", "title", "album", "genre"] { expected.push(format!("{key}={}", if duplicate { "א, ב" } else { "א" })); }
    assert_eq!(values, expected);
}
fn decode(path: &std::path::Path) -> Vec<u8> {
    let result = std::process::Command::new("ffmpeg").args(["-v", "error", "-nostdin", "-i"]).arg(path).args(["-f", "s16le", "-acodec", "pcm_s16le", "-"]).output().expect("FFmpeg required for product decoder proof");
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr)); assert!(!result.stdout.is_empty()); result.stdout
}
fn native_product(opus: bool, duplicate: bool) {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    assert!(MetadataReader::is_available(), "native metadata bridge is mandatory");
    let before = ogg(opus, duplicate, false);
    let original = if duplicate { "à, á" } else { "à" };
    let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some(original)));
    let original_pcm = decode(&fixture.source.join(name(opus)));
    let mut reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata_reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata = StorageMetadataProvider { storage: &mut metadata_reads, reader: MetadataReader };
    let plan = plan_job(&fixture.plan.request, &mut reads, &mut metadata, &mut Ai).unwrap();
    assert_eq!(plan.operations.len(), 1);
    let OperationKind::FixFile(fix) = &plan.operations[0].kind else { panic!("fix"); };
    assert_eq!(fix.repairs.len(), 5);
    for (repair, field) in fix.repairs.iter().zip(sorter_core::fix_plan::FIX_FIELDS) {
        assert_eq!(repair.field, field); assert_eq!(repair.original, original);
        assert_eq!(repair.replacement, if duplicate { "א, ב" } else { "א" });
    }
    let final_name = fix.intended_name.clone().unwrap();
    let original_identity = fix.source.identity.clone().unwrap();
    let scope = prepare_repair_scope(&plan, &mut reads, fixture.scope.resources(), &AtomicBool::new(false)).unwrap();
    let mut job = SelectedWindowsRepairJob::prepare(fixture.roots(), &plan, &mut metadata, &mut Ai, scope.resources(), &scope, &AtomicBool::new(false)).unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}"); assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    assert_eq!(result.items[0].backup.as_ref().unwrap().identity, original_identity);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap(); verify_role(&report, "current"); verify_role(&report, "backup"); drop(job);
    let destination = fixture.source.join(final_name); let after = std::fs::read(&destination).unwrap();
    if duplicate { assert!(after.len() < before.len()); } else { assert_eq!(after.len(), before.len() + 27); }
    assert_tags(&before, &after, duplicate); assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    assert_eq!(decode(&destination), original_pcm);
    let (_, bytes) = journal(&fixture);
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let e: Envelope = serde_json::from_slice(line).unwrap();
        if e.body.phase == "TempPrepared" {
            assert_eq!(e.body.payload["replacement_length"], json!(after.len()));
            assert_eq!(e.body.payload["replacement_sha256"], json!(digest(&after)));
            assert_eq!(e.body.payload["source"]["sha256"], json!(digest(&before)));
        }
    }
}
#[test]
fn real_taglib_provider_plans_rebinds_and_publishes_vorbis_growth() { native_product(false, false); }
#[test]
fn real_taglib_provider_plans_rebinds_and_publishes_opus_duplicate_shrink() { native_product(true, true); }
#[test]
fn duplicate_five_field_ogg_repairs_preserve_artwork_and_exact_backup() {
    for opus in [false, true] {
        let before = ogg(opus, true, false); let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some("à, á")));
        let mut job = fixture.job(); let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}"); drop(job);
        let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
        assert!(after.len() < before.len()); assert_tags(&before, &after, true); assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    }
}
#[test]
fn unchanged_ogg_tags_keep_identity_without_stage_or_backup() {
    for opus in [false, true] {
        let before = ogg(opus, false, true); let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some("שלום")));
        let OperationKind::FixFile(fix) = &fixture.plan.operations[0].kind else { panic!("fix"); };
        let identity = fix.source.identity.clone().unwrap(); let mut job = fixture.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap(); assert_eq!(result.status, RepairJobStatus::Complete);
        assert_eq!((result.renamed_files, result.repaired_files, result.unchanged_files), (1, 0, 1));
        assert_eq!(result.items[0].current.as_ref().unwrap().identity, identity); assert_eq!(result.retained_backup_bytes, 0);
        assert!(result.items[0].backup.is_none() && result.items[0].staged.is_none()); drop(job);
        assert_eq!(std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(), before); assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
    }
}
#[test]
fn corrupt_crc_truncation_chaining_and_multiplexing_retain_filename_only() {
    for opus in [false, true] {
        let original = ogg(opus, false, false); let mut crc = original.clone(); *crc.last_mut().unwrap() ^= 1;
        let mut chained = original.clone(); chained.extend(&original);
        let mut parsed = media::pages(&original); let last = parsed.last_mut().unwrap(); last.raw[14] ^= 1; media::reseal(&mut last.raw);
        let mux = parsed.into_iter().flat_map(|p| p.raw).collect::<Vec<_>>();
        for before in [crc, chained, mux, original[..original.len()-1].to_vec()] {
            let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some("à"))); let mut job = fixture.job();
            let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
            assert_eq!(result.status, RepairJobStatus::Partial, "{result:?}"); assert_eq!((result.renamed_files, result.repaired_files, result.unsupported_files), (1, 0, 1));
            assert_eq!(result.retained_backup_bytes, 0); assert!(result.items[0].staged.is_none() && result.items[0].backup.is_none()); drop(job);
            assert_eq!(std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(), before); assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
        }
    }
}
#[test]
fn cancellation_during_ogg_stage_preserves_original_and_forbids_replay() {
    for opus in [false, true] {
        let before = ogg(opus, false, false); let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some("à")));
        let mut job = fixture.job(); let flag = AtomicBool::new(false);
        let result = job.run(&flag, &mut |p| { if p.phase == "staging_tags" { flag.store(true, Ordering::Release); } }).unwrap();
        assert_eq!(result.status, RepairJobStatus::Cancelled); assert_eq!(result.repaired_files, 0);
        assert!(matches!(job.run(&AtomicBool::new(false), &mut |_| {}), Err(E::NeedsRecovery)));
        verify_role(&job.inspect_recovery(&AtomicBool::new(false)).unwrap(), "current"); drop(job);
        assert_eq!(std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(), before);
    }
}
#[test]
fn cancellation_and_lost_backup_response_keep_grown_stage_and_original_lengths() {
    let before = ogg(false, false, false); let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job(); let flag = AtomicBool::new(false);
    let result = job.run(&flag, &mut |p| { if p.phase == "original_backed_up" { flag.store(true, Ordering::Release); } }).unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery); assert_eq!(result.retained_backup_bytes, before.len() as u64);
    assert_eq!(result.items[0].staged.as_ref().unwrap().size, Some(before.len() as u64 + 27));
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap(); verify_role(&report, "backup"); verify_role(&report, "staged"); drop(job);
    let (path, bytes) = journal(&fixture); let roots = fixture.roots();
    let report = recover_repair_journal(&roots, &through(&bytes, "BackupPrepared"), &AtomicBool::new(false)).unwrap();
    verify_role(&report, "backup_candidate"); verify_role(&report, "staged");
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before); assert_eq!(std::fs::read(path).unwrap(), bytes);
}
#[test]
fn lost_publication_response_reopens_grown_and_shorter_replacements() {
    for opus in [false, true] {
        let before = ogg(opus, opus, false); let fixture = Fixture::new(&[(name(opus), before.clone())], Metadata(Some(if opus { "à, á" } else { "à" })));
        let mut job = fixture.job(); assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete); drop(job);
        let (path, bytes) = journal(&fixture); let roots = fixture.roots();
        let report = recover_repair_journal(&roots, &through(&bytes, "PublishPrepared"), &AtomicBool::new(false)).unwrap();
        verify_role(&report, "published_candidate"); verify_role(&report, "backup");
        assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before); assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn rehashed_oversize_inconsistent_lengths_hashes_and_v1_downgrade_are_rejected() {
    let before = ogg(false, false, false); let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job(); assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete); drop(job);
    let (path, bytes) = journal(&fixture); let roots = fixture.roots();
    for (phase, key, value) in [
        ("TempPrepared", "replacement_length", json!(before.len() + 28)),
        ("TempPrepared", "replacement_length", json!(u64::MAX)),
        ("TempPrepared", "replacement_length", json!(0)),
        ("TempPrepared", "replacement_length", Value::Null),
        ("TempPrepared", "replacement_length", json!(before.len())),
        ("TempPrepared", "replacement_length", json!(before.len() + 26)),
        ("TempPrepared", "replacement_sha256", json!(digest(b"foreign"))),
        ("BackupPrepared", "original_length", json!(before.len() + 27)),
        ("PublishPrepared", "replacement_length", json!(before.len())),
        ("StagedVerified", "length", json!(before.len())),
    ] {
        let changed = rewrite(&bytes, |body| { if body.phase == phase { body.payload[key] = value.clone(); } });
        assert!(matches!(recover_repair_journal(&roots, &changed, &AtomicBool::new(false)), Err(E::CorruptJournal)), "{phase}/{key}");
    }
    let old = rewrite(&bytes, |body| { if body.phase == "Header" { body.payload["schema"] = json!(1); } if body.phase == "TempPrepared" { body.payload = body.payload["source"].clone(); } });
    assert!(matches!(recover_repair_journal(&roots, &old, &AtomicBool::new(false)), Err(E::CorruptJournal)));
    assert_eq!(std::fs::read(path).unwrap(), bytes); assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}
#[test]
fn coherent_rehashed_wrong_replacement_size_never_verifies_current_artifact() {
    let before = ogg(false, false, false); let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job(); assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete); drop(job);
    let (path, bytes) = journal(&fixture);
    let changed = rewrite(&bytes, |body| {
        let length = json!(before.len());
        match body.phase.as_str() {
            "TempPrepared" => body.payload["replacement_length"] = length,
            "StagedVerified" => { body.payload["length"] = length.clone(); body.payload["observation"]["size"] = length; }
            "PublishPrepared" => { body.payload["replacement_length"] = length.clone(); body.payload["staged"]["size"] = length; }
            "PublishedVerified" => body.payload["observation"]["size"] = length,
            "Committed" => body.payload["current"]["size"] = length,
            "Finished" => body.payload["items"][0]["current"]["size"] = length,
            _ => {}
        }
    });
    let roots = fixture.roots(); let report = recover_repair_journal(&roots, &changed, &AtomicBool::new(false)).unwrap();
    assert!(report.operations[0].evidence.iter().any(|e| e.role == "current" && matches!(e.status, RepairEvidenceStatus::Changed))); verify_role(&report, "backup");
    assert_eq!(std::fs::read(path).unwrap(), bytes); assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}
#[test]
fn foreign_collision_after_backup_preserves_grown_stage_and_exact_original() {
    let before = ogg(false, false, false); let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job(); let destination = fixture.source.join(fixture.final_name(0));
    let result = job.run(&AtomicBool::new(false), &mut |p| { if p.phase == "original_backed_up" { std::fs::write(&destination, b"foreign owner").unwrap(); } }).unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery); assert_eq!(result.repaired_files, 0);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap(); verify_role(&report, "backup"); verify_role(&report, "staged"); drop(job);
    assert_eq!(std::fs::read(destination).unwrap(), b"foreign owner"); assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}
