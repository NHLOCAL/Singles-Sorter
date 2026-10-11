use crate::{Model, Tensor, assets::*};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
struct TensorSpec {
    node: usize,
    param: String,
    shape: Vec<usize>,
    dtype: String,
    offset_bytes: usize,
    nbytes: usize,
    sha256: String,
}
fn unsupported(field: &str) -> LoadError {
    LoadError::Unsupported {
        field: field.into(),
    }
}
fn tensor_error(reason: &'static str) -> LoadError {
    LoadError::InvalidTensor { reason }
}

impl Model {
    /// Load only the frozen released graph and original float32 weights.
    /// No partially validated model escapes, and no caller can mutate its tensors.
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let bytes = read_bounded(&dir.join("manifest.json"), "manifest.json", MANIFEST_LIMIT)?;
        let manifest: Value = serde_json::from_slice(&bytes).map_err(|source| LoadError::Json {
            asset: "manifest.json",
            source,
        })?;
        let lock: Value =
            serde_json::from_str(include_str!("../model-lock.json")).map_err(|source| {
                LoadError::Json {
                    asset: "embedded model-lock.json",
                    source,
                }
            })?;
        let fields = lock
            .as_object()
            .ok_or_else(|| unsupported("embedded contract"))?;
        for (key, expected) in fields {
            if manifest.get(key) != Some(expected) {
                return Err(unsupported(key));
            }
        }
        // Optional exporter metadata must not redirect or misidentify the assets.
        if manifest
            .get("tensor_file")
            .is_some_and(|v| v != "tensors.f32")
        {
            return Err(unsupported("tensor_file"));
        }
        if let Some(files) = manifest.get("files")
            && (files.get("tensors.f32").and_then(Value::as_str) != Some(TENSOR_SHA256)
                || files.get("tokenizer.json").and_then(Value::as_str) != Some(TOKENIZER_SHA256))
        {
            return Err(unsupported("files"));
        }
        let specs: Vec<TensorSpec> = serde_json::from_value(
            manifest
                .get("tensors")
                .ok_or_else(|| unsupported("tensors"))?
                .clone(),
        )
        .map_err(|source| LoadError::Json {
            asset: "tensor metadata",
            source,
        })?;
        let bytes = read_bounded(&dir.join("tensors.f32"), "tensors.f32", PARAMETER_BYTES)?;
        if bytes.len() != PARAMETER_BYTES {
            return Err(LoadError::Length {
                asset: "tensors.f32",
                expected: PARAMETER_BYTES,
                actual: bytes.len(),
            });
        }
        verify_hash(&bytes, TENSOR_SHA256, "tensors.f32")?;
        let mut tensors = BTreeMap::new();
        let mut expected_offset = 0usize;
        for t in specs {
            let elements = t
                .shape
                .iter()
                .try_fold(1usize, |n, d| n.checked_mul(*d))
                .ok_or_else(|| tensor_error("shape overflow"))?;
            let size = elements
                .checked_mul(4)
                .ok_or_else(|| tensor_error("byte size overflow"))?;
            let end = t
                .offset_bytes
                .checked_add(t.nbytes)
                .ok_or_else(|| tensor_error("range overflow"))?;
            if t.offset_bytes != expected_offset || size != t.nbytes || t.dtype != "<f4" {
                return Err(tensor_error("layout mismatch"));
            }
            let raw = bytes
                .get(t.offset_bytes..end)
                .ok_or_else(|| tensor_error("range out of bounds"))?;
            verify_hash(raw, &t.sha256, "tensor")?;
            let mut data = Vec::new();
            data.try_reserve_exact(elements)
                .map_err(|_| LoadError::Allocation { asset: "tensor" })?;
            for c in raw.chunks_exact(4) {
                let value = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                if !value.is_finite() {
                    return Err(tensor_error("nonfinite value"));
                }
                data.push(value);
            }
            if tensors
                .insert(
                    (t.node, t.param),
                    Tensor {
                        shape: t.shape,
                        data,
                    },
                )
                .is_some()
            {
                return Err(tensor_error("duplicate parameter"));
            }
            expected_offset = end;
        }
        if expected_offset != bytes.len() || tensors.len() != 31 {
            return Err(tensor_error("incomplete graph"));
        }
        // Exact contract equality above guarantees every shape/key used by the
        // unchanged numeric graph; hashes guarantee their original finite values.
        Ok(Model { tensors })
    }
}
