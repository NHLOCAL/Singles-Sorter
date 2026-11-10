use sha2::{Digest, Sha256};
use sorter_core::{RepairMode, fix_jibrish, fix_plan::FieldRepair, metadata::LegacyField};
use sorter_io::{staged_ogg::OggStagedPlan, staged_repair::StagedRepairPlan};
use std::{io::Cursor, sync::atomic::AtomicBool};
#[path = "../../../tests/staged_ogg/fixture_support.rs"]
mod fixture;

fn repair(original: &str) -> FieldRepair {
    FieldRepair {
        field: LegacyField::Artist,
        original: original.into(),
        replacement: fix_jibrish(original, RepairMode::Hebrew),
    }
}
#[test]
fn shared_dispatch_matches_ogg_stream_bytes_and_all_evidence() {
    for opus in [false, true] {
        let before = fixture::build(
            opus,
            &["ARTIST=à", "artist=á", "X=retain", fixture::PICTURE],
            if opus {
                b"\x01binary\xff"
            } else {
                b"\x81padding"
            },
            255,
        );
        let repairs = [repair("à, á")];
        let cancel = AtomicBool::new(false);
        let hash = format!("{:x}", Sha256::digest(&before));
        let stream = OggStagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &repairs,
            &cancel,
        )
        .unwrap();
        let plan = StagedRepairPlan::inspect(
            if opus { "owned.OPUS" } else { "owned.OGG" },
            &mut Cursor::new(&before),
            before.len() as u64,
            &repairs,
            &cancel,
        )
        .unwrap();
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        let evidence = stream
            .write_staged(&mut Cursor::new(&before), &mut expected, &hash, &cancel)
            .unwrap();
        assert_eq!(
            plan.write_staged(&mut Cursor::new(&before), &mut actual, &hash, &cancel)
                .unwrap(),
            evidence
        );
        assert_eq!(actual, expected);
        assert_eq!(plan.staged_length(), actual.len() as u64);
        assert!(plan.changes_bytes());
    }
}
#[test]
fn dispatcher_allows_necessary_vorbis_wrapper_growth_and_opus_identity_length() {
    for opus in [false, true] {
        let before = fixture::build(opus, &["ARTIST=à"], if opus { b"" } else { b"\x01" }, 255);
        let plan = StagedRepairPlan::inspect(
            if opus { "owned.opus" } else { "owned.ogg" },
            &mut Cursor::new(&before),
            before.len() as u64,
            &[repair("à")],
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            plan.staged_length(),
            before.len() as u64 + if opus { 0 } else { 27 }
        );
    }
}
#[test]
fn extensions_do_not_claim_other_ogg_codecs_or_disguised_containers() {
    let before = fixture::build(true, &["ARTIST=à"], b"", 255);
    for name in [
        "owned.oga",
        "owned.wav",
        "owned.ogg.bak",
        "owned.flac",
        "owned.mp3",
    ] {
        assert!(
            StagedRepairPlan::inspect(
                name,
                &mut Cursor::new(&before),
                before.len() as u64,
                &[repair("à")],
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    for signature in [b"Speex   ".as_slice(), b"\x7fFLAC\x01\0\0", b"\x80theora\0"] {
        let mut parsed = fixture::pages(&before);
        let body = 27 + parsed[0].laces.len();
        parsed[0].raw[body..body + 8].copy_from_slice(signature);
        fixture::reseal(&mut parsed[0].raw);
        let unsupported: Vec<u8> = parsed.into_iter().flat_map(|p| p.raw).collect();
        assert!(
            StagedRepairPlan::inspect(
                "owned.ogg",
                &mut Cursor::new(&unsupported),
                unsupported.len() as u64,
                &[repair("à")],
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
