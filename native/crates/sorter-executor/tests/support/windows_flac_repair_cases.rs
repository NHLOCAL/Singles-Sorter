//! Product fixtures share the exact selected-root host setup with MP3 cases.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const BASE: &[u8] = include_bytes!("../../../../tests/fixtures/media/blank.flac");
const VENDOR: &[u8] = b"Owned product FLAC vendor";
fn split(bytes: &[u8]) -> (Vec<Vec<u8>>, &[u8]) {
    assert_eq!(&bytes[..4], b"fLaC");
    let mut at = 4;
    let mut blocks = Vec::new();
    loop {
        let kind = bytes[at];
        let size = (usize::from(bytes[at + 1]) << 16)
            | (usize::from(bytes[at + 2]) << 8) | usize::from(bytes[at + 3]);
        let end = at + 4 + size;
        blocks.push(bytes[at..end].to_vec());
        at = end;
        if kind & 128 != 0 { break; }
    }
    (blocks, &bytes[at..])
}
fn vector(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend((bytes.len() as u32).to_le_bytes());
    out.extend(bytes);
}
fn picture() -> Vec<u8> {
    let mut out = 3u32.to_be_bytes().to_vec();
    out.extend(9u32.to_be_bytes());
    out.extend(b"image/png");
    out.extend(0u32.to_be_bytes());
    for value in [1u32, 1, 24, 0] { out.extend(value.to_be_bytes()); }
    // Owned, generated 1x1 RGB black PNG; no external artwork.
    let png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00,
        0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x60, 0x60, 0x00,
        0x00, 0x00, 0x04, 0x00, 0x01, 0xf6, 0x17, 0x38, 0x55, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    out.extend((png.len() as u32).to_be_bytes());
    out.extend(png);
    out
}
fn flac(duplicate: bool, opaque: Option<u8>, unchanged: bool) -> Vec<u8> {
    let mut values = vec!["x-Custom=one".to_owned(), "X-custom=two".to_owned(), "TRACKNUMBER=03".to_owned()];
    for key in ["ARTIST", "ALBUMARTIST", "TITLE", "ALBUM", "GENRE"] {
        values.push(format!("{key}={}", if unchanged { "שלום" } else { "à" }));
        if duplicate { values.push(format!("{}=á", key.to_ascii_lowercase())); }
    }
    let mut comments = Vec::new();
    vector(&mut comments, VENDOR);
    comments.extend((values.len() as u32).to_le_bytes());
    for value in values { vector(&mut comments, value.as_bytes()); }
    let mut blocks = vec![(0, BASE[8..42].to_vec()), (6, picture()), (4, comments)];
    if let Some(kind) = opaque { blocks.push((kind, b"testprivate offsets unverified".to_vec())); }
    let mut file = b"fLaC".to_vec();
    for (index, (kind, body)) in blocks.iter().enumerate() {
        let flag = if index + 1 == blocks.len() { 128 } else { 0 };
        file.extend([*kind | flag, (body.len() >> 16) as u8, (body.len() >> 8) as u8, body.len() as u8]);
        file.extend(body);
    }
    file.extend(split(BASE).1);
    file
}
fn comments(bytes: &[u8]) -> (Vec<u8>, Vec<String>) {
    let (blocks, _) = split(bytes);
    let body = &blocks.iter().find(|block| block[0] & 127 == 4).unwrap()[4..];
    fn word(bytes: &[u8], at: &mut usize) -> usize {
        let size = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
        *at += 4;
        size
    }
    let mut at = 0;
    let len = word(body, &mut at);
    let vendor = body[at..at + len].to_vec();
    at += len;
    let count = word(body, &mut at);
    let mut values = Vec::new();
    for _ in 0..count {
        let len = word(body, &mut at);
        values.push(String::from_utf8(body[at..at + len].to_vec()).unwrap());
        at += len;
    }
    assert_eq!(at, body.len());
    (vendor, values)
}
fn journal(fixture: &Fixture) -> (PathBuf, Vec<u8>) {
    let path = std::fs::read_dir(&fixture.journal).unwrap().next().unwrap().unwrap().path();
    let bytes = std::fs::read(&path).unwrap();
    (path, bytes)
}
fn backup(fixture: &Fixture) -> PathBuf {
    std::fs::read_dir(&fixture.source).unwrap().map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "backup")).unwrap()
}
#[derive(Serialize, Deserialize)]
struct Body {
    sequence: u64,
    previous: String,
    job_id: String,
    operation: Option<usize>,
    phase: String,
    payload: Value,
}
#[derive(Serialize, Deserialize)]
struct Envelope { body: Body, sha256: String }
fn rewrite(bytes: &[u8], mut edit: impl FnMut(&mut Body)) -> Vec<u8> {
    let mut previous = String::new();
    let mut output = Vec::new();
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
        edit(&mut envelope.body);
        envelope.body.previous = previous;
        envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
        previous = envelope.sha256.clone();
        output.extend(serde_json::to_vec(&envelope).unwrap());
        output.push(b'\n');
    }
    output
}
fn through(bytes: &[u8], phase: &str) -> Vec<u8> {
    let mut result = Vec::new();
    let mut found = false;
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let envelope: Envelope = serde_json::from_slice(line).unwrap();
        result.extend(line);
        result.push(b'\n');
        if envelope.body.phase == phase { found = true; break; }
    }
    assert!(found);
    result
}
fn verify_role(report: &RepairRecoveryReport, role: &str) {
    assert!(report.operations[0].evidence.iter().any(|e| e.role == role && matches!(e.status, RepairEvidenceStatus::Verified)), "{report:?}");
}

