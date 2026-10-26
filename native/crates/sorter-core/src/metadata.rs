//! Ordered metadata access contracts for read-only policy.
use crate::discovery::{EntryObservation, StorageError};
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum LegacyField {
    Artist,
    AlbumArtist,
    Title,
    Album,
    Genre,
    TrackNumber,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorStage {
    Open,
    Get,
    Value,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub stage: ErrorStage,
    pub kind: String,
    pub detail: String,
}
impl FieldError {
    pub fn get(kind: &str, detail: impl Into<String>) -> Self {
        Self {
            stage: ErrorStage::Get,
            kind: kind.into(),
            detail: detail.into(),
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LegacyScalar {
    Text(String),
    Integer(String),
    Null,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LegacyItem {
    pub values: Vec<LegacyScalar>,
    pub truthy: bool,
    pub value: Result<LegacyScalar, FieldError>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MetadataError {
    Storage(StorageError),
    UnsupportedPlatform,
    SeekRequired,
    InvalidInput,
    LegacyOpen { kind: String, detail: String },
    Parser,
    InputBudget,
    NativeException,
    UnsupportedClass,
}

pub trait MetadataFields {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError>;
}
pub trait MetadataProvider {
    fn open(&mut self, entry: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError>;
}

/// Captures adapter observations, not proof that policy consumed a returned value.
/// Get/value-stage errors stay distinct inside the observed LegacyItem.
#[derive(Clone, Debug, serde::Serialize)]
pub enum MetadataObservation {
    Open {
        source: EntryObservation,
        after_operations: usize,
        result: Result<(), MetadataError>,
    },
    Get {
        source: EntryObservation,
        after_operations: usize,
        field: LegacyField,
        result: Result<LegacyItem, FieldError>,
    },
}
pub(crate) struct RecordingProvider<'a> {
    pub inner: &'a mut dyn MetadataProvider,
    pub clock: std::rc::Rc<std::cell::Cell<usize>>,
    pub observations: std::rc::Rc<std::cell::RefCell<Vec<MetadataObservation>>>,
}
struct RecordingFields {
    inner: Box<dyn MetadataFields>,
    source: EntryObservation,
    clock: std::rc::Rc<std::cell::Cell<usize>>,
    observations: std::rc::Rc<std::cell::RefCell<Vec<MetadataObservation>>>,
}
impl MetadataProvider for RecordingProvider<'_> {
    fn open(
        &mut self,
        source: &EntryObservation,
    ) -> Result<Box<dyn MetadataFields>, MetadataError> {
        let result = self.inner.open(source);
        self.observations
            .borrow_mut()
            .push(MetadataObservation::Open {
                source: source.clone(),
                after_operations: self.clock.get(),
                result: result.as_ref().map(|_| ()).map_err(Clone::clone),
            });
        result.map(|inner| {
            Box::new(RecordingFields {
                inner,
                source: source.clone(),
                clock: self.clock.clone(),
                observations: self.observations.clone(),
            }) as Box<dyn MetadataFields>
        })
    }
}
impl MetadataFields for RecordingFields {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        let result = self.inner.get(field);
        self.observations
            .borrow_mut()
            .push(MetadataObservation::Get {
                source: self.source.clone(),
                after_operations: self.clock.get(),
                field,
                result: result.clone(),
            });
        result
    }
}
