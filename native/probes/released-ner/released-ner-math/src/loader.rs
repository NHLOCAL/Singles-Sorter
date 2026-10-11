use crate::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn hash(x: &[u8]) -> String {
    format!("{:x}", Sha256::digest(x))
}
impl Model {
    pub fn load(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let manifest: Value = serde_json::from_slice(&fs::read(dir.join("manifest.json"))?)?;
        let lock: Value = serde_json::from_str(include_str!("../model-lock.json"))?;
        for (key, expected) in lock.as_object().unwrap() {
            assert_eq!(
                &manifest[key], expected,
                "unsupported model contract field {key}"
            );
        }
        assert_eq!(manifest["schema"], "released-ner-v1");
        assert_eq!(
            manifest["model_sha256"],
            "5f0d50400f685c3992a21d395f7fab30b45ca3a9968383c24044ff4478ec865e"
        );
        assert_eq!(
            manifest["release"],
            "78d001aee5ce49628bdffebd34540fa83ba32c12"
        );
        assert_eq!(manifest["actions"], json!(ACTIONS));
        assert_eq!(manifest["unseen_classes"], json!([]));
        assert_eq!(manifest["parameter_bytes"], 3829976);
        let bytes = fs::read(dir.join("tensors.f32"))?;
        assert_eq!(
            hash(&bytes),
            "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745"
        );
        assert_eq!(hash(&bytes), manifest["tensor_sha256"]);
        assert_eq!(bytes.len(), 3829976);
        let mut m = Model::default();
        let mut expected_offset = 0;
        for t in manifest["tensors"].as_array().unwrap() {
            let offset = t["offset_bytes"].as_u64().unwrap() as usize;
            let len = t["nbytes"].as_u64().unwrap() as usize;
            assert_eq!(offset, expected_offset);
            expected_offset += len;
            assert_eq!(t["dtype"], "<f4");
            let raw = &bytes[offset..offset + len];
            assert_eq!(hash(raw), t["sha256"]);
            let data: Vec<_> = raw
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                .collect();
            assert!(data.iter().all(|x| x.is_finite()));
            let shape = serde_json::from_value(t["shape"].clone())?;
            assert!(
                m.tensors
                    .insert(
                        (
                            t["node"].as_u64().unwrap() as usize,
                            t["param"].as_str().unwrap().to_string()
                        ),
                        Tensor { shape, data }
                    )
                    .is_none()
            );
        }
        assert_eq!(expected_offset, bytes.len());
        m.validate();

        Ok(m)
    }
}
