//! Strict admission for a freshly revalidated, effect-free Sort completion.
//! This does not prepare a mutation binding or authorize any filesystem effects.
use sorter_core::{
    album::AlbumTraceEvent,
    job_plan::{DecisionV2, JobMode, JobPlanV2, verify_job_plan_binding},
    metadata::MetadataObservation,
};

/// A caller must retain the inspected plan, re-read the same roots/resources,
/// and compare the fresh plan binding before returning a successful zero result.
/// Empty operation lists containing failures or unexecuted effects are refused.
pub fn is_clean_sort_noop(plan: &JobPlanV2) -> bool {
    plan.request.mode == JobMode::Sort
        && plan.request.target.is_some()
        && plan.ruleset_id == sorter_core::RULESET_ID
        && verify_job_plan_binding(plan).is_ok()
        && plan.operations.is_empty()
        && plan.predicted_effects.is_empty()
        && plan
            .blockers
            .iter()
            .all(|blocker| blocker == "PHYSICAL_APPLY_UNAVAILABLE")
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
