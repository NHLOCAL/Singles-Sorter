//! Supplied single-song decisions only. No filesystem, model or tag-decoder access.
use super::{
    matching::first_alias,
    text::{check_artist, clean, fix_jibrish, sanitize},
};
use crate::*;

pub fn classify(request: &ClassifyRequest) -> Result<Decision, EngineError> {
    validate_classify_request(request)?;
    let context = &request.context;
    let mut decision = Decision {
        provenance: Provenance::ProvidedFacts,
        artists: None,
        selected_by: None,
        trace: vec![],
        warnings: vec![],
        apply_blockers: vec![],
    };
    if let RecognitionFacts::Configured {
        bundle_id,
        health: RecognitionHealth::Failed,
        ..
    } = &context.recognition
    {
        decision.warnings.push(Diagnostic {
            code: "RECOGNITION_UNHEALTHY".into(),
            message: format!("Supplied configured bundle {bundle_id:?} has failed health; apply is blocked if this decision requires recognition"),
        });
    }
    let filename =
        recognition_text(&request.song.filename).ok_or_else(|| invalid_text("filename"))?;
    let stem = legacy_stem(&filename);
    choose_alias(
        &mut decision,
        stem,
        &context.aliases,
        DecisionSource::FilenameAlias,
    );

    // The release observes metadata even when a filename alias already won.
    let (artist, title) = match &request.song.metadata {
        MetadataFacts::Present { artist, title } => {
            decision
                .trace
                .push(DecisionEvent::MetadataReadObserved { failed: false });
            (Some(artist.as_str()), Some(title.as_str()))
        }
        MetadataFacts::ReadError { code } => {
            decision
                .trace
                .push(DecisionEvent::MetadataReadObserved { failed: true });
            decision.warnings.push(Diagnostic {
                code: "METADATA_READ_ERROR".into(),
                message: format!("Supplied metadata read failed: {code}"),
            });
            (None, None)
        }
    };
    if decision.artists.is_some() {
        return Ok(decision);
    }
    if let Some(artist) = artist.filter(|text| !text.is_empty()) {
        let repaired = fix_jibrish(artist, RepairMode::Hebrew);
        choose_alias(
            &mut decision,
            &repaired,
            &context.aliases,
            DecisionSource::MetadataArtistAlias,
        );
        if decision.artists.is_none()
            && check_artist(&repaired)
            && classify_artist(&mut decision, &context.recognition, &repaired)?
        {
            choose(
                &mut decision,
                vec![repaired],
                DecisionSource::MetadataArtistClassifier,
            );
        }
    }
    if decision.artists.is_some() {
        return Ok(decision);
    }
    // An empty original title suppresses both title lookup and title NER. If
    // cleanup emptied a nonempty title, the original value is retained instead.
    let title = title
        .filter(|text| !text.is_empty())
        .map(|text| {
            recognition_text(text)
                .map(|text| fix_jibrish(&text, RepairMode::Hebrew))
                .ok_or_else(|| invalid_text("title"))
        })
        .transpose()?;
    if let Some(title) = &title {
        choose_alias(
            &mut decision,
            title,
            &context.aliases,
            DecisionSource::TitleAlias,
        );
    }
    if decision.artists.is_some() {
        return Ok(decision);
    }
    if matches!(context.recognition, RecognitionFacts::Configured { .. }) {
        let artists = ner_artists(&mut decision, &context.recognition, stem)?;
        if !artists.is_empty() {
            choose(&mut decision, artists, DecisionSource::FilenameNer);
        } else if let Some(title) = title {
            let artists = ner_artists(&mut decision, &context.recognition, &title)?;
            if !artists.is_empty() {
                choose(&mut decision, artists, DecisionSource::TitleNer);
            }
        }
    }
    Ok(decision)
}

fn choose_alias(decision: &mut Decision, text: &str, aliases: &[Alias], source: DecisionSource) {
    if let Some((index, target)) = first_alias(text, aliases) {
        decision
            .trace
            .push(DecisionEvent::AliasSelected { source, index });
        choose(decision, vec![target.into()], source);
    }
}

fn choose(decision: &mut Decision, artists: Vec<String>, source: DecisionSource) {
    decision.artists = Some(artists);
    decision.selected_by = Some(source);
}

