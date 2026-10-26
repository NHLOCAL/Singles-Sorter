use sorter_io::metadata::{ErrorStage, LegacyField, LegacyScalar, RawFamily, RawValue, project};
#[test]
fn independent_truthiness_and_lazy_numeric_errors() {
    let strings = project(
        RawFamily::Id3,
        LegacyField::Artist,
        vec![RawValue::Text("".into()), RawValue::Text("".into())],
    )
    .unwrap();
    assert!(!strings.truthy);
    assert_eq!(strings.value.unwrap(), LegacyScalar::Text(", ".into()));
    let multi = project(
        RawFamily::Xiph,
        LegacyField::TrackNumber,
        vec![RawValue::Text("1".into()), RawValue::Text("2".into())],
    )
    .unwrap();
    assert_eq!(multi.values.len(), 2);
    assert_eq!(multi.value.unwrap_err().stage, ErrorStage::Value);
    let e = project(
        RawFamily::Id3,
        LegacyField::TrackNumber,
        vec![RawValue::Text("1".into()), RawValue::Text("2".into())],
    )
    .unwrap_err();
    assert_eq!(e.stage, ErrorStage::Get);
}
#[test]
fn arbitrary_precision_unicode_and_missing_values() {
    for (raw, expected) in [
        ("１２", "12"),
        ("-000１２", "-12"),
        ("1_000", "1000"),
        ("0", "0"),
    ] {
        let item = project(
            RawFamily::Xiph,
            LegacyField::TrackNumber,
            vec![RawValue::Text(raw.into())],
        )
        .unwrap();
        assert_eq!(item.value.unwrap(), LegacyScalar::Integer(expected.into()));
    }
    let huge = "9".repeat(100);
    assert_eq!(
        project(
            RawFamily::Xiph,
            LegacyField::TrackNumber,
            vec![RawValue::Text(huge.clone())]
        )
        .unwrap()
        .value
        .unwrap(),
        LegacyScalar::Integer(huge)
    );
    assert_eq!(
        project(RawFamily::Id3, LegacyField::TrackNumber, vec![])
            .unwrap()
            .value
            .unwrap(),
        LegacyScalar::Integer("0".into())
    );
    assert_eq!(
        project(RawFamily::Id3, LegacyField::Artist, vec![])
            .unwrap()
            .value
            .unwrap(),
        LegacyScalar::Text("".into())
    );
}
#[test]
fn invalid_ffi_lengths_discriminants_and_utf8_never_panic() {
    use sorter_io::metadata::decode_raw;
    for size in 0..128 {
        assert!(decode_raw(&vec![255; size]).is_err());
    }
    for bytes in [
        vec![1, 0, 0, 0, 1, 0, 0, 0],
        vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255],
        vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 255],
    ] {
        assert!(decode_raw(&bytes).is_err());
    }
    assert!(decode_raw(&[1, 0, 0, 0, 0, 0, 0, 0, 1]).is_err());
    assert_eq!(
        decode_raw(&[1, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        (RawFamily::Id3, vec![])
    );
    let mut seed = 1729u64;
    for _ in 0..20_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bytes = vec![0; (seed % 256) as usize];
        for b in &mut bytes {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *b = (seed >> 32) as u8;
        }
        let _ = decode_raw(&bytes);
    }
}
#[test]
fn null_numeric_value_and_ape_string_are_not_normalized() {
    let null = project(
        RawFamily::Xiph,
        LegacyField::TrackNumber,
        vec![RawValue::Null],
    )
    .unwrap();
    assert!(!null.truthy);
    assert_eq!(null.value.unwrap(), LegacyScalar::Null);
    let ape = project(
        RawFamily::Ape,
        LegacyField::Artist,
        vec![RawValue::Text("a".into()), RawValue::Text("b".into())],
    )
    .unwrap();
    assert_eq!(ape.values, vec![LegacyScalar::Text("a\0b".into())]);
}
#[test]
fn raw_id3_preserves_empty_components_without_taglib_normalization() {
    use sorter_io::metadata::decode_raw;
    fn record(payload: &[u8]) -> Vec<u8> {
        let mut v = vec![1, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0];
        v.extend((payload.len() as u32).to_le_bytes());
        v.extend(payload);
        v
    }
    assert_eq!(
        decode_raw(&record(&[3, 0, 0])).unwrap().1,
        vec![RawValue::Text("".into()), RawValue::Text("".into())]
    );
    assert_eq!(
        decode_raw(&record(&[3, 0, b'a', 0])).unwrap().1,
        vec![RawValue::Text("".into()), RawValue::Text("a".into())]
    );
    assert!(decode_raw(&record(&[3, 255])).is_err());
    assert!(decode_raw(&record(&[1, 255, 254, 0])).is_err());
}
#[cfg(native_metadata_bridge)]
#[test]
fn tag_only_id3_is_not_a_successful_recording() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    use std::io::Cursor;
    for bytes in [
        b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec(),
        b"ID3\x03\x00\x00\x00\x00\x00\x00".to_vec(),
    ] {
        assert!(
            MetadataReader
                .open(
                    ReadInput::seekable(Cursor::new(bytes.clone()), bytes.len() as u64),
                    "looks-like.mp3"
                )
                .is_err()
        );
    }
}
#[test]
fn streams_require_an_explicit_seekable_adapter() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::io::Cursor;
    let input = ReadInput::streaming(Cursor::new(vec![0; 20]), 20);
    assert!(matches!(
        MetadataReader.open(input, "anything.flac"),
        Err(MetadataError::SeekRequired)
    ));
}
#[cfg(native_metadata_bridge)]
#[test]
fn legacy_unsupported_is_header_based_and_field_errors_stay_local() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::{fs, io::Cursor, path::PathBuf};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media");
    for (name, kind) in [
        ("tagged.wma", "NotImplementedError"),
        ("blank.aac", "AACError"),
    ] {
        let bytes = fs::read(root.join(name)).unwrap();
        assert!(
            matches!(MetadataReader.open(ReadInput::seekable(Cursor::new(bytes.clone()),bytes.len()as u64),"renamed.mp3"),Err(MetadataError::LegacyOpen{kind:k,..}) if k==kind)
        );
    }
    let bytes = fs::read(root.join("malformed_track.mp3")).unwrap();
    let mut h = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(bytes.clone()), bytes.len() as u64),
            "שם 🎵.aac",
        )
        .ok()
        .unwrap();
    assert!(h.get(LegacyField::TrackNumber).is_err());
    assert_eq!(
        h.get(LegacyField::Title).unwrap().value.unwrap(),
        LegacyScalar::Text("Synthetic Title".into())
    );
}
#[cfg(not(native_metadata_bridge))]
#[test]
fn native_platform_bridge_is_explicitly_unavailable() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::io::Cursor;
    assert!(matches!(
        MetadataReader.open(
            ReadInput::seekable(Cursor::new(vec![0; 20]), 20),
            "test.mp3"
        ),
        Err(MetadataError::UnsupportedPlatform)
    ));
}

