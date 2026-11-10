//! Original asset loading only. This API does not activate product inference.
use crate::tokenizer::Tokenizer;
use released_ner_probe::{LoadError, Model};
use std::path::Path;

/// The frozen numeric model and tokenizer, published together only after both
/// have been fully validated. Access is immutable; there is no partial/default
/// constructor, replacement model, normalization, or fallback.
pub struct ReleasedAssets {
    model: Model,
    tokenizer: Tokenizer,
}
impl ReleasedAssets {
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let model = Model::load(dir)?;
        let tokenizer = Tokenizer::load(dir)?;
        Ok(Self { model, tokenizer })
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }
}
