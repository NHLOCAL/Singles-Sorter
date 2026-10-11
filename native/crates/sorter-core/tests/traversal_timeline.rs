use serde_json::{Value, json};
use sorter_core::{ClassifierFact, Settings, discovery::*, job_plan::*};
use std::collections::HashMap;
#[allow(dead_code)]
#[path = "support/album_fixture.rs"]
mod support;
use support::{Provider, entry};
fn id(r: &StorageRef) -> String {
    match r {
        StorageRef::TestOpaque { id } => id.clone(),
        _ => panic!(),
    }
}
#[derive(Clone, Default)]
struct Storage {
    entries: HashMap<String, EntryObservation>,
    listings: HashMap<String, Vec<String>>,
    reads: Vec<String>,
}
impl ReadStorage for Storage {
    fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
        self.entries
            .get(&id(r))
            .cloned()
            .ok_or_else(|| StorageError::new(StorageErrorKind::NotFound, "absent"))
    }
    fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        let path = id(r);
        self.reads.push(path.clone());
        Ok(self
            .listings
            .get(&path)
            .into_iter()
            .flatten()
            .map(|p| self.entries[p].clone())
            .collect())
    }
    fn open_read(&mut self, _: &StorageRef) -> Result<ReadInput, StorageError> {
        panic!("policy must delegate only metadata reads")
    }
}
struct Ai {
    profile: AiProfile,
    spec: Value,
}
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        self.profile.clone()
    }
    fn classify(&mut self, _: &str) -> Result<ClassifierFact, String> {
        Ok(if self.spec["accept"].as_bool().unwrap_or(false) {
            ClassifierFact::Accept
        } else {
            ClassifierFact::Reject
        })
    }
    fn ner(&mut self, s: &str) -> Result<Vec<String>, String> {
        Ok(self.spec["ner"][s]
            .as_array()
            .map(|a| a.iter().map(|s| s.as_str().unwrap().into()).collect())
            .unwrap_or_default())
    }
}
fn inputs() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/full_contract/inputs.json"
    ))
    .unwrap()
}
fn goldens() -> Vec<Value> {
    include_str!("../../../tests/fixtures/full_contract/cases.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
fn fixture(input: &Value, golden: &Value) -> (JobRequestV2, Storage, Provider, Ai) {
    let mut storage = Storage::default();
    for section in ["source", "target"] {
        for e in golden["before"][section].as_array().unwrap() {
            let path = e["path"].as_str().unwrap();
            let kind = if e["kind"] == "directory" {
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            let mut observation = entry(path, kind);
            observation.identity = Some(ObjectIdentity::Test { id: path.into() });
            observation.parent_identity = path
                .rsplit_once('/')
                .map(|(p, _)| ObjectIdentity::Test { id: p.into() });
            storage.entries.insert(path.into(), observation);
            if let Some((parent, _)) = path.rsplit_once('/') {
                storage
                    .listings
                    .entry(parent.into())
                    .or_default()
                    .push(path.into());
            }
        }
    }
    // The source read order is taken from immutable oracle observations, not sorted.
    let mut observed = std::collections::HashSet::new();
    for e in golden["ordered_trace"].as_array().unwrap() {
        if (e["kind"] == "glob" && e["pattern"] == "*") || e["kind"] == "iterdir" {
            let folder = e["folder"].as_str().unwrap();
            let children = e["children"].as_array().unwrap();
            // check_errors consumes only one item of iterdir via any(); this partial
            // trace is not a complete enumeration. Use the first full observed listing.
            if folder.starts_with("source")
                && children.len() == storage.listings.get(folder).map_or(0, Vec::len)
                && observed.insert(folder.to_string())
            {
                storage.listings.insert(
                    folder.into(),
                    children
                        .iter()
                        .map(|e| e["path"].as_str().unwrap().into())
                        .collect(),
                );
            }
        }
    }
    let mut ai_spec = input.get("ai").cloned().unwrap_or(json!({}));
    let mut enabled = input.get("ai").is_some();
    let mut metadata = HashMap::new();
    for f in input["files"].as_array().unwrap() {
        let path = f["path"].as_str().unwrap();
        metadata.insert(
            format!("source/{path}"),
            f.get("metadata").cloned().unwrap_or(json!({})),
        );
        if let Some(artists) = f.get("artists") {
            enabled = true;
            let name = path.rsplit('/').next().unwrap().trim_end_matches(".mp3");
            if ai_spec.get("ner").is_none() {
                ai_spec["ner"] = json!({});
            }
            ai_spec["ner"][name] = artists.clone();
        }
    }
    let profile = if enabled {
        AiProfile::Configured {
            bundle_id: "synthetic-reference".into(),
            available: true,
        }
    } else {
        AiProfile::Disabled
    };
    let mut rows = inputs()["default_aliases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            a.as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().into())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if let Some(csv) = input["personal_csv"].as_str() {
        for line in csv.lines() {
            rows.push(line.split(',').map(String::from).collect());
        }
    }
    let request = JobRequestV2 {
        mode: if input["op"] == "fix" {
            JobMode::FixNames
        } else {
            JobMode::Sort
        },
        source: StorageRef::TestOpaque {
            id: "source".into(),
        },
        target: Some(StorageRef::TestOpaque {
            id: "target".into(),
        }),
        settings: serde_json::from_value(input.get("settings").cloned().unwrap_or(json!({})))
            .unwrap(),
        alias_rows: rows,
        resource_ids: vec!["frozen-v14-test-aliases".into()],
        adapter_versions: vec!["synthetic-read-only-v1".into()],
        ai_profile: profile.clone(),
    };
    (
        request,
        storage,
        Provider(metadata),
        Ai {
            profile,
            spec: ai_spec,
        },
    )
}
#[test]
fn all_15_sort_goldens_match_decisions_transfer_order_and_counters() {
    let mut count = 0;
    for golden in goldens().into_iter().filter(|g| g["op"] == "sort") {
        let inp = inputs();
        let input = inp["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == golden["id"])
            .unwrap();
        let (request, mut storage, mut metadata, mut ai) = fixture(input, &golden);
        let plan = plan_job(&request, &mut storage, &mut metadata, &mut ai).unwrap();
        assert!(!plan.executable);
        let expected = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "album_decision" || e["kind"] == "single_decision")
            .map(|e| json!([e["kind"], e["source_observation"]["path"], e["result"]]))
            .collect::<Vec<_>>();
        let actual = plan
            .decisions
            .iter()
            .filter_map(|d| match d {
                DecisionV2::Album { read, decision } => Some(json!([
                    "album_decision",
                    id(&read.source.reference),
                    [
                        decision.is_album,
                        decision.should_process,
                        decision.album_name,
                        decision.artist_name
                    ]
                ])),
                DecisionV2::Single { read, artists } => Some(json!([
                    "single_decision",
                    id(&read.source.reference),
                    if artists.is_empty() {
                        Value::Null
                    } else {
                        json!(artists)
                    }
                ])),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "decisions {}", input["id"]);
        let expected = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| {
                matches!(e["kind"].as_str(), Some("rename" | "copy2" | "copytree"))
                    && e["source_observation"]["path"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("source"))
                    && e["destination"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("target"))
            })
            .map(|e| json!([e["source_observation"]["path"], e["destination"]]))
            .collect::<Vec<_>>();
        let actual = plan
            .operations
            .iter()
            .flat_map(|o| match &o.kind {
                OperationKind::AlbumTransfer { children, .. } => children
                    .iter()
                    .map(|c| {
                        json!([
                            id(&c.source.reference),
                            format!("target/{}", c.destination.components.join("/"))
                        ])
                    })
                    .collect(),
                OperationKind::SingleTransfer { transfer, .. } => vec![json!([
                    id(&transfer.source.reference),
                    format!("target/{}", transfer.destination.components.join("/"))
                ])],
                _ => vec![],
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "transfers {}", input["id"]);
        assert_eq!(
            serde_json::to_value(&plan.predicted_legacy_counters).unwrap(),
            golden["legacy_counters"],
            "counters {}",
            input["id"]
        );
        assert!(
            plan.operations
                .iter()
                .all(|o| o.stop_and_replan_on_effect_mismatch)
        );
        count += 1;
    }
    assert_eq!(count, 15);
}
#[test]
fn all_13_single_goldens_preserve_error_boundaries() {
    let mut count = 0;
    for golden in goldens().into_iter().filter(|g| g["op"] == "single") {
        let inp = inputs();
        let input = inp["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == golden["id"])
            .unwrap();
        let (request, _, mut metadata, mut ai) = fixture(input, &golden);
        let path = format!("source/{}", input["files"][0]["path"].as_str().unwrap());
        let result = single_decision(
            &entry(&path, EntryKind::File),
            &request.alias_rows,
            &mut metadata,
            &mut ai,
            !matches!(request.ai_profile, AiProfile::Disabled),
        );
        if golden["error"].is_null() {
            let result = result.unwrap();
            assert_eq!(
                if result.is_empty() {
                    Value::Null
                } else {
                    json!(result)
                },
                golden["result"],
                "{}",
                input["id"]
            );
        } else {
            assert!(result.is_err(), "{}", input["id"]);
        }
        count += 1;
    }
    assert_eq!(count, 13);
}
#[test]
fn moved_descendant_is_visited_but_unavailable_copy_retains_physical_binding() {
    for copy in [false, true] {
        let cases = goldens();
        let g = cases
            .iter()
            .find(|g| {
                g["id"]
                    == if copy {
                        "scan-album-nested-copy"
                    } else {
                        "scan-album-nested-move"
                    }
            })
            .unwrap();
        let inp = inputs();
        let i = inp["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == g["id"])
            .unwrap();
        let (r, mut s, mut m, mut a) = fixture(i, g);
        let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
        assert!(
            p.precollected_visits
                .iter()
                .any(|r| id(r) == "source/album/extra")
        );
        if copy {
            let single = p
                .operations
                .iter()
                .find(|o| matches!(o.kind, OperationKind::SingleTransfer { .. }))
                .unwrap();
            assert_eq!(single.depends_on, vec![0]);
            if let OperationKind::SingleTransfer { source, .. } = &single.kind {
                assert_eq!(id(&source.reference), "source/album/extra/אברהם פריד.mp3");
            }
        } else {
            assert!(p.decisions.iter().any(|d|matches!(d,DecisionV2::Vanished{read} if id(&read.source.reference)=="source/album/extra"&&read.depends_on==vec![0])));
            assert_eq!(p.operations.len(), 1);
        }
    }
}
#[test]
fn configured_unavailable_is_not_disabled() {
    let r = JobRequestV2 {
        mode: JobMode::Sort,
        source: StorageRef::TestOpaque {
            id: "source".into(),
        },
        target: None,
        settings: Settings::default(),
        alias_rows: vec![],
        resource_ids: vec![],
        adapter_versions: vec![],
        ai_profile: AiProfile::Configured {
            bundle_id: "missing".into(),
            available: false,
        },
    };
    let mut ai = Ai {
        profile: r.ai_profile.clone(),
        spec: json!({}),
    };
    assert_eq!(
        plan_job(
            &r,
            &mut Storage::default(),
            &mut Provider(HashMap::new()),
            &mut ai
        )
        .unwrap_err()
        .code,
        "RECOGNITION_UNAVAILABLE"
    );
}
fn scenario(files: &[(&str, Value)], settings: Settings) -> (JobRequestV2, Storage, Provider, Ai) {
    let mut s = Storage::default();
    for root in ["source", "target"] {
        let mut e = entry(root, EntryKind::Directory);
        e.identity = Some(ObjectIdentity::Test { id: root.into() });
        s.entries.insert(root.into(), e);
    }
    let mut metadata = HashMap::new();
    for (name, data) in files {
        let path = format!("source/{name}");
        let components = path.split('/').collect::<Vec<_>>();
        for len in 2..=components.len() {
            let p = components[..len].join("/");
            if !s.entries.contains_key(&p) {
                let parent = components[..len - 1].join("/");
                let mut e = entry(
                    &p,
                    if len == components.len() {
                        EntryKind::File
                    } else {
                        EntryKind::Directory
                    },
                );
                e.identity = Some(ObjectIdentity::Test { id: p.clone() });
                e.parent_identity = Some(ObjectIdentity::Test { id: parent.clone() });
                s.entries.insert(p.clone(), e);
                s.listings.entry(parent).or_default().push(p);
            }
        }
        metadata.insert(path, data.clone());
    }
    let r = JobRequestV2 {
        mode: JobMode::Sort,
        source: StorageRef::TestOpaque {
            id: "source".into(),
        },
        target: Some(StorageRef::TestOpaque {
            id: "target".into(),
        }),
        settings,
        alias_rows: vec![vec!["אברהם פריד".into(), "אברהם פריד".into()]],
        resource_ids: vec!["synthetic".into()],
        adapter_versions: vec!["synthetic".into()],
        ai_profile: AiProfile::Disabled,
    };
    (
        r,
        s,
        Provider(metadata),
        Ai {
            profile: AiProfile::Disabled,
            spec: json!({}),
        },
    )
}
#[test]
fn title_error_aborts_item_but_not_later_main_only_items() {
    for main_folder_only in [false, true] {
        let files = [
            (
                "אברהם פריד first.mp3",
                json!({"title":{"value_error":"ValueError"}}),
            ),
            ("אברהם פריד second.mp3", json!({})),
        ];
        let (r, mut s, mut m, mut a) = scenario(
            &files,
            Settings {
                main_folder_only,
                ..Settings::default()
            },
        );
        let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
        assert_eq!(p.operations.len(), usize::from(main_folder_only));
        assert!(
            p.decisions
                .iter()
                .any(|d| matches!(d, DecisionV2::ItemError { .. }))
        );
    }
}
#[test]
fn album_normalization_collisions_are_not_suffixed_and_count_destination_files() {
    let tags = json!({"album":"אלבום","artist":"אברהם פריד"});
    let files = [
        ("album/a?.mp3", tags.clone()),
        ("album/a*.mp3", tags.clone()),
        ("album/a.mp3", tags),
    ];
    let (r, mut s, mut m, mut a) = scenario(&files, Settings::default());
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert_eq!(p.operations.len(), 1);
    let OperationKind::AlbumTransfer {
        children,
        remove_empty_source,
        ..
    } = &p.operations[0].kind
    else {
        panic!()
    };
    assert!(*remove_empty_source);
    assert_eq!(children.len(), 3);
    assert!(
        children
            .iter()
            .all(|c| c.destination.components.last().unwrap() == "a.mp3")
    );
    assert!(children[1..].iter().all(|c| {
        c.blockers
            .contains(&"ALBUM_SANITIZATION_COLLISION_NO_SUFFIX".into())
    }));
    assert_eq!(
        p.predicted_legacy_counters.top_artists,
        vec![("אברהם פריד".into(), 1)]
    );
}
#[test]
fn fix_collision_reservations_follow_predicted_renames_even_after_open_error() {
    let files = [
        ("a_b.mp3", json!({"open_error":"OSError"})),
        ("a-b.mp3", json!({})),
        ("a b.mp3", json!({})),
    ];
    let (mut r, mut s, mut m, mut a) = scenario(&files, Settings::default());
    r.mode = JobMode::FixNames;
    r.target = None;
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    let names = p
        .operations
        .iter()
        .map(|o| {
            let OperationKind::FixFile(f) = &o.kind else {
                panic!()
            };
            f.intended_name.clone().unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["a b_1.mp3", "a b_2.mp3", "a b.mp3"]);
    let OperationKind::FixFile(first) = &p.operations[0].kind else {
        panic!()
    };
    assert!(!first.save_intended);
    assert_eq!(id(&first.source.reference), "source/a_b.mp3");
    assert_eq!(p.operations[1].depends_on, vec![0]);
}
#[test]
fn duet_all_skipped_retains_only_blocked_historical_delete_intention() {
    let (mut r, mut s, mut m, mut a) = scenario(
        &[("song.mp3", json!({}))],
        Settings {
            duet_mode: true,
            exist_only: true,
            ..Settings::default()
        },
    );
    r.ai_profile = AiProfile::Configured {
        bundle_id: "synthetic".into(),
        available: true,
    };
    a.profile = r.ai_profile.clone();
    a.spec = json!({"ner":{"song":["אברהם פריד","מוטי שטיינמץ"]}});
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.operations.is_empty());
    assert!(
        matches!(&p.predicted_effects[0],PredictedEffect::DuetDeleteIntention{blocked:true,after_operations,..} if after_operations.is_empty())
    );
    assert!(s.entries.contains_key("source/song.mp3"));
}
#[test]
fn root_overlap_and_case_collisions_are_visible_apply_blockers() {
    let (mut r, mut s, mut m, mut a) = scenario(
        &[
            ("target/song.mp3", json!({})),
            ("אברהם פריד.mp3", json!({})),
        ],
        Settings::default(),
    );
    r.target = Some(StorageRef::TestOpaque {
        id: "source/target".into(),
    });
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.blockers.contains(&"SOURCE_TARGET_OVERLAP".into()));
}
#[test]
fn title_sanitized_to_none_is_only_an_error_if_that_title_is_used() {
    let (r, _, mut m, mut a) = scenario(
        &[
            ("אברהם פריד.mp3", json!({"title":"<>"})),
            ("song.mp3", json!({"title":"<>","artist":"אברהם פריד"})),
            ("unmatched.mp3", json!({"title":"<>"})),
        ],
        Settings::default(),
    );
    for name in ["אברהם פריד.mp3", "song.mp3"] {
        assert_eq!(
            single_decision(
                &entry(&format!("source/{name}"), EntryKind::File),
                &r.alias_rows,
                &mut m,
                &mut a,
                false
            )
            .unwrap(),
            vec!["אברהם פריד"]
        );
    }
    assert!(
        single_decision(
            &entry("source/unmatched.mp3", EntryKind::File),
            &r.alias_rows,
            &mut m,
            &mut a,
            false
        )
        .is_err()
    );
}
#[test]
fn destination_file_ancestor_prevents_hypothetical_transfers() {
    let (r, mut s, mut m, mut a) = scenario(
        &[("אברהם פריד.mp3", json!({}))],
        Settings {
            abc_sort: true,
            ..Settings::default()
        },
    );
    s.entries
        .insert("target/א".into(), entry("target/א", EntryKind::File));
    s.listings.insert("target".into(), vec!["target/א".into()]);
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.operations.is_empty());
    assert_eq!(p.predicted_legacy_counters.songs_sorted, 0);
    assert_eq!(p.predicted_legacy_counters.artist_folders_created, 0);
}
#[test]
fn fix_audio_directory_rename_invalidates_old_descendant_read_points() {
    let (mut r, mut s, mut m, mut a) =
        scenario(&[("foo?.mp3/child.mp3", json!({}))], Settings::default());
    r.mode = JobMode::FixNames;
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert_eq!(p.operations.len(), 2);
    let OperationKind::FixFile(second) = &p.operations[1].kind else {
        panic!()
    };
    assert!(!second.save_intended);
    assert!(second.failure.is_some());
    assert!(p.decisions.iter().any(|d|matches!(d,DecisionV2::Vanished{read} if id(&read.source.reference)=="source/foo?.mp3/child.mp3")));
}
#[test]
fn extension_only_dotfiles_are_ignored_in_sort_and_fix() {
    for mode in [JobMode::Sort, JobMode::FixNames] {
        let (mut r, mut s, mut m, mut a) = scenario(
            &[(".mp3", json!({"artist":"אברהם פריד"}))],
            Settings::default(),
        );
        r.mode = mode;
        let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
        assert!(p.operations.is_empty());
        assert!(
            !p.decisions
                .iter()
                .any(|d| matches!(d, DecisionV2::Single { .. }))
        );
    }
}
#[test]
fn all_seven_fix_job_goldens_match_collection_rename_and_save_order() {
    let mut count = 0;
    for golden in goldens().into_iter().filter(|g| g["op"] == "fix") {
        let inp = inputs();
        let input = inp["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == golden["id"])
            .unwrap();
        let (request, mut storage, mut metadata, mut ai) = fixture(input, &golden);
        // Fix's complete rglob/glob trace explicitly records the collected order.
        let collected = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| (e["kind"] == "rglob" || e["kind"] == "glob") && e["pattern"] == "*")
            .unwrap();
        let mut listings: HashMap<String, Vec<String>> = HashMap::new();
        for e in collected["children"].as_array().unwrap() {
            let path = e["path"].as_str().unwrap();
            if let Some((parent, _)) = path.rsplit_once('/') {
                listings.entry(parent.into()).or_default().push(path.into());
            }
        }
        for (parent, children) in listings {
            storage.listings.insert(parent, children);
        }
        let p = plan_job(&request, &mut storage, &mut metadata, &mut ai).unwrap();
        let actual = p
            .operations
            .iter()
            .map(|o| {
                let OperationKind::FixFile(f) = &o.kind else {
                    panic!()
                };
                let source = id(&f.source.reference);
                let (parent, _) = source.rsplit_once('/').unwrap();
                format!("{parent}/{}", f.intended_name.as_ref().unwrap())
            })
            .collect::<Vec<_>>();
        let expected = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "metadata_open")
            .map(|e| {
                e["source_observation"]["path"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{}", input["id"]);
        let saves = p
            .operations
            .iter()
            .filter(|o| matches!(&o.kind,OperationKind::FixFile(f) if f.save_intended))
            .count();
        let expected_saves = golden["ordered_trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "metadata_save_in_memory")
            .count();
        assert_eq!(saves, expected_saves, "{}", input["id"]);
        count += 1;
    }
    assert_eq!(count, 7);
}
#[test]
fn changed_empty_directory_is_not_treated_as_unobserved() {
    struct Changing {
        inner: Storage,
        calls: usize,
    }
    impl ReadStorage for Changing {
        fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
            self.inner.stat(r)
        }
        fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
            if id(r) == "source/empty" {
                self.calls += 1;
                if self.calls == 2 {
                    return Ok(vec![entry("source/empty/new.mp3", EntryKind::File)]);
                }
            }
            self.inner.children(r)
        }
        fn open_read(&mut self, r: &StorageRef) -> Result<ReadInput, StorageError> {
            self.inner.open_read(r)
        }
    }
    let (r, mut s, mut m, mut a) =
        scenario(&[("empty/temporary.txt", json!({}))], Settings::default());
    s.listings.insert("source/empty".into(), vec![]);
    s.entries.remove("source/empty/temporary.txt");
    let mut s = Changing { inner: s, calls: 0 };
    let error = plan_job(&r, &mut s, &mut m, &mut a).unwrap_err();
    assert_eq!(error.code, "CHANGED_ENUMERATION");
}
#[test]
fn metadata_reads_are_bound_to_their_logical_operation_epoch() {
    let inp = inputs();
    let cases = goldens();
    let g = cases
        .iter()
        .find(|g| g["id"] == "scan-album-nested-copy")
        .unwrap();
    let i = inp["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == g["id"])
        .unwrap();
    let (r, mut s, mut m, mut a) = fixture(i, g);
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.metadata_observations.iter().any(|o|matches!(o,sorter_core::metadata::MetadataObservation::Open{source,after_operations:1,..} if id(&source.reference)=="source/album/extra/אברהם פריד.mp3")));
}
#[test]
fn newly_reserved_case_only_targets_are_blocked_without_rewriting_raw_intent() {
    let (mut r, mut s, mut m, mut a) = scenario(
        &[("first.mp3", json!({})), ("second.mp3", json!({}))],
        Settings {
            copy_mode: true,
            singles_folder: false,
            ..Settings::default()
        },
    );
    r.ai_profile = AiProfile::Configured {
        bundle_id: "synthetic".into(),
        available: true,
    };
    a.profile = r.ai_profile.clone();
    a.spec = json!({"ner":{"first":["Artist"],"second":["artist"]}});
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    let OperationKind::SingleTransfer { transfer, .. } = &p.operations[1].kind else {
        panic!()
    };
    assert_eq!(transfer.destination.components[0], "artist");
    assert!(
        transfer
            .blockers
            .contains(&"CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION".into())
    );
    assert!(!p.executable);
}
#[test]
fn main_only_deep_target_overlap_is_proven_by_parent_identities() {
    let (mut r, mut s, mut m, mut a) = scenario(
        &[
            ("sub/target/song.mp3", json!({})),
            ("אברהם פריד.mp3", json!({})),
        ],
        Settings {
            main_folder_only: true,
            ..Settings::default()
        },
    );
    r.target = Some(StorageRef::TestOpaque {
        id: "source/sub/target".into(),
    });
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.blockers.contains(&"SOURCE_TARGET_OVERLAP".into()));
}
#[test]
fn lazy_main_only_observations_are_safety_audited() {
    let (r, mut s, mut m, mut a) = scenario(
        &[("child/link.mp3", json!({}))],
        Settings {
            main_folder_only: true,
            ..Settings::default()
        },
    );
    let e = s.entries.get_mut("source/child/link.mp3").unwrap();
    e.kind = EntryKind::LinkOrReparse;
    e.identity = None;
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.blockers.contains(&"LINK_OR_REPARSE".into()));
    assert!(p.blockers.contains(&"MISSING_IDENTITY".into()));
}
#[test]
fn empty_artist_abc_exception_aborts_remaining_duet_targets_and_delete() {
    let (mut r, mut s, mut m, mut a) = scenario(
        &[("song.mp3", json!({}))],
        Settings {
            duet_mode: true,
            abc_sort: true,
            ..Settings::default()
        },
    );
    r.ai_profile = AiProfile::Configured {
        bundle_id: "synthetic".into(),
        available: true,
    };
    a.profile = r.ai_profile.clone();
    a.spec = json!({"ner":{"song":["","אברהם פריד"]}});
    let p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    assert!(p.operations.is_empty());
    assert!(
        !p.predicted_effects
            .iter()
            .any(|e| matches!(e, PredictedEffect::DuetDeleteIntention { .. }))
    );
    assert!(
        p.decisions.iter().any(
            |d| matches!(d,DecisionV2::ItemError{error,..} if error.starts_with("IndexError"))
        )
    );
}
#[test]
fn v2_binding_covers_roots_versions_observations_and_mutation_intentions() {
    let (r, mut s, mut m, mut a) = scenario(&[("אברהם פריד.mp3", json!({}))], Settings::default());
    let mut p = plan_job(&r, &mut s, &mut m, &mut a).unwrap();
    verify_job_plan_binding(&p).unwrap();
    p.request.resource_ids.push("changed".into());
    assert!(verify_job_plan_binding(&p).is_err());
}