#[cfg(native_metadata_bridge)]
#[test]
fn panicking_rust_reader_is_contained_at_callback_boundary() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    use std::io::{self, Cursor, Read, Seek, SeekFrom};
    struct PanicsAfterHeader {
        cursor: Cursor<Vec<u8>>,
        reads: usize,
    }
    impl Read for PanicsAfterHeader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            self.reads += 1;
            if self.reads > 1 {
                panic!("synthetic provider panic");
            }
            self.cursor.read(out)
        }
    }
    impl Seek for PanicsAfterHeader {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.cursor.seek(pos)
        }
    }
    let reader = PanicsAfterHeader {
        cursor: Cursor::new(vec![0; 100]),
        reads: 0,
    };
    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        MetadataReader.open(ReadInput::seekable(reader, 100), "panic.mp3")
    }));
    assert!(observed.is_ok());
    assert!(observed.unwrap().is_err());
}

#[test]
fn bounded_header_retries_one_byte_reads_and_interrupted_reads() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::io::{self, Cursor, Read, Seek, SeekFrom};
    struct ShortReader {
        bytes: Cursor<Vec<u8>>,
        interrupt: bool,
        calls: usize,
    }
    impl Read for ShortReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.interrupt && self.calls % 2 == 1 {
                return Err(io::ErrorKind::Interrupted.into());
            }
            let limit = out.len().min(1);
            self.bytes.read(&mut out[..limit])
        }
    }
    impl Seek for ShortReader {
        fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(from)
        }
    }
    for interrupt in [false, true] {
        for (prefix, expected) in [
            (
                vec![0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11],
                "NotImplementedError",
            ),
            (vec![0xff, 0xf1], "AACError"),
        ] {
            for length in [prefix.len(), 20] {
                let mut bytes = prefix.clone();
                bytes.resize(length, 0);
                let reader = ShortReader {
                    bytes: Cursor::new(bytes),
                    interrupt,
                    calls: 0,
                };
                let result =
                    MetadataReader.open(ReadInput::seekable(reader, length as u64), "renamed.mp3");
                assert!(
                    matches!(result, Err(MetadataError::LegacyOpen { kind, .. }) if kind == expected),
                    "interrupt={interrupt}, length={length}"
                );
            }
        }
    }
    // Observed-length truncation remains a storage failure, even when the short
    // physical prefix would otherwise identify an intentionally unsupported format.
    let truncated = vec![0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11];
    assert!(matches!(
        MetadataReader.open(ReadInput::seekable(Cursor::new(truncated), 16), "x.wma"),
        Err(MetadataError::Storage(_))
    ));
}