#[test]
fn five_fields_shrink_preserves_picture_vendor_unknowns_audio_and_exact_backup() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song__demo.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verify_role(&report, "current");
    verify_role(&report, "backup");
    drop(job);
    let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
    assert!(after.len() < before.len());
    let (old_blocks, old_audio) = split(&before);
    let (new_blocks, new_audio) = split(&after);
    assert_eq!(old_audio, new_audio);
    for (old, new) in old_blocks.iter().zip(&new_blocks) {
        if old[0] & 127 != 4 { assert_eq!(old, new); }
    }
    assert_eq!(comments(&after), (VENDOR.to_vec(), vec![
        "x-Custom=one", "X-custom=two", "TRACKNUMBER=03", "artist=א, ב", "albumartist=א, ב", "title=א, ב", "album=א, ב", "genre=א, ב",
    ].into_iter().map(String::from).collect()));
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    // Reopened roots, no in-memory staged plan or execution guards.
    let (path, bytes) = journal(&fixture);
    let roots = fixture.roots();
    let reopened = recover_repair_journal(&roots, &bytes, &AtomicBool::new(false)).unwrap();
    verify_role(&reopened, "current");
    verify_role(&reopened, "backup");
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn unchanged_flac_tags_keep_identity_without_stage_or_backup() {
    let before = flac(false, None, true);
    let fixture = Fixture::new(&[("song__demo.flac", before.clone())], Metadata(Some("שלום")));
    let OperationKind::FixFile(fix) = &fixture.plan.operations[0].kind else { panic!("fix"); };
    let original_id = fix.source.identity.clone().unwrap();
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete);
    assert_eq!((result.renamed_files, result.repaired_files, result.unchanged_files), (1, 0, 1));
    assert_eq!(result.retained_backup_bytes, 0);
    assert_eq!(result.items[0].current.as_ref().unwrap().identity, original_id);
    assert!(result.items[0].backup.is_none() && result.items[0].staged.is_none());
    drop(job);
    assert_eq!(std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(), before);
    assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
}

#[test]
fn opaque_application_and_reserved_blocks_reject_shift_after_filename_repair() {
    for kind in [2, 126] {
        let before = flac(true, Some(kind), false);
        let fixture = Fixture::new(&[("song__demo.flac", before.clone())], Metadata(Some("à, á")));
        let mut job = fixture.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Partial);
        assert_eq!((result.renamed_files, result.unsupported_files, result.repaired_files), (1, 1, 0));
        assert!(result.items[0].detail.contains("opaque FLAC"));
        drop(job);
        assert_eq!(std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(), before);
        assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
    }
}

#[test]
fn equal_length_flac_repair_keeps_opaque_block_bytes() {
    let before = flac(false, Some(2), false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    drop(job);
    let after = std::fs::read(fixture.source.join("song.flac")).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(split(&before).0.last(), split(&after).0.last());
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}

#[test]
fn cancellation_during_flac_stage_preserves_original_and_forbids_replay() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let result = job.run(&cancel, &mut |p| {
        if p.phase == "staging_tags" { cancel.store(true, Ordering::Release); }
    }).unwrap();
    assert_eq!(result.status, RepairJobStatus::Cancelled);
    assert_eq!(result.repaired_files, 0);
    assert!(matches!(job.run(&AtomicBool::new(false), &mut |_| {}), Err(E::NeedsRecovery)));
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verify_role(&report, "current");
    drop(job);
    assert_eq!(std::fs::read(fixture.source.join("song.flac")).unwrap(), before);
}

