//! The release's single-song access/exception boundary, independent of v1 facts.
use super::*;
use crate::metadata::{LegacyField, LegacyScalar, MetadataProvider};

fn text_value(value: LegacyScalar) -> Result<String, String> {
    match value {
        LegacyScalar::Text(s) => Ok(s),
        LegacyScalar::Null => Ok(String::new()),
        LegacyScalar::Integer(s) if s == "0" => Ok(String::new()),
        _ => Err("TypeError: non-string metadata value".into()),
    }
}
fn recognition_text(text: &str) -> Result<String, String> {
    let cleaned = crate::clean(text);
    if cleaned.is_empty() {
        Ok(text.into())
    } else {
        crate::sanitize(&cleaned).ok_or_else(|| "TypeError: sanitized text is None".into())
    }
}
pub(crate) fn stem(text: &str) -> &str {
    match text.rfind('.') {
        Some(i) if i > 0 && i + 1 < text.len() => &text[..i],
        _ => text,
    }
}
pub(crate) fn alias(text: &str, rows: &[Vec<String>]) -> Result<Option<String>, String> {
    for (index, row) in rows.iter().enumerate() {
        if row.len() != 2 {
            return Err(format!(
                "ValueError: alias row {index} has width {}",
                row.len()
            ));
        }
        if text.contains(&row[0]) && crate::exact_match(text, &row[0]) {
            return Ok(Some(row[1].clone()));
        }
    }
    Ok(None)
}
/// Metadata open failure preserves a filename match. Title get/value failure does
/// not: it escapes this function and aborts the enclosing released scan item.
pub fn single_decision(
    entry: &EntryObservation,
    aliases: &[Vec<String>],
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    enabled: bool,
) -> Result<Vec<String>, RecognitionError> {
    single_decision_controlled(
        entry,
        aliases,
        metadata,
        recognition,
        enabled,
        &UncontrolledPlanning,
    )
}
pub fn single_decision_controlled(
    entry: &EntryObservation,
    aliases: &[Vec<String>],
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    enabled: bool,
    control: &dyn PlanningControl,
) -> Result<Vec<String>, RecognitionError> {
    control.checkpoint(PlanningBoundary::File)?;
    let filename = recognition_text(&entry.name)?;
    let filename = stem(&filename);
    let mut found = alias(filename, aliases)?.into_iter().collect::<Vec<_>>();
    let mut handle = metadata.open(entry).ok();
    // Title is accessed even if filename matching already succeeded.
    let title = if let Some(h) = handle.as_mut() {
        let title = text_value(
            h.get(LegacyField::Title)
                .map_err(|e| format!("{e:?}"))?
                .value
                .map_err(|e| format!("{e:?}"))?,
        )?;
        if title.is_empty() {
            None
        } else {
            let cleaned = crate::clean(&title);
            Some(if cleaned.is_empty() {
                Some(title)
            } else {
                crate::sanitize(&cleaned)
            })
        }
    } else {
        None
    };
    if found.is_empty()
        && let Some(h) = handle.as_mut()
    {
        let artist = text_value(
            h.get(LegacyField::Artist)
                .map_err(|e| format!("{e:?}"))?
                .value
                .map_err(|e| format!("{e:?}"))?,
        )?;
        if !artist.is_empty() {
            let artist = crate::fix_jibrish(&artist, crate::RepairMode::Hebrew);
            if let Some(mapped) = alias(&artist, aliases)? {
                found.push(mapped);
            } else if enabled && crate::check_artist(&artist) {
                // The released classifier is explicitly fail-open on inference errors.
                control.checkpoint(PlanningBoundary::Recognition)?;
                let result = recognition.classify(&artist);
                control.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
                match result {
                    Err(RecognitionError::Abort(abort)) => return Err(abort.into()),
                    Ok(crate::ClassifierFact::Reject) => {}
                    _ => found.push(artist),
                }
            }
        }
    }
    if !found.is_empty() {
        control.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
        return Ok(found);
    }
    let title = title
        .map(|t| {
            t.ok_or_else(|| "TypeError: sanitized title is None".to_string())
                .map(|t| crate::fix_jibrish(&t, crate::RepairMode::Hebrew))
        })
        .transpose()?;
    if found.is_empty()
        && let Some(t) = &title
        && let Some(a) = alias(t, aliases)?
    {
        found.push(a);
    }
    if found.is_empty() && enabled {
        control.checkpoint(PlanningBoundary::Recognition)?;
        let result = recognition.ner(filename);
        control.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
        found = result?;
        if found.is_empty()
            && let Some(t) = title
        {
            control.checkpoint(PlanningBoundary::Recognition)?;
            let result = recognition.ner(&t);
            control.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
            found = result?;
        }
    }
    control.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
    Ok(found)
}