#[cfg(native_metadata_bridge)]
#[test]
fn id3_component_preflight_rejects_before_native_audio_reads() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::{
        io::{self, Cursor, Read, Seek, SeekFrom},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    struct TagOnlyReads {
        bytes: Cursor<Vec<u8>>,
        tag_end: u64,
        escaped: Arc<AtomicBool>,
    }
    impl Read for TagOnlyReads {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.bytes.position() + out.len() as u64 > self.tag_end {
                self.escaped.store(true, Ordering::SeqCst);
                return Err(io::Error::other(
                    "native parser reached outside the preflight tag",
                ));
            }
            self.bytes.read(out)
        }
    }
    impl Seek for TagOnlyReads {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(p)
        }
    }
    fn sync(n: usize) -> [u8; 4] {
        [
            ((n >> 21) & 127) as u8,
            ((n >> 14) & 127) as u8,
            ((n >> 7) & 127) as u8,
            (n & 127) as u8,
        ]
    }
    for frame in [b"TPE1", b"TXXX"] {
        // Only 8,195 payload bytes; no memory stress. Unknown text fields must
        // not bypass preflight just because the six requested fields omit them.
        let mut payload = vec![3];
        for _ in 0..4097 {
            payload.extend_from_slice(b"x\0");
        }
        let mut body = frame.to_vec();
        body.extend(sync(payload.len()));
        body.extend([0, 0]);
        body.extend(payload);
        let mut tag = b"ID3\x04\x00\x00".to_vec();
        tag.extend(sync(body.len()));
        tag.extend(body);
        for container in ["mpeg", "wave", "aiff", "dsf"] {
            let mut bytes = match container {
                "wave" | "aiff" => {
                    let little = container == "wave";
                    let mut out = if little {
                        b"RIFF".to_vec()
                    } else {
                        b"FORM".to_vec()
                    };
                    let length = (tag.len() + 12) as u32;
                    out.extend(if little {
                        length.to_le_bytes()
                    } else {
                        length.to_be_bytes()
                    });
                    out.extend(if little { b"WAVE" } else { b"AIFF" });
                    out.extend(b"ID3 ");
                    let size = tag.len() as u32;
                    out.extend(if little {
                        size.to_le_bytes()
                    } else {
                        size.to_be_bytes()
                    });
                    out.extend(&tag);
                    out
                }
                "dsf" => {
                    let mut out = vec![0; 28];
                    out[..4].copy_from_slice(b"DSD ");
                    out[20..28].copy_from_slice(&28u64.to_le_bytes());
                    out.extend(&tag);
                    out
                }
                _ => tag.clone(),
            };
            let tag_end = bytes.len() as u64;
            if container == "mpeg" {
                bytes.extend([0; 256]);
            }
            let escaped = Arc::new(AtomicBool::new(false));
            let length = bytes.len() as u64;
            let input = ReadInput::seekable(
                TagOnlyReads {
                    bytes: Cursor::new(bytes),
                    tag_end,
                    escaped: escaped.clone(),
                },
                length,
            );
            let result = MetadataReader.open(input, "small-controlled-input");
            assert!(
                matches!(result, Err(MetadataError::InputBudget)),
                "{container}, frame={frame:?}"
            );
            assert!(
                !escaped.load(Ordering::SeqCst),
                "native file parser ran before budget rejection"
            );
        }
    }
}

