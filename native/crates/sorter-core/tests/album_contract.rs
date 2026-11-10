use serde_json::{Value, json};
use sorter_core::album::{AlbumTraceEvent, FolderObservation, analyze_album};
use sorter_core::discovery::EntryKind;
#[path = "support/album_fixture.rs"]
mod support;
use support::*;
#[test]
fn all_45_album_goldens_match_results_and_ordered_metadata_accesses() {
    let input: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/full_contract/inputs.json"
    ))
    .unwrap();
    let goldens: Vec<Value> = include_str!("../../../tests/fixtures/full_contract/cases.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let mut count = 0;
    for input in input["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["op"] == "album")
    {
        let golden = goldens.iter().find(|g| g["id"] == input["id"]).unwrap();
        let (folder, mut provider) = fixture(input, golden);
        let actual = analyze_album(&folder, &mut provider);
        assert_eq!(
            json!([
                actual.is_album,
                actual.should_process,
                actual.album_name,
                actual.artist_name
            ]),
            golden["result"],
            "{}",
            input["id"]
        );
        let expected: Vec<Value> = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"].as_str().unwrap().starts_with("metadata_"))
            .map(|e| {
                let mut e = e.clone();
                if e["kind"] == "metadata_open" {
                    e["source"] = e["source_observation"]["path"].clone();
                    e.as_object_mut().unwrap().remove("source_observation");
                    e.as_object_mut().unwrap().remove("provider");
                }
                e
            })
            .collect();
        assert_eq!(trace(&actual.trace), expected, "trace {}", input["id"]);
        count += 1;
    }
    assert_eq!(count, 45);
}
fn numbered(names: Vec<String>, tracks: Vec<Value>) -> sorter_core::album::AlbumDecision {
    let children: Vec<_> = names.iter().map(|n| entry(n, EntryKind::File)).collect();
    let mut p = Provider(
        names
            .into_iter()
            .zip(
                tracks
                    .into_iter()
                    .map(|t| json!({"artist":"artist", "tracknumber":t})),
            )
            .collect(),
    );
    analyze_album(
        &FolderObservation {
            entry: entry("album", EntryKind::Directory),
            direct_children: children,
        },
        &mut p,
    )
}
#[test]
fn unicode15_decimal_and_python_integer_edges() {
    for digits in [
        ["\u{1e4f1}", "\u{1e4f2}", "\u{1e4f3}"],
        ["١", "٢", "٣"],
        ["１", "２", "３"],
    ] {
        let expected = true;
        assert_eq!(
            numbered(
                digits.iter().map(|s| format!("{s}.mp3")).collect(),
                vec![json!(""); 3]
            )
            .is_album,
            expected
        );
    }
    assert!(
        numbered(
            vec!["a.mp3".into(), "b.mp3".into(), "c.mp3".into()],
            vec![json!(" +١/9"), json!("0_2"), json!("３\u{a0}")]
        )
        .is_album
    );
    assert!(
        !numbered(
            vec!["a.mp3".into(), "b.mp3".into(), "c.mp3".into()],
            vec![json!("1"), json!("2"), json!("9".repeat(4300))]
        )
        .is_album
    );
    let a = numbered(
        vec![
            "1.mp3".into(),
            "2.mp3".into(),
            format!("{}.mp3", "9".repeat(4301)),
        ],
        vec![json!(""); 3],
    );
    assert!(!a.is_album);
    assert!(
        a.trace
            .iter()
            .any(|e| matches!(e, AlbumTraceEvent::FileError { kind, .. } if kind == "ValueError"))
    );
}
#[test]
fn stateful_second_get_is_used_and_first_value_is_not_evaluated() {
    use sorter_core::metadata::*;
    struct Changing {
        calls: usize,
    }
    impl MetadataFields for Changing {
        fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
            if field == LegacyField::Album {
                self.calls += 1;
                Ok(LegacyItem {
                    truthy: true,
                    values: vec![],
                    value: if self.calls == 1 {
                        Err(FieldError {
                            stage: ErrorStage::Value,
                            kind: "ValueError".into(),
                            detail: "must not be consumed".into(),
                        })
                    } else {
                        Ok(LegacyScalar::Text("album".into()))
                    },
                })
            } else {
                Ok(LegacyItem {
                    truthy: false,
                    values: vec![],
                    value: Err(FieldError::get("ValueError", "must not be consumed")),
                })
            }
        }
    }
    struct ChangingProvider;
    impl MetadataProvider for ChangingProvider {
        fn open(
            &mut self,
            _: &sorter_core::discovery::EntryObservation,
        ) -> Result<Box<dyn MetadataFields>, MetadataError> {
            Ok(Box::new(Changing { calls: 0 }))
        }
    }
    let folder = FolderObservation {
        entry: entry("album", EntryKind::Directory),
        direct_children: ["a.mp3", "b.mp3", "c.mp3"]
            .iter()
            .map(|s| entry(s, EntryKind::File))
            .collect(),
    };
    let a = analyze_album(&folder, &mut ChangingProvider);
    assert!(a.is_album);
    assert!(!a.should_process);
    assert_eq!(a.album_name.as_deref(), Some("album"));
    assert!(
        !a.trace
            .iter()
            .any(|e| matches!(e, AlbumTraceEvent::FileError { .. }))
    );
}
#[test]
fn audio_suffix_and_directory_guards() {
    use sorter_core::album::is_audio;
    assert!(is_audio("song.MP3"));
    assert!(is_audio("..mp3"));
    assert!(!is_audio(".mp3"));
    assert!(!is_audio("song.mp3."));
    assert!(!is_audio("song.aif"));
    let folder = FolderObservation {
        entry: entry("album", EntryKind::File),
        direct_children: ["1.mp3", "2.mp3", "3.mp3"]
            .iter()
            .map(|s| entry(s, EntryKind::File))
            .collect(),
    };
    assert!(
        analyze_album(&folder, &mut Provider(Default::default()))
            .trace
            .is_empty()
    );
}
