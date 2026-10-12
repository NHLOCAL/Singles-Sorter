//! Borrowed original Album intentions only. This is not execution admission.
//! Provider consumers must separately bind a complete subtree, fresh observations,
//! byte hashes, grants and platform effects. No path is inferred from StorageRef.
use super::*;

#[derive(Debug, Serialize)]
pub struct AlbumChildProjection<'a> {
    pub intent: &'a TransferIntent,
    pub planned_name: &'a str,
    /// Display comparison only, not permission to rename.
    pub name_changes: bool,
}
#[derive(Debug, Serialize)]
pub struct AlbumOperationProjection<'a> {
    pub id: usize,
    pub source: &'a EntryObservation,
    pub destination: &'a Destination,
    pub children: Vec<AlbumChildProjection<'a>>,
    pub depends_on: &'a [usize],
    pub stop_and_replan_on_effect_mismatch: bool,
    pub copy_mode: bool,
    pub remove_empty_source: bool,
}
#[derive(Debug, Serialize)]
pub struct AlbumPlanProjection<'a> {
    pub plan_id: &'a str,
    pub blockers: &'a [String],
    pub operations: Vec<AlbumOperationProjection<'a>>,
}
fn invalid(detail: &str) -> JobError {
    JobError {
        code: "ALBUM_PROJECTION_INVALID".into(),
        detail: detail.into(),
    }
}
fn reserve<T>(values: &mut Vec<T>, additional: usize) -> Result<(), JobError> {
    values.try_reserve(additional).map_err(|_| {
        PlanningAbort {
            kind: AbortKind::Allocation,
            detail: "Album projection allocation failed".into(),
        }
        .into()
    })
}
/// Projects every original Album operation and every ordered direct-child intent,
/// including predicted failures and blockers. It never expands nested children,
/// changes names, resolves collisions, or removes original plan blockers.
pub fn project_album_operations<'a>(
    plan: &'a JobPlanV2,
    control: &dyn PlanningControl,
) -> Result<AlbumPlanProjection<'a>, JobError> {
    control.checkpoint(PlanningBoundary::Start)?;
    verify_job_plan_binding(plan)?;
    let mut operations = Vec::new();
    let mut total_children = 0usize;
    if plan.operations.len() > 10000 {
        return Err(invalid("Operation projection limit"));
    }
    for (index, operation) in plan.operations.iter().enumerate() {
        control.checkpoint(PlanningBoundary::Visit)?;
        if operation.id != index {
            return Err(invalid("Original operation order"));
        }
        let OperationKind::AlbumTransfer {
            source,
            destination,
            children,
            remove_empty_source,
        } = &operation.kind
        else {
            continue;
        };
        if plan.request.mode != JobMode::Sort
            || source.kind != EntryKind::Directory
            || plan.request.target.as_ref() != Some(&destination.root)
            || destination.components.is_empty()
            || (plan.request.settings.copy_mode && *remove_empty_source)
        {
            return Err(invalid("Original Album scope or mode"));
        }
        total_children = total_children
            .checked_add(children.len())
            .ok_or_else(|| invalid("Child projection limit"))?;
        if total_children > 100000 {
            return Err(invalid("Child projection limit"));
        }
        let mut projected = Vec::new();
        reserve(&mut projected, children.len())?;
        for child in children {
            control.checkpoint(PlanningBoundary::File)?;
            if child.copy != plan.request.settings.copy_mode
                || child.destination.root != destination.root
                || child.destination.components.len() != destination.components.len() + 1
                || !child
                    .destination
                    .components
                    .starts_with(&destination.components)
            {
                return Err(invalid("Original child destination or mode"));
            }
            let name = child
                .destination
                .components
                .last()
                .ok_or_else(|| invalid("Missing original child name"))?;
            projected.push(AlbumChildProjection {
                intent: child,
                planned_name: name,
                name_changes: name != &child.source.name,
            });
        }
        reserve(&mut operations, 1)?;
        operations.push(AlbumOperationProjection {
            id: operation.id,
            source,
            destination,
            children: projected,
            depends_on: &operation.depends_on,
            stop_and_replan_on_effect_mismatch: operation.stop_and_replan_on_effect_mismatch,
            copy_mode: plan.request.settings.copy_mode,
            remove_empty_source: *remove_empty_source,
        });
    }
    control.checkpoint(PlanningBoundary::Result)?;
    Ok(AlbumPlanProjection {
        plan_id: &plan.binding_sha256,
        blockers: &plan.blockers,
        operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: &str, name: &str, kind: EntryKind) -> EntryObservation {
        EntryObservation {
            reference: StorageRef::TestOpaque { id: id.into() },
            identity: None,
            parent_identity: None,
            name: name.into(),
            kind,
            size: None,
            modified: None,
            capabilities: Capabilities::default(),
        }
    }
    fn fixture(copy: bool) -> JobPlanV2 {
        let source = entry("source", "album", EntryKind::Directory);
        let target = StorageRef::TestOpaque {
            id: "target".into(),
        };
        let destination = Destination {
            root: target.clone(),
            components: vec!["artist".into(), "album".into()],
        };
        let children = [
            ("song", "song?.mp3", "song.mp3", EntryKind::File, true),
            ("nested", "nested", "nested", EntryKind::Directory, true),
            ("link", "link", "link", EntryKind::LinkOrReparse, false),
        ]
        .into_iter()
        .map(|(id, old, name, kind, success)| {
            let mut child_destination = destination.clone();
            child_destination.components.push(name.into());
            TransferIntent {
                source: entry(id, old, kind),
                destination: child_destination,
                copy,
                predicted_success: success,
                blockers: if success {
                    vec![]
                } else {
                    vec!["LINK_OR_REPARSE".into()]
                },
            }
        })
        .collect();
        let mut plan = JobPlanV2 {
            schema_version: 2,
            ruleset_id: "fixture".into(),
            binding_sha256: String::new(),
            content_binding: "fixture".into(),
            executable: false,
            request: JobRequestV2 {
                mode: JobMode::Sort,
                source: source.reference.clone(),
                target: Some(target),
                settings: Settings {
                    copy_mode: copy,
                    ..Settings::default()
                },
                alias_rows: vec![],
                resource_ids: vec![],
                adapter_versions: vec![],
                ai_profile: AiProfile::Disabled,
            },
            observations: vec![source.clone()],
            metadata_observations: vec![],
            precollected_visits: vec![],
            decisions: vec![],
            operations: vec![OperationV2 {
                id: 0,
                kind: OperationKind::AlbumTransfer {
                    source,
                    destination,
                    children,
                    remove_empty_source: false,
                },
                depends_on: vec![],
                stop_and_replan_on_effect_mismatch: true,
            }],
            predicted_effects: vec![],
            predicted_legacy_counters: PredictedCounters::default(),
            warnings: vec![],
            blockers: vec!["PHYSICAL_APPLY_UNAVAILABLE".into()],
        };
        plan.binding_sha256 = binding_digest(&plan).unwrap();
        plan
    }
    #[test]
    fn borrows_exact_order_names_failures_and_original_blockers() {
        let plan = fixture(false);
        let projected = project_album_operations(&plan, &UncontrolledPlanning).unwrap();
        let OperationKind::AlbumTransfer { children, .. } = &plan.operations[0].kind else {
            panic!()
        };
        let album = &projected.operations[0];
        assert_eq!(album.children.len(), 3);
        for (actual, original) in album.children.iter().zip(children) {
            assert!(std::ptr::eq(actual.intent, original));
        }
        assert_eq!(album.children[0].planned_name, "song.mp3");
        assert!(album.children[0].name_changes);
        assert!(!album.children[1].name_changes);
        assert!(!album.children[2].intent.predicted_success);
        assert_eq!(album.children[2].intent.blockers, ["LINK_OR_REPARSE"]);
        assert_eq!(projected.blockers, plan.blockers);
        assert_eq!(projected.plan_id, plan.binding_sha256);
        assert!(!album.remove_empty_source);
        assert!(!album.copy_mode);
    }
    #[test]
    fn copy_flags_and_binding_integrity_are_preserved() {
        let mut plan = fixture(true);
        let projected = project_album_operations(&plan, &UncontrolledPlanning).unwrap();
        assert!(projected.operations[0].copy_mode);
        assert!(!projected.operations[0].remove_empty_source);
        plan.operations[0].id = 1;
        assert_eq!(
            project_album_operations(&plan, &UncontrolledPlanning)
                .unwrap_err()
                .code,
            "JOB_BINDING_MISMATCH"
        );
    }
    #[test]
    fn malformed_retained_child_prefix_is_refused_without_replanning() {
        let mut plan = fixture(false);
        let OperationKind::AlbumTransfer { children, .. } = &mut plan.operations[0].kind else {
            panic!()
        };
        children[0].destination.components[0] = "different".into();
        plan.binding_sha256 = binding_digest(&plan).unwrap();
        assert_eq!(
            project_album_operations(&plan, &UncontrolledPlanning)
                .unwrap_err()
                .code,
            "ALBUM_PROJECTION_INVALID"
        );
    }
    #[test]
    fn cancellation_before_start_and_at_final_return_never_returns_projection() {
        struct FinalCancel;
        impl PlanningControl for FinalCancel {
            fn checkpoint(&self, boundary: PlanningBoundary) -> Result<(), PlanningAbort> {
                if boundary == PlanningBoundary::Result {
                    Err(PlanningAbort::cancelled())
                } else {
                    Ok(())
                }
            }
        }
        let plan = fixture(false);
        let token = CancellationToken::new();
        token.cancel();
        for control in [&token as &dyn PlanningControl, &FinalCancel] {
            assert_eq!(
                project_album_operations(&plan, control).unwrap_err().code,
                "PLANNING_CANCELLED"
            );
        }
    }
}
