use serde_json::{Value, json};
use sorter_core::{album::*, discovery::*, metadata::*};
use std::collections::HashMap;
pub fn entry(path: &str, kind: EntryKind) -> EntryObservation {
    EntryObservation {
        reference: StorageRef::TestOpaque { id: path.into() },
        identity: None,
        parent_identity: None,
        name: path.rsplit('/').next().unwrap().into(),
        kind,
        size: None,
        modified: None,
        capabilities: Capabilities::default(),
    }
}
pub fn field_name(f: LegacyField) -> &'static str {
    match f {
        LegacyField::Album => "album",
        LegacyField::Artist => "artist",
        LegacyField::AlbumArtist => "albumartist",
        LegacyField::TrackNumber => "tracknumber",
        LegacyField::Title => "title",
        LegacyField::Genre => "genre",
    }
}
fn scalar(v: &Value) -> LegacyScalar {
    match v {
        Value::String(s) => LegacyScalar::Text(s.clone()),
        Value::Null => LegacyScalar::Null,
        Value::Number(n) => LegacyScalar::Integer(n.to_string()),
        _ => match v["kind"].as_str() {
            Some("integer") => LegacyScalar::Integer(v["decimal"].as_str().unwrap().into()),
            Some("null") => LegacyScalar::Null,
            Some("text") => LegacyScalar::Text(v["text"].as_str().unwrap().into()),
            _ => LegacyScalar::Text(String::new()),
        },
    }
}
fn truthy(v: &LegacyScalar) -> bool {
    match v {
        LegacyScalar::Text(s) => !s.is_empty(),
        LegacyScalar::Null => false,
        LegacyScalar::Integer(s) => s.chars().any(|c| c.is_ascii_digit() && c != '0'),
    }
}
pub struct Fields(pub Value);
impl MetadataFields for Fields {
    fn get(&mut self, f: LegacyField) -> Result<LegacyItem, FieldError> {
        let empty = json!("");
        let spec = self.0.get(field_name(f)).unwrap_or(&empty);
        if let Some(kind) = spec.get("get_error").and_then(Value::as_str) {
            return Err(FieldError::get(kind, "injected"));
        }
        let val = if spec.is_object() && spec.get("kind").is_none() {
            scalar(spec.get("value").unwrap_or(&empty))
        } else {
            scalar(spec)
        };
        let values = spec
            .get("values")
            .and_then(Value::as_array)
            .map(|v| v.iter().map(scalar).collect())
            .unwrap_or_else(|| vec![val.clone()]);
        let truthy = spec
            .get("truthy")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| values.iter().any(truthy));
        let value = if let Some(kind) = spec.get("value_error").and_then(Value::as_str) {
            Err(FieldError {
                stage: ErrorStage::Value,
                kind: kind.into(),
                detail: "injected".into(),
            })
        } else {
            Ok(val)
        };
        Ok(LegacyItem {
            values,
            truthy,
            value,
        })
    }
}
pub struct Provider(pub HashMap<String, Value>);
impl MetadataProvider for Provider {
    fn open(&mut self, entry: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        let StorageRef::TestOpaque { id } = &entry.reference else {
            panic!("synthetic only")
        };
        let spec = self.0.get(id).cloned().unwrap_or_else(|| json!({}));
        if let Some(kind) = spec["open_error"].as_str() {
            return Err(MetadataError::LegacyOpen {
                kind: kind.into(),
                detail: "injected".into(),
            });
        }
        Ok(Box::new(Fields(spec)))
    }
}
fn id(s: &StorageRef) -> &str {
    let StorageRef::TestOpaque { id } = s else {
        panic!()
    };
    id
}
fn encode(s: &LegacyScalar) -> Value {
    match s {
        LegacyScalar::Text(s) => json!({"kind":"text","text":s}),
        LegacyScalar::Integer(s) => json!({"kind":"integer","decimal":s}),
        LegacyScalar::Null => json!({"kind":"null"}),
    }
}
pub fn trace(events: &[AlbumTraceEvent]) -> Vec<Value> {
    events.iter().filter_map(|e| Some(match e {
    AlbumTraceEvent::Open { source, result } => { let mut v = json!({"kind":"metadata_open","source":id(source)}); if let Err(e) = result { if let MetadataError::LegacyOpen { kind, .. } = e { v["error"] = json!({"type":kind}); } else { panic!("unexpected error {e:?}"); } } v },
    AlbumTraceEvent::Get { source, field, result } => { let mut v = json!({"kind":"metadata_get","source":id(source),"field":field_name(*field)}); if let Err(e) = result { v["error"] = json!({"type":e.kind}); } v },
    AlbumTraceEvent::Truthy { source, field, result } => json!({"kind":"metadata_truthy","source":id(source),"field":field_name(*field),"result":result}),
    AlbumTraceEvent::Value { source, field, values, result } => { let mut v = json!({"kind":"metadata_value","source":id(source),"field":field_name(*field)}); match result { Ok(s) => { v["result"] = encode(s); v["values"] = Value::Array(values.iter().map(encode).collect()); }, Err(e) => v["error"] = json!({"type":e.kind}) }; v },
    _ => return None,
})).collect()
}
pub fn fixture(input: &Value, golden: &Value) -> (FolderObservation, Provider) {
    let folder = input["folder"].as_str().unwrap();
    let specs: HashMap<_, _> = input["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                format!("source/{}", f["path"].as_str().unwrap()),
                f["metadata"].clone(),
            )
        })
        .collect();
    // Replay actual oracle enumeration, NOT authored input order (ties depend on it).
    let children = golden["ordered_trace"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "glob")
        .map(|e| {
            e["children"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    entry(
                        e["path"].as_str().unwrap(),
                        if e["kind"] == "directory" {
                            EntryKind::Directory
                        } else {
                            EntryKind::File
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    (
        FolderObservation {
            entry: entry(folder, EntryKind::Directory),
            direct_children: children,
        },
        Provider(specs),
    )
}
