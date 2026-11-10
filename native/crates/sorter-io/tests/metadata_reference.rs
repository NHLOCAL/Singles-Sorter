#![cfg(native_metadata_bridge)]
use sorter_io::{
    metadata::{LegacyField, MetadataReader},
    storage::ReadInput,
};
use std::{fs, io::Cursor, path::PathBuf};
#[test]
fn frozen_real_format_reference_fields_and_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media");
    let oracle: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("reference-observations.json")).unwrap())
            .unwrap();
    for row in oracle["files"].as_array().unwrap() {
        let name = row["path"]
            .as_str()
            .unwrap()
            .strip_prefix("fixtures/")
            .unwrap();
        let path = root.join(name);
        let before = fs::read(&path).unwrap();
        let result = MetadataReader.open(
            ReadInput::seekable(Cursor::new(before.clone()), before.len() as u64),
            name,
        );
        if row["open"]["ok"] == false {
            assert!(
                result.is_err(),
                "{name}: legacy open failure must remain failure"
            );
        } else {
            let mut handle = result.unwrap_or_else(|e| panic!("{name}: {e:?}"));
            for (field, key) in [
                (LegacyField::Artist, "artist"),
                (LegacyField::AlbumArtist, "albumartist"),
                (LegacyField::Title, "title"),
                (LegacyField::Album, "album"),
                (LegacyField::Genre, "genre"),
                (LegacyField::TrackNumber, "tracknumber"),
            ] {
                let actual = handle.get(field);
                let expected = &row["fields"][key];
                if expected["get_ok"] == false {
                    let error = actual.unwrap_err();
                    assert_eq!(
                        error.stage,
                        sorter_io::metadata::ErrorStage::Get,
                        "{name}/{key}"
                    );
                    assert_eq!(
                        error.kind,
                        expected["error"]["type"].as_str().unwrap(),
                        "{name}/{key}"
                    );
                } else {
                    let item = actual.unwrap_or_else(|e| panic!("{name}/{key}: {e:?}"));
                    assert_eq!(reference_json(&item), *expected, "{name}/{key}");
                }
            }
        }
        assert_eq!(before, fs::read(path).unwrap());
    }
}

fn reference_json(item: &sorter_io::metadata::LegacyItem) -> serde_json::Value {
    use serde_json::json;
    use sorter_io::metadata::LegacyScalar;
    fn scalar(s: &LegacyScalar) -> serde_json::Value {
        match s {
            LegacyScalar::Text(s) => json!({"type":"str","value":s}),
            LegacyScalar::Integer(s) => json!({"type":"integer","decimal":s}),
            LegacyScalar::Null => json!({"type":"NoneType","value":null}),
        }
    }
    json!({"get_ok":true,"values":item.values.iter().map(scalar).collect::<Vec<_>>(),"truthy":item.truthy,"value":match &item.value{Ok(s)=>json!({"ok":true,"scalar":scalar(s)}),Err(e)=>json!({"ok":false,"error":{"type":e.kind,"message":e.detail}})}})
}

#[test]
fn real_metadata_single_song_routing_matches_unchanged_release() {
    use sorter_core::{
        Alias, ClassifyRequest, MetadataFacts, PolicyContext, RecognitionFacts, Settings,
        SongFacts, classify,
    };
    use sorter_io::metadata::LegacyScalar;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media");
    let routes: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("route-observations.json")).unwrap()).unwrap();
    let aliases = routes["aliases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Alias {
            source: v[0].as_str().unwrap().into(),
            target: v[1].as_str().unwrap().into(),
        })
        .collect::<Vec<_>>();
    for route in routes["routes"].as_array().unwrap() {
        let fixture = route["fixture"].as_str().unwrap();
        let filename = route["filename"].as_str().unwrap();
        let bytes = fs::read(root.join(fixture)).unwrap();
        // This is a test-only comparison of the existing supplied-facts single-song
        // policy. It does not introduce a production v2->v1 conversion or album API.
        let metadata = match MetadataReader.open(
            ReadInput::seekable(Cursor::new(bytes.clone()), bytes.len() as u64),
            filename,
        ) {
            Ok(mut handle) => {
                let text = |s| match s {
                    LegacyScalar::Text(s) => s,
                    other => panic!("unexpected string field: {other:?}"),
                };
                let title = text(handle.get(LegacyField::Title).unwrap().value.unwrap());
                let artist = text(handle.get(LegacyField::Artist).unwrap().value.unwrap());
                MetadataFacts::Present { artist, title }
            }
            Err(_) => MetadataFacts::ReadError {
                code: "observed-open-failure".into(),
            },
        };
        let request = ClassifyRequest {
            schema_version: 1,
            context: PolicyContext {
                ruleset_id: "singles-v14.0".into(),
                settings: Settings::default(),
                aliases: aliases.clone(),
                recognition: RecognitionFacts::Disabled,
            },
            song: SongFacts {
                source_id: fixture.into(),
                filename: filename.into(),
                metadata,
            },
        };
        let actual = classify(&request).unwrap();
        assert_eq!(
            serde_json::to_value(actual.artists).unwrap(),
            route["result"]["artists"],
            "{filename}"
        );
    }
}

#[cfg(windows)]
#[test]
fn native_windows_unicode_path_metadata_preserves_bytes_and_mtime() {
    use sorter_io::{desktop::DesktopStorage, storage::ReadStorage};
    let media =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/tagged.mp3");
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("מקור 🎵");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("שיר 𝄞.mp3");
    fs::copy(media, &path).unwrap();
    let before = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let mut storage = DesktopStorage::new(&[folder]).unwrap();
    let reference = storage.reference(&path).unwrap();
    let entry = storage.stat(&reference).unwrap();
    let input = storage.open_read(&reference).unwrap();
    let mut metadata = MetadataReader.open(input, &entry.name).ok().unwrap();
    assert_eq!(metadata.get(LegacyField::Artist).unwrap().values.len(), 2);
    drop(metadata);
    drop(storage);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
}

#[test]
fn acquired_source_and_fixture_hashes_match_manifests() {
    use sha2::{Digest, Sha256};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("dependencies/source-manifest.json")).unwrap())
            .unwrap();
    for row in manifest["files"].as_array().unwrap() {
        let relative = row["path"].as_str().unwrap();
        let path = if let Some(suffix) = relative.strip_prefix("taglib-2.3.2/") {
            PathBuf::from(env!("SORTER_TAGLIB_SOURCE_DIR")).join(suffix)
        } else {
            PathBuf::from(env!("SORTER_ZLIB_SOURCE_DIR"))
                .join(relative.strip_prefix("zlib-1.3.1/").unwrap())
        };
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            row["sha256"],
            "{}",
            path.display()
        );
    }
    let media = root.join("../../tests/fixtures/media");
    let observations: serde_json::Value =
        serde_json::from_slice(&fs::read(media.join("reference-observations.json")).unwrap())
            .unwrap();
    for row in observations["files"].as_array().unwrap() {
        let path = media.join(
            row["path"]
                .as_str()
                .unwrap()
                .strip_prefix("fixtures/")
                .unwrap(),
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(&path).unwrap())),
            row["sha256"],
            "{}",
            path.display()
        );
    }
}