#[test]
fn cancellation_and_lost_backup_response_keep_distinct_original_and_stage_lengths() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let result = job.run(&cancel, &mut |p| {
        if p.phase == "original_backed_up" { cancel.store(true, Ordering::Release); }
    }).unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery);
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    assert!(result.items[0].staged.as_ref().unwrap().size.unwrap() < before.len() as u64);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verify_role(&report, "backup");
    verify_role(&report, "staged");
    drop(job);
    let (path, bytes) = journal(&fixture);
    let lost = through(&bytes, "BackupPrepared");
    let roots = fixture.roots();
    let reopened = recover_repair_journal(&roots, &lost, &AtomicBool::new(false)).unwrap();
    verify_role(&reopened, "backup_candidate");
    verify_role(&reopened, "staged");
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    assert!(!fixture.source.join("song.flac").exists());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn lost_publication_response_reopens_with_replacement_length_and_original_backup() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete);
    drop(job);
    let (path, bytes) = journal(&fixture);
    let roots = fixture.roots();
    let lost = through(&bytes, "PublishPrepared");
    let report = recover_repair_journal(&roots, &lost, &AtomicBool::new(false)).unwrap();
    verify_role(&report, "published_candidate");
    verify_role(&report, "backup");
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn rehashed_original_replacement_length_or_hash_substitution_is_rejected() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete);
    drop(job);
    let (path, bytes) = journal(&fixture);
    let roots = fixture.roots();
    for (phase, key, value) in [
        ("TempPrepared", "replacement_length", json!(before.len())),
        ("TempPrepared", "replacement_length", Value::Null),
        ("TempPrepared", "replacement_length", json!(-1)),
        ("TempPrepared", "replacement_length", json!(before.len() + 1)),
        ("TempPrepared", "replacement_sha256", json!(digest(b"foreign"))),
        ("BackupPrepared", "original_length", json!(before.len() - 1)),
        ("PublishPrepared", "replacement_length", json!(before.len())),
        ("StagedVerified", "length", json!(before.len())),
    ] {
        let tampered = rewrite(&bytes, |body| {
            if body.phase == phase { body.payload[key] = value.clone(); }
        });
        assert!(matches!(recover_repair_journal(&roots, &tampered, &AtomicBool::new(false)), Err(E::CorruptJournal)), "{phase}/{key}");
    }
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}

#[test]
fn coherent_rehashed_wrong_sizes_and_foreign_identical_bytes_never_verify() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete);
    drop(job);
    let (path, bytes) = journal(&fixture);
    let tampered = rewrite(&bytes, |body| {
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
    let roots = fixture.roots();
    let report = recover_repair_journal(&roots, &tampered, &AtomicBool::new(false)).unwrap();
    assert!(report.operations[0].evidence.iter().any(|e| e.role == "current" && matches!(e.status, RepairEvidenceStatus::Changed)));
    verify_role(&report, "backup");
    drop(roots);
    let current = fixture.source.join("song.flac");
    let replacement = std::fs::read(&current).unwrap();
    std::fs::rename(&current, fixture.source.join("retained-replacement.flac")).unwrap();
    std::fs::write(&current, &replacement).unwrap();
    let roots = fixture.roots();
    let report = recover_repair_journal(&roots, &bytes, &AtomicBool::new(false)).unwrap();
    assert!(report.operations[0].evidence.iter().any(|e| e.role == "current" && matches!(e.status, RepairEvidenceStatus::Changed)));
    verify_role(&report, "backup");
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    assert_eq!(std::fs::read(current).unwrap(), replacement);
}

#[test]
fn legacy_v1_mp3_journal_reopens_without_new_evidence_or_rewriting_bytes() {
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("àáøäí")));
    let mut job = fixture.job();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete);
    drop(job);
    let (path, bytes) = journal(&fixture);
    let legacy = rewrite(&bytes, |body| {
        match body.phase.as_str() {
            "Header" => body.payload["schema"] = json!(1),
            "TempPrepared" => body.payload = body.payload["source"].clone(),
            "BackupPrepared" => { body.payload.as_object_mut().unwrap().remove("original_length"); }
            "PublishPrepared" => { body.payload.as_object_mut().unwrap().remove("replacement_length"); }
            _ => {}
        }
    });
    let roots = fixture.roots();
    let report = recover_repair_journal(&roots, &legacy, &AtomicBool::new(false)).unwrap();
    verify_role(&report, "current");
    verify_role(&report, "backup");
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn v1_journal_cannot_claim_v2_length_changing_evidence() {
    let fixture = Fixture::new(&[("song.flac", flac(true, None, false))], Metadata(Some("à, á")));
    let mut job = fixture.job();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, RepairJobStatus::Complete);
    drop(job);
    let (path, bytes) = journal(&fixture);
    let roots = fixture.roots();
    for replace_intent in [false, true] {
        let tampered = rewrite(&bytes, |body| {
            if body.phase == "Header" { body.payload["schema"] = json!(1); }
            if replace_intent && body.phase == "TempPrepared" {
                body.payload = body.payload["source"].clone();
            }
        });
        assert!(matches!(recover_repair_journal(&roots, &tampered, &AtomicBool::new(false)), Err(E::CorruptJournal)));
    }
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn foreign_collision_after_backup_never_overwrites_or_loses_shorter_stage() {
    let before = flac(true, None, false);
    let fixture = Fixture::new(&[("song.flac", before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let destination = fixture.source.join("song.flac");
    let result = job.run(&AtomicBool::new(false), &mut |p| {
        if p.phase == "original_backed_up" { std::fs::write(&destination, b"foreign owner").unwrap(); }
    }).unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery);
    assert_eq!(result.repaired_files, 0);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verify_role(&report, "backup");
    verify_role(&report, "staged");
    assert!(report.operations[0].evidence.iter().any(|e| e.role == "published_candidate" && matches!(e.status, RepairEvidenceStatus::Changed)));
    drop(job);
    assert_eq!(std::fs::read(destination).unwrap(), b"foreign owner");
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}
