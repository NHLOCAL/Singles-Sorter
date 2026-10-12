//! Read-only evidence of fresh Apply-preflight policy acceptance. This is not an
//! interleaved effect trace, a serialized grant, or a claim of transfer success.
use crate::{ExecutionError as E, Result};
use serde::Serialize;
use sorter_core::{discovery::StorageRef, job_plan::{DecisionV2, JobPlanV2, OperationKind}};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, Serialize)]
pub struct SingleRoute {
    source: StorageRef,
    artist: String,
    operation_ids: Vec<usize>,
}

/// Only successful fresh validation constructs this value. Deserialization and
/// public construction are deliberately unavailable. Empty route IDs mean no
/// admitted transfer; they do not assert a particular execution-time skip reason.
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionDecisions {
    schema_version: u16,
    basis: &'static str,
    plan_binding_sha256: String,
    decisions: Vec<DecisionV2>,
    single_routes: Vec<SingleRoute>,
}
impl ExecutionDecisions {
    pub(crate) fn fresh(plan: &JobPlanV2, no_op: bool, cancel: Option<&std::sync::atomic::AtomicBool>) -> Result<Self> {
        let checkpoint = || {
            if cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire)) { Err(E::Cancelled) } else { Ok(()) }
        };
        checkpoint()?;
        let mut operations: HashMap<(StorageRef, String), VecDeque<usize>> = HashMap::new();
        for operation in &plan.operations {
            checkpoint()?;
            if let OperationKind::SingleTransfer {source, artist, ..} = &operation.kind {
                operations.entry((source.reference.clone(), artist.clone())).or_default().push_back(operation.id);
            }
        }
        let mut routes = vec![];
        for decision in &plan.decisions {
            checkpoint()?;
            let DecisionV2::Single {read, artists} = decision else { continue };
            let count = if plan.request.settings.duet_mode { artists.len() } else { artists.len().min(1) };
            for artist in artists.iter().take(count) {
                checkpoint()?;
                let key = (read.source.reference.clone(), artist.clone());
                routes.push(SingleRoute {
                    source: key.0.clone(), artist: artist.clone(),
                    operation_ids: operations.get_mut(&key).and_then(VecDeque::pop_front).into_iter().collect(),
                });
            }
        }
        if operations.values().any(|ids| !ids.is_empty()) || (no_op && !plan.operations.is_empty()) { return Err(E::InvalidBinding); }
        Ok(Self {
            schema_version: 1,
            basis: if no_op { "fresh_noop_preflight" } else { "fresh_apply_preflight" },
            plan_binding_sha256: plan.binding_sha256.clone(),
            decisions: plan.decisions.clone(),
            single_routes: routes,
        })
    }
}