fn recognition_text(text: &str) -> Option<String> {
    let cleaned = clean(text);
    if cleaned.is_empty() {
        Some(text.into())
    } else {
        sanitize(&cleaned)
    }
}

// Python 3.12 pathlib stem: only a non-leading, non-trailing final dot is a
// suffix. '.' path components and separators are ignored by PurePosixPath.
fn legacy_stem(text: &str) -> &str {
    let name = text
        .rsplit('/')
        .find(|part| !part.is_empty() && *part != ".")
        .unwrap_or("");
    match name.rfind('.') {
        Some(index) if index > 0 && index + 1 < name.len() => &name[..index],
        _ => name,
    }
}

fn invalid_text(field: &str) -> EngineError {
    EngineError {
        code: ErrorCode::InvalidInput,
        message: format!(
            "Supplied {field} sanitizes to no string; the release cannot analyze that value"
        ),
    }
}

fn required_recognition(decision: &mut Decision, recognition: &RecognitionFacts) {
    if let RecognitionFacts::Configured {
        health: RecognitionHealth::Failed,
        bundle_id,
        ..
    } = recognition
    {
        block_recognition(
            decision,
            format!("Decision requires supplied failed bundle {bundle_id:?}"),
        );
    }
}

fn block_recognition(decision: &mut Decision, message: String) {
    if !decision
        .apply_blockers
        .iter()
        .any(|diagnostic| diagnostic.code == "RECOGNITION_UNHEALTHY")
    {
        decision.apply_blockers.push(Diagnostic {
            code: "RECOGNITION_UNHEALTHY".into(),
            message,
        });
    }
}

fn missing(operation: &str, text: &str) -> EngineError {
    EngineError {
        code: ErrorCode::MissingInferenceFact,
        message: format!("Missing supplied {operation} fact for exact text {text:?}"),
    }
}

fn classify_artist(
    decision: &mut Decision,
    recognition: &RecognitionFacts,
    text: &str,
) -> Result<bool, EngineError> {
    let RecognitionFacts::Configured { classifier, .. } = recognition else {
        return Ok(false);
    };
    decision
        .trace
        .push(DecisionEvent::ClassifierRequested { text: text.into() });
    required_recognition(decision, recognition);
    let fact = classifier
        .get(text)
        .ok_or_else(|| missing("classifier", text))?;
    match fact {
        ClassifierFact::Accept => Ok(true),
        ClassifierFact::Reject => Ok(false),
        ClassifierFact::Unavailable | ClassifierFact::Error { .. } => {
            let failure = match fact {
                ClassifierFact::Error { code } => format!("error {code:?}"),
                _ => "unavailable".into(),
            };
            decision.warnings.push(Diagnostic {
                code: "CLASSIFIER_FAIL_OPEN".into(),
                message: format!(
                    "Supplied classifier {failure} for {text:?}; legacy acceptance is retained"
                ),
            });
            block_recognition(
                decision,
                format!("Decision depends on failed/unavailable classifier for {text:?}"),
            );
            Ok(true)
        }
    }
}

fn ner_artists(
    decision: &mut Decision,
    recognition: &RecognitionFacts,
    text: &str,
) -> Result<Vec<String>, EngineError> {
    let RecognitionFacts::Configured { ner, .. } = recognition else {
        return Ok(vec![]);
    };
    decision
        .trace
        .push(DecisionEvent::NerRequested { text: text.into() });
    required_recognition(decision, recognition);
    match ner.get(text).ok_or_else(|| missing("NER", text))? {
        NerFact::Artists(artists) => Ok(artists.clone()),
        NerFact::Unavailable => {
            decision.warnings.push(Diagnostic {
                code: "NER_UNAVAILABLE".into(),
                message: format!(
                    "Supplied NER is unavailable for {text:?}; legacy empty result is retained"
                ),
            });
            block_recognition(
                decision,
                format!("Decision depends on unavailable NER for {text:?}"),
            );
            Ok(vec![])
        }
        NerFact::Error { code } => Err(EngineError {
            code: ErrorCode::InferenceError,
            message: format!("Supplied NER invocation failed for {text:?}: {code}"),
        }),
    }
}
