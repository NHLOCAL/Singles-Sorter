use serde_json::{Value, json};
use sorter_core::fix_plan::*;
use sorter_core::{discovery::*, metadata::*};
use std::{cell::RefCell, rc::Rc};

fn entry(name: &str) -> EntryObservation {
    EntryObservation {
        reference: StorageRef::TestOpaque { id: name.into() },
        identity: Some(ObjectIdentity::Test { id: name.into() }),
        parent_identity: None,
        name: name.rsplit('/').next().unwrap().into(),
        kind: EntryKind::File,
        size: Some(52),
        modified: None,
        capabilities: Capabilities {
            read: Some(true),
            list: Some(false),
            seek: Some(true),
        },
    }
}
struct Provider {
    data: Value,
    opened: Rc<RefCell<Vec<StorageRef>>>,
    gets: Rc<RefCell<Vec<LegacyField>>>,
}
impl Provider {
    fn new(data: Value) -> Self {
        Self {
            data,
            opened: Default::default(),
            gets: Default::default(),
        }
    }
}
struct Fields {
    data: Value,
    gets: Rc<RefCell<Vec<LegacyField>>>,
}
fn key(field: LegacyField) -> &'static str {
    match field {
        LegacyField::Artist => "artist",
        LegacyField::AlbumArtist => "albumartist",
        LegacyField::Title => "title",
        LegacyField::Album => "album",
        LegacyField::Genre => "genre",
        LegacyField::TrackNumber => "tracknumber",
    }
}
impl MetadataProvider for Provider {
    fn open(&mut self, entry: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        self.opened.borrow_mut().push(entry.reference.clone());
        if let Some(error) = self.data.get("open_error") {
            return Err(MetadataError::LegacyOpen {
                kind: error.as_str().unwrap().into(),
                detail: String::new(),
            });
        }
        Ok(Box::new(Fields {
            data: self.data.clone(),
            gets: self.gets.clone(),
        }))
    }
}
impl MetadataFields for Fields {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        self.gets.borrow_mut().push(field);
        let data = self.data.get(key(field)).cloned().unwrap_or(json!(""));
        if let Some(error) = data.get("get_error") {
            return Err(FieldError::get(error.as_str().unwrap(), "synthetic"));
        }
        let value = if let Some(error) = data.get("value_error") {
            Err(FieldError {
                stage: ErrorStage::Value,
                kind: error.as_str().unwrap().into(),
                detail: "synthetic".into(),
            })
        } else if let Some(n) = data.as_i64() {
            Ok(LegacyScalar::Integer(n.to_string()))
        } else {
            Ok(LegacyScalar::Text(data.as_str().unwrap_or("").into()))
        };
        Ok(LegacyItem {
            values: vec![],
            truthy: false,
            value,
        }) // fix uses value truthiness, never item truthiness
    }
}

#[test]
fn frozen_fix_contracts() {
    let inputs: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/full_contract/inputs.json"
    ))
    .unwrap();
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/full_contract/cases.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        if case["op"] != "fix" {
            continue;
        }
        count += 1;
        let input = inputs["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == case["id"])
            .unwrap();
        let mut occupied: Vec<String> = case["before"]["source"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["path"].as_str().unwrap().into())
            .collect();
        let trace = case["ordered_trace"].as_array().unwrap();
        let mut pending_rename: Option<(String, String)> = None;
        for (i, event) in trace.iter().enumerate() {
            if event["kind"] == "rename" {
                pending_rename = Some((
                    event["source_observation"]["path"].as_str().unwrap().into(),
                    event["destination"].as_str().unwrap().into(),
                ));
            }
            if event["kind"] != "metadata_open" {
                continue;
            }
            let final_path = event["source_observation"]["path"].as_str().unwrap();
            let (original, intended) = pending_rename
                .take()
                .unwrap_or((final_path.into(), final_path.into()));
            let source = entry(&original);
            let parent = original.rsplit_once('/').unwrap().0;
            let names: Vec<String> = occupied
                .iter()
                .filter_map(|p| {
                    p.rsplit_once('/')
                        .filter(|(dir, _)| *dir == parent)
                        .map(|(_, name)| name.into())
                })
                .collect();
            let file = input["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| format!("source/{}", f["path"].as_str().unwrap()) == original)
                .unwrap();
            let mut provider = Provider::new(file.get("metadata").cloned().unwrap_or(json!({})));
            let plan = fix_file(&source, &names, &mut provider);
            assert_eq!(
                plan.intended_name.as_deref(),
                Some(intended.rsplit('/').next().unwrap()),
                "{}",
                case["id"]
            );
            assert_eq!(
                provider.opened.borrow().as_slice(),
                std::slice::from_ref(&source.reference)
            );
            assert_eq!(plan.source, source);
            assert!(!plan.executable);
            let segment: Vec<_> = trace[i + 1..]
                .iter()
                .take_while(|x| !["metadata_open", "rename"].contains(&x["kind"].as_str().unwrap()))
                .collect();
            let expected_gets: Vec<_> = segment
                .iter()
                .filter(|x| x["kind"] == "metadata_item")
                .map(|x| x["field"].as_str().unwrap())
                .collect();
            assert_eq!(
                provider
                    .gets
                    .borrow()
                    .iter()
                    .map(|f| key(*f))
                    .collect::<Vec<_>>(),
                expected_gets,
                "{}",
                case["id"]
            );
            let expected_repairs: Vec<_> = segment
                .iter()
                .filter(|x| x["kind"] == "metadata_set")
                .map(|x| {
                    (
                        x["field"].as_str().unwrap(),
                        x["result"]["text"].as_str().unwrap(),
                    )
                })
                .collect();
            assert_eq!(
                plan.repairs
                    .iter()
                    .map(|r| (key(r.field), r.replacement.as_str()))
                    .collect::<Vec<_>>(),
                expected_repairs,
                "{}",
                case["id"]
            );
            assert_eq!(
                plan.save_intended,
                segment
                    .iter()
                    .any(|x| x["kind"] == "metadata_save_in_memory"),
                "{}",
                case["id"]
            );
            if original != intended {
                occupied.retain(|p| *p != original);
                occupied.push(intended);
                assert!(
                    plan.blockers
                        .contains(&FixBlocker::RenameMustSucceedAndRebindOriginal)
                );
            }
        }
    }
    assert_eq!(count, 7);
}