#[cfg(native_metadata_bridge)]
#[test]
fn wav_info_preflight_does_not_enable_legacy_ignored_metadata() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    use std::{fs, io::Cursor, path::PathBuf};
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/blank.wav");
    let original = fs::read(path).unwrap();
    let mut bytes = original.clone();
    let title = b"must remain ignored\0";
    let mut info = b"INFOINAM".to_vec();
    info.extend((title.len() as u32).to_le_bytes());
    info.extend(title);
    if title.len() % 2 == 1 {
        info.push(0);
    }
    bytes.extend(b"LIST");
    bytes.extend((info.len() as u32).to_le_bytes());
    bytes.extend(info);
    let riff_length = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_length.to_le_bytes());
    let length = bytes.len() as u64;
    let mut handle = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "controlled.wav",
        )
        .ok()
        .unwrap();
    assert_eq!(
        handle.get(LegacyField::Title).unwrap().value.unwrap(),
        LegacyScalar::Text(String::new())
    );
}

#[cfg(native_metadata_bridge)]
#[test]
fn id3_ignored_frame_flags_and_structured_classes_fail_closed_at_open() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::io::Cursor;
    for (frame, flags) in [(b"TXXX", [0, 8]), (b"CHAP", [0, 0]), (b"SYLT", [0, 0])] {
        // Tiny raw envelope; no media generation or native allocation stress.
        let mut bytes = b"ID3\x04\x00\x00\x00\x00\x00\x0e".to_vec();
        bytes.extend(frame);
        bytes.extend([0, 0, 0, 4]);
        bytes.extend(flags);
        bytes.extend([3, b'x', 0, 0]);
        let length = bytes.len() as u64;
        assert!(matches!(
            MetadataReader.open(
                ReadInput::seekable(Cursor::new(bytes), length),
                "controlled.mp3"
            ),
            Err(MetadataError::UnsupportedClass)
        ));
    }
}

#[cfg(native_metadata_bridge)]
#[test]
fn native_parser_replays_preflight_metadata_instead_of_rereading_mutable_provider() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    use std::{
        fs,
        io::{self, Cursor, Read, Seek, SeekFrom},
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    struct ReadOnceMetadata {
        bytes: Cursor<Vec<u8>>,
        tag_end: u64,
        read_body: bool,
        repeated: Arc<AtomicBool>,
    }
    impl Read for ReadOnceMetadata {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let start = self.bytes.position();
            let end = start + out.len() as u64;
            // First 64 bytes are the small header probe. Every later body read
            // must use the already-validated snapshot rather than this provider.
            if start < self.tag_end && end > 64 {
                if self.read_body {
                    self.repeated.store(true, Ordering::SeqCst);
                    return Err(io::Error::other(
                        "provider metadata changed after preflight",
                    ));
                }
                self.read_body = true;
            }
            self.bytes.read(out)
        }
    }
    impl Seek for ReadOnceMetadata {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(p)
        }
    }
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/tagged.mp3");
    let bytes = fs::read(path).unwrap();
    let size = bytes[6..10]
        .iter()
        .fold(0u64, |n, b| (n << 7) | u64::from(*b));
    let repeated = Arc::new(AtomicBool::new(false));
    let length = bytes.len() as u64;
    let input = ReadInput::seekable(
        ReadOnceMetadata {
            bytes: Cursor::new(bytes),
            tag_end: 10 + size,
            read_body: false,
            repeated: repeated.clone(),
        },
        length,
    );
    let result = MetadataReader.open(input, "stable-observation.mp3");
    assert!(
        result.is_ok(),
        "validated metadata was reread during native parsing"
    );
    assert!(!repeated.load(Ordering::SeqCst));
}

