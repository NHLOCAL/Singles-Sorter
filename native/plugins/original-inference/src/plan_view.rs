use serde::Serialize;
use sorter_core::{
    discovery::{EntryObservation, StorageRef},
    job_plan::{DecisionV2, JobMode, JobPlanV2, OperationKind},
};
#[derive(Serialize)]
pub struct DisplayRow {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub destination: Option<String>,
    pub reason: String,
    pub blockers: Vec<String>,
}
fn entry_display(entry: &EntryObservation) -> String {
    match &entry.reference {
        StorageRef::WindowsPath { utf16 } => String::from_utf16_lossy(utf16),
        // Opaque IDs remain authority only, not filenames. Android display names
        // and Linux test names are never reconstructed into storage references.
        _ => entry.name.clone(),
    }
}
fn operation_source(kind: &OperationKind) -> &EntryObservation {
    match kind {
        OperationKind::AlbumTransfer { source, .. }
        | OperationKind::SingleTransfer { source, .. } => source,
        OperationKind::FixFile(f) => &f.source,
    }
}
pub fn project_rows(plan: &JobPlanV2) -> Vec<DisplayRow> {
    if plan.request.mode == JobMode::FixNames {
        return plan
            .operations
            .iter()
            .map(|op| match &op.kind {
                OperationKind::FixFile(f) => DisplayRow {
                    id: format!("operation-{}", op.id),
                    source: entry_display(&f.source),
                    kind: "FixNames".into(),
                    destination: f.intended_name.clone(),
                    reason: format!(
                        "rename_intended={}; save_intended={}; repairs={}; failure={:?}",
                        f.rename_intended,
                        f.save_intended,
                        f.repairs.len(),
                        f.failure
                    ),
                    blockers: f.blockers.iter().map(|b| format!("{b:?}")).collect(),
                },
                _ => unreachable!("fix planner only emits fix operations"),
            })
            .collect();
    }
    let mut operations_by_source =
        std::collections::HashMap::<&StorageRef, Vec<&sorter_core::job_plan::OperationV2>>::new();
    for operation in &plan.operations {
        operations_by_source
            .entry(&operation_source(&operation.kind).reference)
            .or_default()
            .push(operation);
    }
    plan.decisions
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let (read, kind, reason) = match d {
                DecisionV2::Album { read, decision } => (
                    read,
                    "Album",
                    if !decision.is_album {
                        "התיקייה אינה אלבום לפי כללי המיון".into()
                    } else if !decision.should_process {
                        "זוהה אלבום שאינו מיועד לעיבוד לפי הכללים".into()
                    } else {
                        format!(
                            "אלבום: {}; אמן: {}",
                            decision.album_name.as_deref().unwrap_or(""),
                            decision.artist_name.as_deref().unwrap_or("")
                        )
                    },
                ),
                DecisionV2::Single { read, artists } => (
                    read,
                    "Single",
                    if artists.is_empty() {
                        "לא נבחר אמן לפי כללי המיון".into()
                    } else {
                        artists.join("; ")
                    },
                ),
                DecisionV2::ItemError { read, error } => (read, "ItemError", error.clone()),
                DecisionV2::Vanished { read } => {
                    (read, "Vanished", "המקור אינו זמין בשלב זה של התכנון".into())
                }
            };
            let mut destinations = vec![];
            let mut blockers = vec![];
            for op in operations_by_source
                .get(&read.source.reference)
                .into_iter()
                .flatten()
            {
                match &op.kind {
                    OperationKind::SingleTransfer { transfer, .. } => {
                        destinations.push(transfer.destination.components.join(" / "));
                        blockers.extend(transfer.blockers.clone());
                    }
                    OperationKind::AlbumTransfer {
                        destination,
                        children,
                        ..
                    } => {
                        destinations.push(destination.components.join(" / "));
                        for c in children {
                            blockers.extend(c.blockers.clone());
                        }
                    }
                    _ => {}
                }
            }
            DisplayRow {
                id: format!("decision-{i}"),
                source: entry_display(&read.source),
                kind: kind.into(),
                destination: (!destinations.is_empty()).then(|| destinations.join("; ")),
                reason,
                blockers,
            }
        })
        .collect()
}
