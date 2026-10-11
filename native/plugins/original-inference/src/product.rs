//! Android dry-run adapter. Provider observations are not file-identity evidence.
//! Metadata is read from bounded, hashed private snapshots; sorting stays in core.
use super::*;
use jni::objects::{JObject, JValue};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sorter_core::{
    Settings,
    discovery::*,
    job_plan::{
        DisabledRecognition, JobMode, JobRequestV2, PlanningBoundary, PlanningControl,
        plan_job_controlled,
    },
    metadata::{MetadataError, MetadataFields, MetadataProvider},
};
use sorter_io::{metadata::MetadataReader, resources::load_aliases};
use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::{Cursor, Read},
    path::PathBuf,
};
#[path = "plan_view.rs"]
mod plan_view;
const BUNDLED: &[u8] =
    include_bytes!("../../../tests/fixtures/full_contract/released-singer-list.csv");
const MAX_MANIFEST: usize = 2 * 1024 * 1024;
const MAX_FILE: u64 = 64 * 1024 * 1024;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    token: String,
    parent: Option<String>,
    tree_uri: String,
    authority: String,
    document_id: String,
    name: String,
    directory: bool,
    observed_size: Option<u64>,
    observed_modified: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplicitSettings {
    copy_mode: bool,
    main_folder_only: bool,
    singles_folder: bool,
    exist_only: bool,
    abc_sort: bool,
    duet_mode: bool,
}
impl From<ExplicitSettings> for Settings {
    fn from(s: ExplicitSettings) -> Self {
        Self {
            copy_mode: s.copy_mode,
            main_folder_only: s.main_folder_only,
            singles_folder: s.singles_folder,
            exist_only: s.exist_only,
            abc_sort: s.abc_sort,
            duet_mode: s.duet_mode,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u8,
    mode: JobMode,
    source: String,
    target: Option<String>,
    settings: ExplicitSettings,
    ai_profile: String,
    personal_csv: Option<String>,
    entries: Vec<Entry>,
}
#[derive(Clone)]
struct SnapshotStorage {
    observations: HashMap<StorageRef, EntryObservation>,
    children: HashMap<StorageRef, Vec<EntryObservation>>,
}
impl ReadStorage for SnapshotStorage {
    fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
        self.observations.get(r).cloned().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::InvalidReference,
                "unregistered SAF reference",
            )
        })
    }
    fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        self.children.get(r).cloned().ok_or_else(|| {
            StorageError::new(StorageErrorKind::NotDirectory, "not a captured directory")
        })
    }
    fn open_read(&mut self, _: &StorageRef) -> Result<ReadInput, StorageError> {
        Err(StorageError::new(
            StorageErrorKind::UnsupportedCapability,
            "provider observations do not prove a bound read handle",
        ))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Captured {
    snapshot_file: String,
    snapshot_size: u64,
    sha256: String,
}
struct SnapshotMetadata<'a, 'b, 'c> {
    directory: PathBuf,
    entries: HashMap<StorageRef, Entry>,
    cancel: JobCancellation,
    env: &'a mut JNIEnv<'b>,
    provider: &'a JObject<'c>,
    hashes: Vec<String>,
    failed: bool,
}
impl MetadataProvider for SnapshotMetadata<'_, '_, '_> {
    fn open(&mut self, entry: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        let result = self.read_snapshot(entry);
        if matches!(
            &result,
            Err(MetadataError::Storage(_) | MetadataError::InputBudget)
        ) {
            self.failed = true;
        }
        result
    }
}
impl SnapshotMetadata<'_, '_, '_> {
    fn read_snapshot(
        &mut self,
        entry: &EntryObservation,
    ) -> Result<Box<dyn MetadataFields>, MetadataError> {
        let fail = |detail: &str| {
            MetadataError::Storage(StorageError::new(StorageErrorKind::Changed, detail))
        };
        if self.cancel.is_cancelled() {
            return Err(MetadataError::Storage(StorageError::new(
                StorageErrorKind::Cancelled,
                "cancelled",
            )));
        }
        let captured = self
            .entries
            .get(&entry.reference)
            .ok_or_else(|| fail("missing snapshot"))?;
        // Java callback stays on this owned JNI worker. No JNIEnv/reference crosses threads.
        let token = captured.token.as_bytes();
        let response = self
            .env
            .with_local_frame(8, |env| -> jni::errors::Result<Vec<u8>> {
                let input = env.byte_array_from_slice(token)?;
                let value = env
                    .call_method(
                        self.provider,
                        "snapshot",
                        "([B)[B",
                        &[JValue::Object(input.as_ref())],
                    )?
                    .l()?;
                let array = JByteArray::from(value);
                let size = env.get_array_length(&array)?;
                if !(0..=16384).contains(&size) {
                    return Err(jni::errors::Error::NullPtr("snapshot response limit"));
                }
                env.convert_byte_array(&array)
            });
        let response = match response {
            Ok(value) => value,
            Err(_) => {
                // The planner can continue after a metadata failure. Clear the Java exception
                // before any later callback; return a typed read failure instead of using JNI
                // with a pending exception. Cancellation is separately checked by core.
                if self.env.exception_check().unwrap_or(true) {
                    let _ = self.env.exception_clear();
                }
                return Err(fail("SAF snapshot callback failed"));
            }
        };
        let snapshot: Captured =
            serde_json::from_slice(&response).map_err(|_| fail("invalid snapshot receipt"))?;
        let name = snapshot.snapshot_file.as_str();
        if name.is_empty()
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
            || name == "."
            || name == ".."
        {
            return Err(fail("invalid snapshot basename"));
        }
        let size = snapshot.snapshot_size;
        if size > MAX_FILE {
            return Err(MetadataError::InputBudget);
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let mut file = options
            .open(self.directory.join(name))
            .map_err(|_| fail("snapshot unavailable"))?;
        let stat = file.metadata().map_err(|_| fail("snapshot stat"))?;
        if !stat.is_file() || stat.len() != size {
            return Err(fail("snapshot shape changed"));
        }
        let mut data = Vec::new();
        data.try_reserve_exact(size as usize)
            .map_err(|_| MetadataError::InputBudget)?;
        let mut buffer = [0; 32768];
        loop {
            if self.cancel.is_cancelled() {
                return Err(MetadataError::Storage(StorageError::new(
                    StorageErrorKind::Cancelled,
                    "cancelled",
                )));
            }
            let n = file
                .read(&mut buffer)
                .map_err(|e| MetadataError::Storage(e.into()))?;
            if n == 0 {
                break;
            }
            if data.len() + n > size as usize {
                return Err(fail("snapshot grew"));
            }
            data.extend_from_slice(&buffer[..n]);
        }
        let hash = format!("{:x}", Sha256::digest(&data));
        if data.len() != size as usize || snapshot.sha256 != hash {
            return Err(fail("snapshot hash changed"));
        }
        self.hashes
            .push(format!("saf-snapshot:{}:sha256:{hash}", captured.token));
        // This is an immutable copy's metadata, not a fabricated provider ReadBinding.
        Ok(Box::new(MetadataReader.open(
            ReadInput::seekable(Cursor::new(data), size),
            &entry.name,
        )?))
    }
}
fn plan(
    env: &mut JNIEnv,
    provider: &JObject,
    model_directory: &[u8],
    snapshot_directory: &[u8],
    bytes: &[u8],
    cancel: JobCancellation,
) -> Result<Value, String> {
    if !MetadataReader::is_available() {
        return Err("ANDROID_METADATA_UNAVAILABLE".into());
    }
    cancel
        .checkpoint(PlanningBoundary::Start)
        .map_err(|_| "cancelled")?;
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| "invalid_manifest")?;
    if manifest.schema != 1 || manifest.entries.len() > 2000 || manifest.entries.is_empty() {
        return Err("manifest_limit".into());
    }
    let directory =
        PathBuf::from(std::str::from_utf8(snapshot_directory).map_err(|_| "invalid_cache_path")?);
    if !directory.is_absolute() {
        return Err("invalid_cache_path".into());
    }
    let mut observations = HashMap::new();
    let mut by_token = HashMap::new();
    let mut originals = HashMap::new();
    let mut unique = HashSet::new();
    let mut children = HashMap::new();
    let mut resources = vec![];
    for e in &manifest.entries {
        if [&e.token, &e.name, &e.tree_uri, &e.authority, &e.document_id]
            .iter()
            .any(|s| s.len() > 4096)
            || !unique.insert(e.token.clone())
        {
            return Err("invalid_entry".into());
        }
        let reference = StorageRef::AndroidDocument {
            tree_uri: e.tree_uri.clone(),
            authority: e.authority.clone(),
            document_id: e.document_id.clone(),
        };
        let observation = EntryObservation {
            reference: reference.clone(),
            identity: None,
            parent_identity: None,
            name: e.name.clone(),
            kind: if e.directory {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            size: e.observed_size,
            modified: e.observed_modified.clone(),
            capabilities: Capabilities {
                read: Some(true),
                list: Some(e.directory),
                seek: None,
            },
        };
        if observations
            .insert(reference.clone(), observation.clone())
            .is_some()
        {
            return Err("duplicate_document".into());
        }
        if e.directory {
            children.insert(reference.clone(), vec![]);
        }
        by_token.insert(e.token.clone(), observation);
        originals.insert(reference, e.clone());
    }
    for e in &manifest.entries {
        if let Some(parent) = &e.parent {
            let p = by_token.get(parent).ok_or("missing_parent")?;
            children
                .get_mut(&p.reference)
                .ok_or("parent_not_directory")?
                .push(by_token[&e.token].clone());
        }
    }
    let source = by_token
        .get(&manifest.source)
        .ok_or("source_missing")?
        .reference
        .clone();
    let target = manifest
        .target
        .as_ref()
        .map(|t| {
            by_token
                .get(t)
                .map(|e| e.reference.clone())
                .ok_or("target_missing")
        })
        .transpose()?;
    let aliases = load_aliases(
        BUNDLED,
        manifest.personal_csv.as_ref().map(|s| s.as_bytes()),
    )
    .map_err(|e| format!("{e:?}"))?;
    resources.extend(
        aliases
            .source_hashes
            .iter()
            .map(|h| format!("aliases:{:?}:sha256:{}", h.source, h.sha256)),
    );
    let mut adapters = vec![
        "android-saf-snapshot-preview-v1".into(),
        "taglib:2.3.2:3ca2d8afaa7f1cf7f6ed10e511ebc368bfacd6dcaa3dbfa690b89e502e8963dc".into(),
        format!(
            "metadata-coverage:sha256:{:x}",
            Sha256::digest(include_bytes!(
                "../../../tests/fixtures/media/metadata-coverage.json"
            ))
        ),
    ];
    let mut recognition: Box<dyn RecognitionProvider> = match manifest.ai_profile.as_str() {
        "Disabled" => Box::new(DisabledRecognition),
        "Configured" => {
            let dir = std::str::from_utf8(model_directory).map_err(|_| "invalid_model_path")?;
            let model = OriginalRecognition::load(
                Path::new(dir),
                BUNDLE_ID,
                Limits::default(),
                cancel.clone(),
            )
            .map_err(|e| format!("{e:?}"))?;
            resources.extend(model.resource_ids());
            adapters.extend(model.adapter_versions());
            Box::new(model)
        }
        _ => return Err("invalid_ai_profile".into()),
    };
    let request = JobRequestV2 {
        mode: manifest.mode,
        source,
        target,
        settings: manifest.settings.into(),
        alias_rows: aliases.ordered_rows.into_iter().map(|r| r.fields).collect(),
        resource_ids: resources,
        adapter_versions: adapters,
        ai_profile: recognition.profile(),
    };
    let mut storage = SnapshotStorage {
        observations,
        children,
    };
    let mut metadata = SnapshotMetadata {
        directory,
        entries: originals,
        cancel: cancel.clone(),
        env,
        provider,
        hashes: vec![],
        failed: false,
    };
    let plan = plan_job_controlled(
        &request,
        &mut storage,
        &mut metadata,
        recognition.as_mut(),
        &cancel,
    )
    .map_err(|e| format!("{}: {}", e.code, e.detail))?;
    cancel
        .checkpoint(PlanningBoundary::Start)
        .map_err(|_| "cancelled")?;
    if metadata.failed {
        return Err("SAF_SNAPSHOT_READ_FAILED".into());
    }
    Ok(
        json!({"plan_id":plan.binding_sha256,"rows":plan_view::project_rows(&plan),"blockers":plan.blockers,"warnings":plan.warnings,"inference_executed":recognition.inference_executed(),"snapshot_hashes":metadata.hashes,"provider_snapshot_only":true,"executable":false}),
    )
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalPlanning_plan0(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    models: JByteArray,
    cache: JByteArray,
    input: JByteArray,
    provider: JObject,
) -> jbyteArray {
    let cancel = match claim(id) {
        Ok(c) => c,
        Err(e) => return output(&mut env, json!({"ok":false,"error":e})),
    };
    let _owner = OwnedRun(id);
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Value, String> {
            let models = bytes(&mut env, &models, MAX_PATH)?;
            let cache = bytes(&mut env, &cache, MAX_PATH)?;
            let input = bytes(&mut env, &input, MAX_MANIFEST)?;
            plan(&mut env, &provider, &models, &cache, &input, cancel.clone())
        }));
    let value = match result {
        _ if cancel.is_cancelled() => json!({"ok":false,"error":"cancelled"}),
        Ok(Ok(v)) => json!({"ok":true,"data":v}),
        Ok(Err(e)) => json!({"ok":false,"error":e}),
        Err(_) => json!({"ok":false,"error":"native_panic"}),
    };
    output(&mut env, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_are_complete_and_strict() {
        let full = json!({"copy_mode":false,"main_folder_only":false,"singles_folder":true,"exist_only":false,"abc_sort":false,"duet_mode":false});
        assert!(serde_json::from_value::<ExplicitSettings>(full.clone()).is_ok());
        for field in [
            "copy_mode",
            "main_folder_only",
            "singles_folder",
            "exist_only",
            "abc_sort",
            "duet_mode",
        ] {
            let mut missing = full.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<ExplicitSettings>(missing).is_err());
            let mut wrong = full.clone();
            wrong[field] = json!(1);
            assert!(serde_json::from_value::<ExplicitSettings>(wrong).is_err());
        }
        let mut extra = full;
        extra["execute"] = json!(true);
        assert!(serde_json::from_value::<ExplicitSettings>(extra).is_err());
    }
    #[test]
    fn provider_entry_cannot_supply_an_identity_or_snapshot_path() {
        let entry = json!({"token":"owned","parent":null,"tree_uri":"content://owned/tree/root","authority":"owned","document_id":"root","name":"root","directory":true,"observed_size":null,"observed_modified":null});
        assert!(serde_json::from_value::<Entry>(entry.clone()).is_ok());
        for field in ["identity", "snapshot_file", "read_binding"] {
            let mut injected = entry.clone();
            injected[field] = json!("/arbitrary");
            assert!(serde_json::from_value::<Entry>(injected).is_err());
        }
    }
    #[test]
    fn captured_observations_never_offer_provider_bound_reads() {
        let mut storage = SnapshotStorage {
            observations: HashMap::new(),
            children: HashMap::new(),
        };
        let reference = StorageRef::AndroidDocument {
            tree_uri: "content://owned/tree/root".into(),
            authority: "owned".into(),
            document_id: "file".into(),
        };
        assert!(storage.stat(&reference).is_err());
        assert!(storage.children(&reference).is_err());
        assert!(storage.open_read(&reference).is_err());
    }
}

