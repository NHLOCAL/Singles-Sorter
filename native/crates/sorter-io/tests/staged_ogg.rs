use sha2::{Digest, Sha256};
use sorter_core::{
    RepairMode, fix_jibrish,
    fix_plan::{FIX_FIELDS, FieldRepair},
    metadata::LegacyField,
};
use sorter_io::{
    staged_ogg::OggStagedPlan,
    staged_tags::{StagedTagEvidence, TagWriteError},
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "../../../tests/staged_ogg/fixture_support.rs"]
mod fixture;
use fixture::*;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn repair(field: LegacyField, original: &str) -> FieldRepair {
    FieldRepair {
        field,
        original: original.into(),
        replacement: fix_jibrish(original, RepairMode::Hebrew),
    }
}
fn inspect(bytes: &[u8], repairs: &[FieldRepair]) -> Result<OggStagedPlan, TagWriteError> {
    OggStagedPlan::inspect(
        &mut Cursor::new(bytes),
        bytes.len() as u64,
        repairs,
        &AtomicBool::new(false),
    )
}
fn stage(bytes: &[u8], repairs: &[FieldRepair]) -> (Vec<u8>, StagedTagEvidence) {
    let plan = inspect(bytes, repairs).unwrap();
    let mut after = Vec::new();
    let evidence = plan
        .write_staged(
            &mut Cursor::new(bytes),
            &mut after,
            &hash(bytes),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(plan.staged_length(), after.len() as u64);
    assert_eq!(evidence.staged_length, after.len() as u64);
    assert_eq!(evidence.source_sha256, hash(bytes));
    assert_eq!(evidence.staged_sha256, hash(&after));
    let payload: Vec<u8> = audio_pages(bytes)
        .into_iter()
        .flat_map(|p| p.data)
        .collect();
    assert_eq!(evidence.media_sha256, hash(&payload));
    assert_eq!(evidence.media_length, payload.len() as u64);
    assert_preserved(bytes, &after);
    (after, evidence)
}
fn rich(opus: bool) -> Vec<u8> {
    build(
        opus,
        &[
            "x-Custom=first",
            "GENRE=à",
            "ArTiSt=à",
            PICTURE,
            "ALBUM=à",
            "TITLE=à",
            "ALBUMARTIST=à",
            "artist=á",
            "X-custom=last",
            "TRACKNUMBER=03",
            "ALBUM ARTIST=untouched alias",
        ],
        if opus {
            b"\x01opaque trailer\0\xff"
        } else {
            b"\x81\0padding\0"
        },
        255,
    )
}
fn rich_repairs() -> Vec<FieldRepair> {
    FIX_FIELDS
        .iter()
        .map(|f| {
            repair(
                *f,
                if *f == LegacyField::Artist {
                    "à, á"
                } else {
                    "à"
                },
            )
        })
        .collect()
}
fn rebuild(mut p: Vec<Page>, index: usize, change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    change(&mut p[index].raw);
    reseal(&mut p[index].raw);
    p.into_iter().flat_map(|p| p.raw).collect()
}

#[test]
fn five_fields_preserve_vendor_unknown_artwork_packets_and_timing() {
    for opus in [false, true] {
        let before = rich(opus);
        let (after, evidence) = stage(&before, &rich_repairs());
        assert_eq!(evidence.changed_fields, FIX_FIELDS);
        let (vendor, values, suffix) = comments(&after);
        assert_eq!(vendor, VENDOR);
        assert_eq!(suffix, comments(&before).2);
        assert_eq!(
            values,
            [
                "x-Custom=first",
                PICTURE,
                "X-custom=last",
                "TRACKNUMBER=03",
                "ALBUM ARTIST=untouched alias",
                "artist=א, ב",
                "albumartist=א",
                "title=א",
                "album=א",
                "genre=א"
            ]
        );
    }
}
#[test]
fn frozen_ordinary_and_rich_no_op_preserve_exact_bytes() {
    for before in [VORBIS.to_vec(), OPUS.to_vec(), rich(false), rich(true)] {
        let plan = inspect(&before, &[]).unwrap();
        assert!(!plan.changes_bytes());
        let (after, evidence) = stage(&before, &[]);
        assert_eq!(after, before);
        assert!(evidence.changed_fields.is_empty());
    }
}
#[test]
fn continued_headers_and_exact_255_multiples_keep_terminating_zero_laces() {
    for opus in [false, true] {
        for target in [255, 65_025] {
            let empty = comment_packet(opus, &["ARTIST=à", "X="], if opus { b"" } else { b"\x01" });
            let custom = format!("X={}", "x".repeat(target - empty.len()));
            let before = build(
                opus,
                &["ARTIST=à", &custom],
                if opus { b"" } else { b"\x01" },
                255,
            );
            assert_eq!(packets(&before)[1].len(), target);
            let (after, _) = stage(&before, &[repair(LegacyField::Artist, "à")]);
            assert_eq!(packets(&after)[1].len(), target);
            assert!(pages(&after).iter().any(|p| p.laces.last() == Some(&0)));
        }
    }
}
#[test]
fn header_page_count_shrink_resequences_only_audio_sequence_and_crc() {
    for opus in [false, true] {
        let long = format!("X={}", "x".repeat(1024));
        let before = build(
            opus,
            &["ArTiSt=à", "artist=á", &long],
            if opus { b"\x00padding" } else { b"\x01" },
            1,
        );
        let (after, _) = stage(&before, &[repair(LegacyField::Artist, "à, á")]);
        assert!(after.len() < before.len());
        assert!(pages(&after).len() < pages(&before).len());
        assert_ne!(
            audio_pages(&before)[0].sequence,
            audio_pages(&after)[0].sequence
        );
    }
}
#[test]
fn empty_duplicates_match_released_scalar_and_reject_lossy_native_projection() {
    for opus in [false, true] {
        let before = build(
            opus,
            &["ARTIST=", "Artist=à", "artist="],
            if opus { b"" } else { b"\x01" },
            255,
        );
        assert!(matches!(
            inspect(&before, &[repair(LegacyField::Artist, "à")]),
            Err(TagWriteError::Stale)
        ));
        let (after, _) = stage(&before, &[repair(LegacyField::Artist, ", à, ")]);
        assert_eq!(comments(&after).1, ["artist=, א, "]);
    }
}
#[test]
fn missing_stale_wrong_order_and_nonpolicy_assignments_reject() {
    for opus in [false, true] {
        let before = rich(opus);
        for repairs in [
            vec![repair(LegacyField::Artist, "à")],
            vec![repair(LegacyField::TrackNumber, "à")],
            vec![
                repair(LegacyField::Genre, "à"),
                repair(LegacyField::Title, "à"),
            ],
            vec![
                repair(LegacyField::Artist, "à, á"),
                repair(LegacyField::Artist, "à, á"),
            ],
            vec![FieldRepair {
                field: LegacyField::Artist,
                original: "à, á".into(),
                replacement: "arbitrary".into(),
            }],
        ] {
            assert!(inspect(&before, &repairs).is_err());
        }
        let missing = build(opus, &["X=à"], if opus { b"" } else { b"\x01" }, 255);
        assert!(matches!(
            inspect(&missing, &[repair(LegacyField::Artist, "à")]),
            Err(TagWriteError::Stale)
        ));
    }
}
#[test]
fn malformed_comment_utf8_names_lengths_count_framing_and_nul_reject() {
    for opus in [false, true] {
        for values in [vec!["=bad"], vec!["X~key=bad"], vec!["bad-no-equals"]] {
            let before = build(opus, &values, if opus { b"" } else { b"\x01" }, 255);
            assert!(inspect(&before, &[]).is_err());
        }
        let nul = build(opus, &["ARTIST=à\0"], if opus { b"" } else { b"\x01" }, 255);
        assert!(inspect(&nul, &[repair(LegacyField::Artist, "à\0")]).is_err());
        let source = build(opus, &["ARTIST=à"], if opus { b"" } else { b"\x01" }, 255);
        let parsed = pages(&source);
        let start = 27 + parsed[1].laces.len() + if opus { 8 } else { 7 };
        for changed in [
            rebuild(parsed.clone(), 1, |b| {
                b[start..start + 4].copy_from_slice(&u32::MAX.to_le_bytes())
            }),
            rebuild(parsed.clone(), 1, |b| b[start + 4] = 0xff),
            rebuild(parsed.clone(), 1, |b| {
                let at = start + 4 + VENDOR.len();
                b[at..at + 4].copy_from_slice(&4097u32.to_le_bytes());
            }),
        ] {
            assert!(inspect(&changed, &[]).is_err());
        }
    }
    for suffix in [b"".as_slice(), b"\0"] {
        assert!(inspect(&build(false, &["ARTIST=à"], suffix, 255), &[]).is_err());
    }
}
#[test]
fn bad_crc_sequence_serial_continuation_truncation_and_chaining_reject() {
    for opus in [false, true] {
        let before = rich(opus);
        let parsed = pages(&before);
        let last = parsed.len() - 1;
        let mut bad_crc = before.clone();
        *bad_crc.last_mut().unwrap() ^= 1;
        let mut chained = before.clone();
        chained.extend(&before);
        let mut trailing = before.clone();
        trailing.extend(b"tag mirror");
        for changed in [
            bad_crc,
            chained,
            trailing,
            before[..before.len() - 1].to_vec(),
            rebuild(parsed.clone(), last, |b| b[18] ^= 1),
            rebuild(parsed.clone(), last, |b| b[14] ^= 1),
            rebuild(parsed.clone(), last, |b| b[5] |= 1),
            rebuild(parsed.clone(), last, |b| b[5] |= 2),
            rebuild(parsed.clone(), last, |b| b[5] &= !4),
            rebuild(parsed.clone(), 1, |b| b[5] |= 8),
        ] {
            assert!(inspect(&changed, &[]).is_err());
        }
    }
}
#[test]
fn invalid_identification_headers_codec_and_granules_reject() {
    for opus in [false, true] {
        let before = rich(opus);
        let parsed = pages(&before);
        let last = parsed.len() - 1;
        let body = 27 + parsed[0].laces.len();
        for changed in [
            rebuild(parsed.clone(), 0, |b| b[body] = b'X'),
            rebuild(parsed.clone(), 0, |b| {
                b[body + if opus { 8 } else { 7 }] = 16
            }),
            rebuild(parsed.clone(), 0, |b| b[5] = 0),
            rebuild(parsed.clone(), 1, |b| b[6] = 1),
            rebuild(parsed.clone(), last, |b| b[6..14].fill(255)),
        ] {
            assert!(inspect(&changed, &[]).is_err());
        }
    }
}
#[test]
fn continued_audio_payload_lacing_and_granules_are_never_repaginated() {
    for opus in [false, true] {
        let before = rich(opus);
        let parsed = pages(&before);
        let last = parsed.last().unwrap();
        let audio_at = before.len() - last.raw.len();
        let mut source = before[..audio_at].to_vec();
        let data = vec![if opus { 0xf8 } else { 0 }; 510];
        source.extend(page(
            0,
            u64::MAX,
            last.serial,
            last.sequence,
            &[255],
            &data[..255],
        ));
        source.extend(page(
            1,
            u64::MAX,
            last.serial,
            last.sequence + 1,
            &[255],
            &data[255..],
        ));
        source.extend(page(5, 960, last.serial, last.sequence + 2, &[0], &[]));
        let (after, _) = stage(&source, &rich_repairs());
        assert_eq!(audio_pages(&after).len(), 3);
    }
}
#[test]
fn empty_audio_unterminated_packets_and_mixed_header_audio_pages_reject() {
    for opus in [false, true] {
        let before = rich(opus);
        let parsed = pages(&before);
        let last = parsed.last().unwrap();
        let prefix = &before[..before.len() - last.raw.len()];
        for tail in [
            page(4, 0, last.serial, last.sequence, &[0], &[]),
            page(4, u64::MAX, last.serial, last.sequence, &[255], &[0; 255]),
        ] {
            let mut source = prefix.to_vec();
            source.extend(tail);
            assert!(inspect(&source, &[]).is_err());
        }
        let h = &parsed[1];
        let mut laces = h.laces.clone();
        laces.extend(&last.laces);
        let mut data = h.data.clone();
        data.extend(&last.data);
        let mut mixed = parsed[0].raw.clone();
        mixed.extend(page(0, 0, h.serial, 1, &laces, &data));
        mixed.extend(&last.raw);
        assert!(inspect(&mixed, &[]).is_err());
    }
}
#[test]
fn stale_hash_length_header_tail_and_extra_bytes_never_return_success_evidence() {
    let before = rich(true);
    let plan = inspect(&before, &rich_repairs()).unwrap();
    let mut out = Vec::new();
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut out,
            &"0".repeat(64),
            &AtomicBool::new(false)
        ),
        Err(TagWriteError::Stale)
    );
    assert!(out.is_empty());
    let parsed = pages(&before);
    let last = parsed.len() - 1;
    let modified_tail = rebuild(parsed, last, |b| *b.last_mut().unwrap() ^= 1);
    for changed in [
        modified_tail,
        {
            let mut b = before.clone();
            b.push(0);
            b
        },
        before[..before.len() - 1].to_vec(),
        {
            let mut b = before.clone();
            b[45] ^= 1;
            b
        },
    ] {
        assert!(
            plan.write_staged(
                &mut Cursor::new(&changed),
                &mut Vec::new(),
                &hash(&before),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
struct ShortRead<'a> {
    cursor: Cursor<&'a [u8]>,
    flag: Option<&'a AtomicBool>,
    after: u64,
}
impl Read for ShortRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(7);
        let read = self.cursor.read(&mut buf[..len])?;
        if self.cursor.position() >= self.after
            && let Some(flag) = self.flag
        {
            flag.store(true, Ordering::Release);
        }
        Ok(read)
    }
}
impl Seek for ShortRead<'_> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(to)
    }
}
struct CancelWrite<'a> {
    flag: &'a AtomicBool,
}
impl Write for CancelWrite<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.flag.store(true, Ordering::Release);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn cancellation_during_inspection_and_staging_has_no_success_evidence() {
    let before = rich(false);
    let flag = AtomicBool::new(true);
    assert!(matches!(
        OggStagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &rich_repairs(),
            &flag
        ),
        Err(TagWriteError::Cancelled)
    ));
    flag.store(false, Ordering::Release);
    let mut reader = ShortRead {
        cursor: Cursor::new(&before),
        flag: Some(&flag),
        after: 65,
    };
    assert!(matches!(
        OggStagedPlan::inspect(&mut reader, before.len() as u64, &rich_repairs(), &flag),
        Err(TagWriteError::Cancelled)
    ));
    flag.store(false, Ordering::Release);
    let plan = inspect(&before, &rich_repairs()).unwrap();
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut CancelWrite { flag: &flag },
            &hash(&before),
            &flag
        ),
        Err(TagWriteError::Cancelled)
    );
}
#[test]
fn short_reads_succeed_and_failed_writes_are_not_evidence() {
    let before = rich(true);
    let flag = AtomicBool::new(false);
    let mut reader = ShortRead {
        cursor: Cursor::new(&before),
        flag: None,
        after: u64::MAX,
    };
    let plan =
        OggStagedPlan::inspect(&mut reader, before.len() as u64, &rich_repairs(), &flag).unwrap();
    let mut after = Vec::new();
    plan.write_staged(&mut reader, &mut after, &hash(&before), &flag)
        .unwrap();
    assert_preserved(&before, &after);
    struct Fails;
    impl Write for Fails {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("owned failure"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        plan.write_staged(&mut Cursor::new(&before), &mut Fails, &hash(&before), &flag),
        Err(TagWriteError::Io)
    );
}
#[test]
fn header_page_and_comment_count_budgets_fail_closed() {
    let large = format!("X={}", "x".repeat(255 * 4096));
    let before = build(true, &["ARTIST=à", &large], b"", 1);
    assert!(inspect(&before, &[]).is_err());
    let values = vec!["X=a"; 4097];
    assert!(inspect(&build(true, &values, b"", 255), &[]).is_err());
}

fn ffmpeg(args: &[&std::ffi::OsStr]) -> Vec<u8> {
    let output = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin"])
        .args(args)
        .output()
        .expect("FFmpeg is required for this decoder gate");
    assert!(
        output.status.success(),
        "FFmpeg: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
fn decode(path: &std::path::Path) -> Vec<u8> {
    ffmpeg(&[
        "-i".as_ref(),
        path.as_os_str(),
        "-f".as_ref(),
        "s16le".as_ref(),
        "-acodec".as_ref(),
        "pcm_s16le".as_ref(),
        "-".as_ref(),
    ])
}
#[test]
fn ffmpeg_decodes_real_staged_vorbis_and_opus_to_identical_pcm() {
    let dir = tempfile::tempdir().unwrap();
    for (opus, extension, encoder) in [(false, "ogg", "libvorbis"), (true, "opus", "libopus")] {
        for generated in [false, true] {
            let input = dir.path().join(format!("before-{generated}.{extension}"));
            let before = if generated {
                ffmpeg(&[
                    "-f".as_ref(),
                    "lavfi".as_ref(),
                    "-i".as_ref(),
                    "sine=frequency=440:duration=1.2:sample_rate=48000".as_ref(),
                    "-c:a".as_ref(),
                    encoder.as_ref(),
                    "-metadata".as_ref(),
                    "artist=à".as_ref(),
                    "-y".as_ref(),
                    input.as_os_str(),
                ]);
                std::fs::read(&input).unwrap()
            } else {
                let bytes = rich(opus);
                std::fs::write(&input, &bytes).unwrap();
                bytes
            };
            let repairs = if generated {
                vec![repair(LegacyField::Artist, "à")]
            } else {
                rich_repairs()
            };
            let (after, _) = stage(&before, &repairs);
            let output = dir.path().join(format!("after-{generated}.{extension}"));
            std::fs::write(&output, after).unwrap();
            let original_pcm = decode(&input);
            assert!(!original_pcm.is_empty());
            assert_eq!(original_pcm, decode(&output));
        }
    }
}

#[cfg(native_metadata_bridge)]
#[test]
fn real_native_reader_roundtrip_matches_five_ogg_assignments_and_track() {
    use sorter_core::metadata::LegacyScalar;
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    assert!(MetadataReader::is_available());
    for opus in [false, true] {
        let before = rich(opus);
        let reader = |bytes: Vec<u8>| {
            let length = bytes.len() as u64;
            MetadataReader
                .open(
                    ReadInput::seekable(Cursor::new(bytes), length),
                    if opus { "owned.opus" } else { "owned.ogg" },
                )
                .unwrap()
        };
        let mut original = reader(before.clone());
        let repairs = FIX_FIELDS
            .iter()
            .map(|field| {
                let LegacyScalar::Text(value) = original.get(*field).unwrap().value.unwrap() else {
                    panic!("text");
                };
                repair(*field, &value)
            })
            .collect::<Vec<_>>();
        let (after, _) = stage(&before, &repairs);
        let mut changed = reader(after);
        for r in repairs {
            let observed = changed.get(r.field).unwrap();
            assert_eq!(observed.values, [LegacyScalar::Text(r.replacement.clone())]);
            assert_eq!(observed.value.unwrap(), LegacyScalar::Text(r.replacement));
        }
        assert_eq!(
            changed.get(LegacyField::TrackNumber).unwrap(),
            original.get(LegacyField::TrackNumber).unwrap()
        );
    }
}
