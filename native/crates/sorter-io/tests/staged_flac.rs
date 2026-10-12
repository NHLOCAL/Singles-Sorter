use sha2::{Digest, Sha256};
use sorter_core::{
    RepairMode, fix_jibrish,
    fix_plan::{FIX_FIELDS, FieldRepair},
    metadata::LegacyField,
};
use sorter_io::{
    staged_flac::FlacStagedPlan,
    staged_tags::{StagedTagEvidence, TagWriteError},
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const VENDOR: &[u8] = b"Owned FLAC fixture vendor \xc2\xa9";
const BASE: &[u8] = include_bytes!("../../../tests/fixtures/media/blank.flac");
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn repair(field: LegacyField, original: &str) -> FieldRepair {
    FieldRepair { field, original: original.into(), replacement: fix_jibrish(original, RepairMode::Hebrew) }
}
fn block(kind: u8, body: &[u8], last: bool) -> Vec<u8> {
    assert!(body.len() <= 0x00ff_ffff);
    let mut out = vec![kind | if last { 128 } else { 0 }, (body.len() >> 16) as u8, (body.len() >> 8) as u8, body.len() as u8];
    out.extend_from_slice(body);
    out
}
fn vector(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend((bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}
fn comments(values: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    vector(&mut out, VENDOR);
    out.extend((values.len() as u32).to_le_bytes());
    for value in values { vector(&mut out, value.as_bytes()); }
    out
}
fn split(bytes: &[u8]) -> (Vec<Vec<u8>>, &[u8]) {
    assert_eq!(&bytes[..4], b"fLaC");
    let mut at = 4;
    let mut blocks = Vec::new();
    loop {
        let kind = bytes[at];
        let size = (usize::from(bytes[at + 1]) << 16) | (usize::from(bytes[at + 2]) << 8) | usize::from(bytes[at + 3]);
        let end = at + 4 + size;
        blocks.push(bytes[at..end].to_vec());
        at = end;
        if kind & 128 != 0 { break; }
    }
    (blocks, &bytes[at..])
}
fn audio() -> &'static [u8] { split(BASE).1 }
fn fixture(blocks: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"fLaC".to_vec();
    out.extend(block(0, &BASE[8..42], blocks.is_empty()));
    for (i, (kind, bytes)) in blocks.iter().enumerate() {
        out.extend(block(*kind, bytes, i + 1 == blocks.len()));
    }
    out.extend_from_slice(audio());
    out
}
fn inspect(bytes: &[u8], repairs: &[FieldRepair]) -> Result<FlacStagedPlan, TagWriteError> {
    FlacStagedPlan::inspect(&mut Cursor::new(bytes), bytes.len() as u64, repairs, &AtomicBool::new(false))
}
fn stage(bytes: &[u8], repairs: &[FieldRepair]) -> (Vec<u8>, StagedTagEvidence) {
    let plan = inspect(bytes, repairs).unwrap();
    let mut output = Vec::new();
    let evidence = plan.write_staged(&mut Cursor::new(bytes), &mut output, &hash(bytes), &AtomicBool::new(false)).unwrap();
    assert_eq!(plan.staged_length(), output.len() as u64);
    assert_eq!(evidence.staged_length, output.len() as u64);
    assert_eq!(evidence.source_sha256, hash(bytes));
    assert_eq!(evidence.staged_sha256, hash(&output));
    assert_eq!(evidence.media_sha256, hash(audio()));
    assert_eq!(evidence.media_length, audio().len() as u64);
    (output, evidence)
}
fn stored_comments(bytes: &[u8]) -> (Vec<u8>, Vec<String>) {
    let (blocks, _) = split(bytes);
    let body = &blocks.iter().find(|b| b[0] & 127 == 4).unwrap()[4..];
    let mut at = 0;
    fn word(bytes: &[u8], at: &mut usize) -> usize {
        let n = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
        *at += 4;
        n
    }
    let len = word(body, &mut at);
    let vendor = body[at..at + len].to_vec();
    at += len;
    let count = word(body, &mut at);
    let mut fields = Vec::new();
    for _ in 0..count {
        let len = word(body, &mut at);
        fields.push(String::from_utf8(body[at..at + len].to_vec()).unwrap());
        at += len;
    }
    assert_eq!(at, body.len());
    (vendor, fields)
}
fn seektable() -> Vec<u8> {
    let mut out = Vec::new();
    for (sample, offset, count) in [(0u64, 0u64, 4608u16), (4608, 11, 684)] {
        out.extend(sample.to_be_bytes());
        out.extend(offset.to_be_bytes());
        out.extend(count.to_be_bytes());
    }
    out
}
fn cuesheet() -> Vec<u8> {
    let mut out = vec![0; 395];
    out.push(2);
    for (sample, number, has_index) in [(0u64, 1, true), (5292, 255, false)] {
        out.extend(sample.to_be_bytes());
        out.push(number);
        out.extend([0; 26]);
        out.push(u8::from(has_index));
        if has_index {
            out.extend([0; 8]);
            out.push(1);
            out.extend([0; 3]);
        }
    }
    out
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

#[test]
fn removes_all_matching_duplicates_and_appends_in_original_five_field_order() {
    let before = fixture(&[(4, comments(&[
        "GENRE=à", "x-Custom=before", "ArTiSt=à", "TITLE=à", "ALBUMARTIST=à",
        "artist=á", "ALBUM=à", "X-custom=after", "TRACKNUMBER=03/12", "ALBUM ARTIST=untouched alias",
    ]))]);
    let repairs = FIX_FIELDS.iter().map(|f| repair(*f, if *f == LegacyField::Artist { "à, á" } else { "à" })).collect::<Vec<_>>();
    let (after, evidence) = stage(&before, &repairs);
    assert!(after.len() < before.len());
    assert_eq!(evidence.changed_fields, FIX_FIELDS);
    assert_eq!(stored_comments(&after), (VENDOR.to_vec(), vec![
        "x-Custom=before", "X-custom=after", "TRACKNUMBER=03/12", "ALBUM ARTIST=untouched alias",
        "artist=א, ב", "albumartist=א", "title=א", "album=א", "genre=א",
    ].into_iter().map(String::from).collect()));
    assert_eq!(split(&after).1, audio());
}

#[test]
fn no_padding_needed_and_all_other_block_bytes_survive_the_audio_offset_shift() {
    let before = fixture(&[
        (2, b"testprivate opaque bytes\0\xff".to_vec()),
        (3, seektable()), (6, picture()),
        (4, comments(&["Artist=à", "aRTIST=á", "X-Unknown=à\0retained"])),
        (5, cuesheet()), (126, b"reserved opaque\0\xff".to_vec()),
    ]);
    let (after, _) = stage(&before, &[repair(LegacyField::Artist, "à, á")]);
    let (old_blocks, old_audio) = split(&before);
    let (new_blocks, new_audio) = split(&after);
    assert!(after.len() < before.len());
    assert_eq!(old_audio, new_audio);
    assert_eq!(old_blocks.len(), new_blocks.len());
    for (old, new) in old_blocks.iter().zip(&new_blocks) {
        if old[0] & 127 != 4 { assert_eq!(old, new); }
    }
    // The second actual frame starts 11 bytes after the first in the frozen
    // 5292-sample recording, independently of the metadata prefix length.
    assert_eq!(&new_audio[11..13], &[0xff, 0xf8]);
    assert_eq!(stored_comments(&after).1, ["X-Unknown=à\0retained", "artist=א, ב"]);
}

#[test]
fn last_comment_flag_and_existing_padding_are_preserved() {
    for blocks in [
        vec![(4, comments(&["Artist=à"]))],
        vec![(4, comments(&["Artist=à"])), (1, vec![0; 29])],
    ] {
        let before = fixture(&blocks);
        let (after, _) = stage(&before, &[repair(LegacyField::Artist, "à")]);
        assert_eq!(before.len(), after.len());
        let (old, _) = split(&before);
        let (new, _) = split(&after);
        assert_eq!(old.iter().map(|b| b[0]).collect::<Vec<_>>(), new.iter().map(|b| b[0]).collect::<Vec<_>>());
        assert_eq!(stored_comments(&after).1, ["artist=א"]);
    }
}

#[test]
fn no_op_preserves_exact_vendor_comments_blocks_and_audio() {
    for before in [fixture(&[]), fixture(&[(4, comments(&["TITLE=", "TITLE=kept", "X~Key=valid RFC key", "Other=\0"]))]), BASE.to_vec()] {
        let plan = inspect(&before, &[]).unwrap();
        assert!(!plan.changes_bytes());
        let (after, evidence) = stage(&before, &[]);
        assert_eq!(before, after);
        assert!(evidence.changed_fields.is_empty());
    }
}

#[test]
fn raw_empty_duplicates_keep_released_projection_and_reject_lossy_reader_repairs() {
    let before = fixture(&[(4, comments(&["ARTIST=", "artist=à", "Artist="]))]);
    let (after, _) = stage(&before, &[repair(LegacyField::Artist, ", à, ")]);
    assert_eq!(stored_comments(&after).1, ["artist=, א, "]);
    assert!(matches!(inspect(&before, &[repair(LegacyField::Artist, "à")]), Err(TagWriteError::Stale)));
}

#[test]
fn stale_missing_nonpolicy_and_wrong_order_repairs_fail_preflight() {
    let before = fixture(&[(4, comments(&["Artist=à", "TITLE=á"]))]);
    for repairs in [
        vec![repair(LegacyField::Artist, "á")],
        vec![repair(LegacyField::Album, "à")],
        vec![repair(LegacyField::TrackNumber, "à")],
        vec![repair(LegacyField::Title, "á"), repair(LegacyField::Artist, "à")],
        vec![repair(LegacyField::Artist, "à"), repair(LegacyField::Artist, "à")],
        vec![FieldRepair { field: LegacyField::Artist, original: "à".into(), replacement: "arbitrary".into() }],
        vec![repair(LegacyField::Artist, "already Hebrew א")],
    ] { assert!(inspect(&before, &repairs).is_err()); }
    assert!(matches!(inspect(&fixture(&[]), &[repair(LegacyField::Artist, "à")]), Err(TagWriteError::Stale)));
    let nul = fixture(&[(4, comments(&["Artist=à\0á"]))]);
    assert!(matches!(inspect(&nul, &[repair(LegacyField::Artist, "à\0á")]), Err(TagWriteError::Unsupported(_))));
}

#[test]
fn malformed_comment_vectors_utf8_keys_counts_and_trailers_fail_before_output() {
    let valid = comments(&["Artist=à"]);
    let vendor_end = 4 + VENDOR.len();
    let mut cases = vec![vec![], vec![0; 7], comments(&["missing separator"]), comments(&["=empty key"]), comments(&["bad\nkey=value"])];
    let mut bad = valid.clone(); bad[4] = 0xff; cases.push(bad);
    let mut bad = valid.clone(); *bad.last_mut().unwrap() = 0xff; cases.push(bad);
    let mut bad = valid.clone(); bad[..4].copy_from_slice(&u32::MAX.to_le_bytes()); cases.push(bad);
    let mut bad = valid.clone(); bad[vendor_end..vendor_end + 4].copy_from_slice(&4097u32.to_le_bytes()); cases.push(bad);
    let mut bad = valid.clone(); bad[vendor_end + 4..vendor_end + 8].copy_from_slice(&u32::MAX.to_le_bytes()); cases.push(bad);
    let mut bad = valid.clone(); bad.push(1); cases.push(bad);
    for length in 0..valid.len() { cases.push(valid[..length].to_vec()); }
    for body in cases { assert!(matches!(inspect(&fixture(&[(4, body)]), &[]), Err(TagWriteError::Unsupported(_)))); }
}

#[test]
fn malformed_metadata_duplicate_blocks_wrappers_and_missing_audio_fail_preflight() {
    let before = fixture(&[(4, comments(&["Artist=à"]))]);
    let mut cases = vec![
        fixture(&[(4, comments(&[])), (4, comments(&[]))]),
        fixture(&[(0, BASE[8..42].to_vec())]),
        fixture(&[(127, vec![])]), fixture(&[(1, vec![1])]),
        fixture(&[(2, vec![1, 2, 3])]), fixture(&[(3, vec![0; 17])]),
        fixture(&[(3, vec![]), (3, vec![])]),
    ];
    let mut bad = before.clone(); bad[4] = 1; cases.push(bad);
    let mut bad = before.clone(); bad[7] = 33; cases.push(bad);
    let mut bad = before.clone(); bad[8..10].copy_from_slice(&0u16.to_be_bytes()); cases.push(bad);
    let mut bad = before.clone(); bad[43..46].copy_from_slice(&[255; 3]); cases.push(bad);
    let mut bad = before.clone(); bad.truncate(before.len() - audio().len()); cases.push(bad);
    let mut bad = before.clone(); let at = bad.len() - audio().len(); bad[at] = 0; cases.push(bad);
    for prefix in [b"OggS".as_slice(), b"ID3\x04\0\0\0\0\0\0".as_slice()] {
        let mut bad = prefix.to_vec(); bad.extend_from_slice(&before); cases.push(bad);
    }
    for (signature, tail) in [(b"TAG".as_slice(), 125), (b"APETAGEX".as_slice(), 24)] {
        let mut bad = before.clone(); bad.extend(signature); bad.extend(vec![0; tail]); cases.push(bad);
    }
    for bad in cases { assert!(inspect(&bad, &[]).is_err()); }
    let blocks = vec![(1, vec![]); 4096];
    assert!(matches!(inspect(&fixture(&blocks), &[]), Err(TagWriteError::Unsupported(_))));
}

#[test]
fn changed_header_length_tail_or_digest_never_returns_success_evidence() {
    let before = fixture(&[(4, comments(&["Artist=à"]))]);
    let flag = AtomicBool::new(false);
    let plan = inspect(&before, &[repair(LegacyField::Artist, "à")]).unwrap();
    let mut changed = before.clone(); changed[8] ^= 1;
    let mut output = Vec::new();
    assert_eq!(plan.write_staged(&mut Cursor::new(changed), &mut output, &hash(&before), &flag), Err(TagWriteError::Stale));
    assert!(output.is_empty());
    let mut changed = before.clone(); *changed.last_mut().unwrap() ^= 1;
    assert_eq!(plan.write_staged(&mut Cursor::new(changed), &mut Vec::new(), &hash(&before), &flag), Err(TagWriteError::Stale));
    let mut changed = before.clone(); changed.push(0);
    assert_eq!(plan.write_staged(&mut Cursor::new(changed), &mut Vec::new(), &hash(&before), &flag), Err(TagWriteError::Stale));
    assert_eq!(plan.write_staged(&mut Cursor::new(&before[..before.len() - 1]), &mut Vec::new(), &hash(&before), &flag), Err(TagWriteError::Io));
    assert_eq!(plan.write_staged(&mut Cursor::new(&before), &mut Vec::new(), &hash(b"wrong"), &flag), Err(TagWriteError::Stale));
    assert!(matches!(FlacStagedPlan::inspect(&mut Cursor::new(&before), before.len() as u64 + 1, &[], &flag), Err(TagWriteError::Stale)));
}

struct ShortReads(Cursor<Vec<u8>>);
impl Read for ShortReads {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let len = out.len().min(1);
        self.0.read(&mut out[..len])
    }
}
impl Seek for ShortReads {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> { self.0.seek(from) }
}
struct CancellingWriter<'a>(&'a AtomicBool, Vec<u8>);
impl Write for CancellingWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.1.extend_from_slice(bytes);
        self.0.store(true, Ordering::Release);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
struct FailingWriter;
impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> { Err(io::Error::other("injected owned stage error")) }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[test]
fn short_reads_io_failure_and_cancellation_produce_no_success_evidence() {
    let before = fixture(&[(4, comments(&["Artist=à"]))]);
    let repairs = [repair(LegacyField::Artist, "à")];
    let flag = AtomicBool::new(false);
    let mut short = ShortReads(Cursor::new(before.clone()));
    let plan = FlacStagedPlan::inspect(&mut short, before.len() as u64, &repairs, &flag).unwrap();
    let mut output = Vec::new();
    plan.write_staged(&mut short, &mut output, &hash(&before), &flag).unwrap();
    assert_eq!(output, stage(&before, &repairs).0);
    assert_eq!(plan.write_staged(&mut short, &mut FailingWriter, &hash(&before), &flag), Err(TagWriteError::Io));
    let mut cancelling = CancellingWriter(&flag, Vec::new());
    assert_eq!(plan.write_staged(&mut short, &mut cancelling, &hash(&before), &flag), Err(TagWriteError::Cancelled));
    assert!(!cancelling.1.is_empty());
    let mut output = Vec::new();
    assert_eq!(plan.write_staged(&mut short, &mut output, &hash(&before), &flag), Err(TagWriteError::Cancelled));
    assert!(output.is_empty());
    assert!(matches!(FlacStagedPlan::inspect(&mut short, before.len() as u64, &repairs, &flag), Err(TagWriteError::Cancelled)));
}

#[cfg(native_metadata_bridge)]
#[test]
fn real_native_reader_roundtrip_matches_all_five_assignments_and_untouched_track() {
    use sorter_io::{metadata::{LegacyScalar, MetadataReader}, storage::ReadInput};
    let before = fixture(&[
        (4, comments(&["ARTIST=à", "artist=á", "ALBUMARTIST=à", "TITLE=á", "ALBUM=â", "GENRE=ã", "TRACKNUMBER=03"])),
        (3, seektable()), (5, cuesheet()), (6, picture()),
    ]);
    let reader = |bytes: Vec<u8>| {
        let length = bytes.len() as u64;
        MetadataReader.open(ReadInput::seekable(Cursor::new(bytes), length), "owned.flac").unwrap()
    };
    let mut original = reader(before.clone());
    let repairs = FIX_FIELDS.iter().map(|field| {
        let LegacyScalar::Text(value) = original.get(*field).unwrap().value.unwrap() else { panic!("text field required") };
        repair(*field, &value)
    }).collect::<Vec<_>>();
    let (after, _) = stage(&before, &repairs);
    let mut repaired = reader(after);
    for repair in repairs {
        let observed = repaired.get(repair.field).unwrap();
        assert_eq!(observed.values, [LegacyScalar::Text(repair.replacement.clone())]);
        assert_eq!(observed.value.unwrap(), LegacyScalar::Text(repair.replacement));
    }
    assert_eq!(repaired.get(LegacyField::TrackNumber).unwrap(), original.get(LegacyField::TrackNumber).unwrap());
}

#[test]
fn shared_product_dispatch_matches_sealed_flac_stream_evidence() {
    use sorter_io::staged_repair::StagedRepairPlan;
    let before = fixture(&[(4, comments(&["Artist=à", "artist=á", "X-Unknown=retain"]))]);
    let repairs = [repair(LegacyField::Artist, "à, á")];
    let plan = StagedRepairPlan::inspect("owned.FLAC", &mut Cursor::new(&before), before.len() as u64, &repairs, &AtomicBool::new(false)).unwrap();
    assert!(plan.changes_bytes());
    let mut after = Vec::new();
    let evidence = plan.write_staged(&mut Cursor::new(&before), &mut after, &hash(&before), &AtomicBool::new(false)).unwrap();
    assert_eq!((after, evidence), stage(&before, &repairs));
    assert!(plan.staged_length() < before.len() as u64);
}

#[test]
fn product_dispatch_rejects_shifted_opaque_blocks_without_broadening_stream_claims() {
    use sorter_io::staged_repair::StagedRepairPlan;
    for kind in [2, 7, 126] {
        let before = fixture(&[(kind, b"testopaque bytes".to_vec()), (4, comments(&["Artist=à", "artist=á"]))]);
        let repairs = [repair(LegacyField::Artist, "à, á")];
        assert!(inspect(&before, &repairs).is_ok());
        assert!(matches!(StagedRepairPlan::inspect("owned.flac", &mut Cursor::new(&before), before.len() as u64, &repairs, &AtomicBool::new(false)), Err(TagWriteError::Unsupported("opaque FLAC metadata cannot be moved during product repair"))));
    }
}

#[test]
fn product_dispatch_accepts_equal_length_opaque_bytes_and_rejects_other_formats() {
    use sorter_io::staged_repair::StagedRepairPlan;
    let before = fixture(&[(2, b"testopaque bytes".to_vec()), (4, comments(&["Artist=à"]))]);
    let repairs = [repair(LegacyField::Artist, "à")];
    let plan = StagedRepairPlan::inspect("owned.flac", &mut Cursor::new(&before), before.len() as u64, &repairs, &AtomicBool::new(false)).unwrap();
    assert_eq!(plan.staged_length(), before.len() as u64);
    let mut after = Vec::new();
    plan.write_staged(&mut Cursor::new(&before), &mut after, &hash(&before), &AtomicBool::new(false)).unwrap();
    assert_eq!(split(&before).0[1], split(&after).0[1]);
    for filename in ["owned.ogg", "owned.mp3", "owned.flac.bak"] {
        assert!(StagedRepairPlan::inspect(filename, &mut Cursor::new(&before), before.len() as u64, &repairs, &AtomicBool::new(false)).is_err());
    }
}
