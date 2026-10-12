//! Strict admission for a freshly revalidated, effect-free Sort completion.
//! This does not prepare a mutation binding or authorize any filesystem effects.
use sorter_core::{
    album::AlbumTraceEvent,
    job_plan::{DecisionV2, JobMode, JobPlanV2, PredictedEffect, verify_job_plan_binding},
    metadata::MetadataObservation,
};
use std::collections::HashMap;

const HISTORICAL_DELETE: &str = "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS";

/// The original all-skipped duet deletion is historical evidence only. Accept
/// precisely that empty, blocked intention as a source-retaining no-op, never
/// as an executable effect. Every source must match its actual duet decision.
fn only_empty_historical_duets(plan: &JobPlanV2) -> bool {
    if plan.predicted_effects.is_empty() {
        return !plan.blockers.iter().any(|blocker| blocker == HISTORICAL_DELETE);
    }
    let settings = &plan.request.settings;
    if settings.copy_mode || !settings.duet_mode || !settings.exist_only {
        return false;
    }
    let mut sources = HashMap::new();
    for decision in &plan.decisions {
        if let DecisionV2::Single { read, artists } = decision
            && artists.len() > 1
        {
            if !read.virtually_present || read.after_operations != 0 || !read.depends_on.is_empty() {
                return false;
            }
            if sources.insert(read.source.reference.clone(), &read.source).is_some() {
                return false;
            }
        }
    }
    plan.blockers.iter().any(|blocker| blocker == HISTORICAL_DELETE)
        && plan.predicted_effects.iter().all(|effect| {
            matches!(effect, PredictedEffect::DuetDeleteIntention { source, after_operations, blocked }
                if *blocked && after_operations.is_empty()
                    && sources.remove(&source.reference).is_some_and(|observed| observed == source))
        })
        && sources.is_empty()
}

/// A caller must retain the inspected plan, re-read the same roots/resources,
/// and compare the fresh plan binding before returning a successful zero result.
/// Empty operation lists containing failures or unexecuted effects are refused.
pub fn is_clean_sort_noop(plan: &JobPlanV2) -> bool {
    plan.request.mode == JobMode::Sort
        && plan.request.target.is_some()
        && plan.ruleset_id == sorter_core::RULESET_ID
        && verify_job_plan_binding(plan).is_ok()
        && plan.operations.is_empty()
        && plan.predicted_legacy_counters == Default::default()
        && only_empty_historical_duets(plan)
        && plan
            .blockers
            .iter()
            .all(|blocker| blocker == "PHYSICAL_APPLY_UNAVAILABLE" || blocker == HISTORICAL_DELETE)
        && plan.warnings.iter().all(|warning| {
            matches!(
                warning.as_str(),
                "Effects and counters are predicted; no files or tags were changed"
                    | "AI intentionally disabled; no native inference bundle is used"
            )
        })
        && plan.decisions.iter().all(|decision| match decision {
            DecisionV2::ItemError { .. } | DecisionV2::Vanished { .. } => false,
            DecisionV2::Single { read, .. } => read.virtually_present,
            DecisionV2::Album { read, decision } => {
                read.virtually_present
                    && decision.trace.iter().all(|event| {
                        !matches!(
                            event,
                            AlbumTraceEvent::Open { result: Err(_), .. }
                                | AlbumTraceEvent::Get { result: Err(_), .. }
                                | AlbumTraceEvent::Value { result: Err(_), .. }
                                | AlbumTraceEvent::FileError { .. }
                                | AlbumTraceEvent::ProtectedError { .. }
                        )
                    })
            }
        })
        && plan
            .metadata_observations
            .iter()
            .all(|observation| match observation {
                MetadataObservation::Open { result, .. } => result.is_ok(),
                MetadataObservation::Get { result, .. } => {
                    result.as_ref().is_ok_and(|item| item.value.is_ok())
                }
            })
}