#[test]
fn exceptions_stop_later_fields_and_save_but_keep_prior_intentions() {
    for marker in ["get_error", "value_error"] {
        let mut p = Provider::new(
            json!({ "artist": "à", "albumartist": { marker: "ValueError" }, "title": "à" }),
        );
        let plan = fix_file(&entry("a_b.mp3"), &[], &mut p);
        assert_eq!(plan.repairs.len(), 1);
        assert!(!plan.save_intended);
        assert!(plan.failure.is_some());
        assert_eq!(p.gets.borrow().len(), 2);
    }
}

#[test]
fn unsafe_names_remain_visible_and_empty_names_skip_metadata() {
    for name in ["CON.mp3", "name.mp3.", "nul\0.mp3"] {
        let mut p = Provider::new(json!({}));
        let plan = fix_file(&entry(name), &[], &mut p);
        assert_eq!(plan.intended_name.as_deref(), Some(name));
        assert!(plan.blockers.contains(&FixBlocker::UnsafeIntendedName));
    }
    let mut p = Provider::new(json!({}));
    let plan = fix_file(&entry("???"), &[], &mut p);
    assert!(plan.intended_name.is_none());
    assert!(p.opened.borrow().is_empty());
    assert!(!plan.save_intended);
}

#[test]
fn scalar_truthiness_and_integer_iteration_follow_python_boundary() {
    for n in [0, 1] {
        let mut p = Provider::new(json!({"artist": n}));
        let plan = fix_file(&entry("àìáåí.mp3"), &[], &mut p);
        assert_eq!(plan.save_intended, n == 0);
        assert_eq!(plan.intended_name.as_deref(), Some("àìáåí.mp3"));
    }
}

#[test]
fn renamed_android_file_keeps_opaque_identity_for_all_metadata_intentions() {
    let mut source = entry("bad_name.mp3");
    source.reference = StorageRef::AndroidDocument {
        tree_uri: "content://tree".into(),
        authority: "test".into(),
        document_id: "opaque:not/a/path".into(),
    };
    let mut p = Provider::new(json!({"artist":"à"}));
    let plan = fix_file(
        &source,
        &["bad name.mp3".into(), "bad name_1.mp3".into()],
        &mut p,
    );
    assert_eq!(plan.intended_name.as_deref(), Some("bad name_2.mp3"));
    assert_eq!(p.opened.borrow().as_slice(), &[source.reference.clone()]);
    assert_eq!(plan.source, source);
    assert!(
        plan.blockers
            .contains(&FixBlocker::RenameMustSucceedAndRebindOriginal)
    );
    assert!(plan.save_intended);
    assert!(!plan.executable);
}

#[test]
fn collision_suffix_preserves_python_leading_dot_and_last_extension_rules() {
    for (original, occupied, expected) in [
        ("._foo", ". foo", ". foo_1"),
        ("a_b.tar.mp3", "a b.tar.mp3", "a b.tar_1.mp3"),
        ("a_b.", "a b.", "a b._1"),
    ] {
        let plan = fix_file(
            &entry(original),
            &[occupied.into()],
            &mut Provider::new(json!({})),
        );
        assert_eq!(plan.intended_name.as_deref(), Some(expected));
    }
}