/// Same explicit codecs and original CSV parser as the desktop import flow.
fn decode_csv(bytes: &[u8], encoding: &str) -> Result<Value, String> {
    let (codec, data) = match encoding {
        "Utf8" => (encoding_rs::UTF_8, bytes),
        "Utf8Bom" => (
            encoding_rs::UTF_8,
            bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
        ),
        "Windows1255" => (encoding_rs::WINDOWS_1255, bytes),
        "Windows1252" => (encoding_rs::WINDOWS_1252, bytes),
        "Utf16Le" => (
            encoding_rs::UTF_16LE,
            bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes),
        ),
        "Utf16Be" => (
            encoding_rs::UTF_16BE,
            bytes.strip_prefix(&[0xfe, 0xff]).unwrap_or(bytes),
        ),
        _ => return Err("ENCODING_INVALID".into()),
    };
    let text = codec
        .decode_without_bom_handling_and_without_replacement(data)
        .ok_or("ENCODING_INVALID")?;
    if text.len() > MAX_MANIFEST {
        return Err("CSV_TOO_LARGE".into());
    }
    let parsed = load_aliases(&[], Some(text.as_bytes())).map_err(|_| "CSV_INVALID")?;
    if parsed.ordered_rows.len() > 10000 {
        return Err("CSV_TOO_MANY_ROWS".into());
    }
    let rows: Vec<_> = parsed.ordered_rows.into_iter().map(|r| r.fields).collect();
    let diagnostics: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.len() != 2)
        .map(|(index, r)| json!({"index":index,"code":"MALFORMED_ROW","width":r.len()}))
        .collect();
    Ok(json!({"rows":rows,"diagnostics":diagnostics}))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalPlanning_decodeCsv0(
    mut env: JNIEnv,
    _: JClass,
    input: JByteArray,
    encoding: JByteArray,
) -> jbyteArray {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Value, String> {
            let input = bytes(&mut env, &input, MAX_MANIFEST)?;
            let encoding = bytes(&mut env, &encoding, 32)?;
            decode_csv(
                &input,
                std::str::from_utf8(&encoding).map_err(|_| "ENCODING_INVALID")?,
            )
        }));
    let result = match result {
        Ok(Ok(data)) => json!({"ok":true,"data":data}),
        Ok(Err(error)) => json!({"ok":false,"error":error}),
        Err(_) => json!({"ok":false,"error":"native_panic"}),
    };
    output(&mut env, result)
}
#[cfg(test)]
mod csv_tests {
    use super::*;
    #[test]
    fn original_csv_order_width_bom_and_explicit_codecs() {
        let plain = decode_csv(b"a,b\r\na,b,c\r\n\r\n", "Utf8").unwrap();
        assert_eq!(plain["rows"], json!([["a", "b"], ["a", "b", "c"], []]));
        assert_eq!(plain["diagnostics"].as_array().unwrap().len(), 2);
        assert_eq!(
            decode_csv(&[0xef, 0xbb, 0xbf, b'a', b',', b'b'], "Utf8Bom").unwrap()["rows"],
            json!([["a", "b"]])
        );
        assert_eq!(
            decode_csv(&[0xff, 0xfe, b'a', 0, b',', 0, b'b', 0], "Utf16Le").unwrap()["rows"],
            json!([["a", "b"]])
        );
        assert_eq!(
            decode_csv(&[0xfe, 0xff, 0, b'a', 0, b',', 0, b'b'], "Utf16Be").unwrap()["rows"],
            json!([["a", "b"]])
        );
        assert_eq!(
            decode_csv(&[0xe0, b',', 0xe1], "Windows1255").unwrap()["rows"],
            json!([["א", "ב"]])
        );
        assert_eq!(
            decode_csv(&[0xe9, b',', b'x'], "Windows1252").unwrap()["rows"],
            json!([["é", "x"]])
        );
        assert!(decode_csv(&[0xff], "Utf8").is_err());
        assert!(decode_csv(b"a,b", "Auto").is_err());
    }
}