#[cfg(native_metadata_bridge)]
#[test]
fn id3_native_aliases_cannot_bypass_preflight() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::{io::Cursor, path::PathBuf};
    let original = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/tagged.mp3"),
    )
    .unwrap();
    let size = original[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    for key in [b"TPE1", b"IPLS", b"IPL\0", b"IPL ", b"TCO\0", b"SLT "] {
        let mut body = key.to_vec();
        body.extend(8195u32.to_be_bytes());
        body.extend([0, 0, 0]);
        for _ in 0..4097 {
            body.extend(b"x\0");
        }
        let mut bytes = b"ID3\x03\0\0".to_vec();
        for shift in [21, 14, 7, 0] {
            bytes.push(((body.len() >> shift) & 127) as u8);
        }
        bytes.extend(body);
        bytes.extend(&original[10 + size..]);
        assert_eq!(bytes.len(), 9650);
        let length = bytes.len() as u64;
        let result =
            MetadataReader.open(ReadInput::seekable(Cursor::new(bytes), length), "alias.mp3");
        if key[3] == 0 || key[3] == b' ' {
            assert!(
                matches!(result, Err(MetadataError::UnsupportedClass)),
                "{key:?}"
            );
        } else {
            assert!(matches!(result, Err(MetadataError::InputBudget)), "{key:?}");
        }
    }
}

#[cfg(native_metadata_bridge)]
#[test]
fn invalid_mpeg_prefix_cannot_enable_unpreflighted_id3_discovery() {
    use sorter_io::{
        metadata::{MetadataError, MetadataReader},
        storage::ReadInput,
    };
    use std::{io::Cursor, path::PathBuf};
    let original = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/tagged.mp3"),
    )
    .unwrap();
    let mut bytes = vec![0xff, 0xe2, 0, 0];
    bytes.extend(original);
    assert_eq!(bytes.len(), 2761);
    let length = bytes.len() as u64;
    assert!(matches!(
        MetadataReader.open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "displaced.mp3"
        ),
        Err(MetadataError::UnsupportedClass)
    ));
}

#[cfg(native_metadata_bridge)]
#[test]
fn mpeg_fast_discovery_never_parses_displaced_id3_body() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    use std::{
        io::{self, Cursor, Read, Seek, SeekFrom},
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    struct NoDisplacedBody {
        bytes: Cursor<Vec<u8>>,
        parsed: Arc<AtomicBool>,
        tag_end: usize,
    }
    impl Read for NoDisplacedBody {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            // The first 64 bytes are replay-cached. Native ID3::Tag::read
            // then requests exactly the remaining displaced tag body.
            if self.bytes.position() == 64 && out.len() == self.tag_end - 64 {
                self.parsed.store(true, Ordering::SeqCst);
                return Err(io::Error::other("unpreflighted ID3 body"));
            }
            self.bytes.read(out)
        }
    }
    impl Seek for NoDisplacedBody {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(p)
        }
    }
    let original = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/tagged.mp3"),
    )
    .unwrap();
    // Plausible header bits, but no consistent next MPEG frame. Accurate
    // discovery scans to the ID3 at offset four; Fast discovery must not.
    let tag_end = 14
        + original[6..10]
            .iter()
            .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    let mut bytes = vec![0xff, 0xfb, 0x90, 0];
    bytes.extend(original);
    let length = bytes.len() as u64;
    let parsed = Arc::new(AtomicBool::new(false));
    let input = ReadInput::seekable(
        NoDisplacedBody {
            bytes: Cursor::new(bytes),
            parsed: parsed.clone(),
            tag_end,
        },
        length,
    );
    let mut handle = MetadataReader.open(input, "plausible-prefix.mp3").unwrap();
    assert!(handle.get(LegacyField::Title).unwrap().values.is_empty());
    assert!(!parsed.load(Ordering::SeqCst));
}
