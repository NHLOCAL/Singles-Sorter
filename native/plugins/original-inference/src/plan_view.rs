use serde::Serialize;
use sorter_core::{
    album::AlbumTraceEvent,
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
    pub operation_ids: Vec<usize>,
    pub artist: Option<String>,
    pub developer_only: bool,
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
fn routine_inspection(decision: &DecisionV2, operation_ids: &[usize]) -> bool {
    match decision {
        DecisionV2::Album { decision, .. } => {
            !decision.is_album
                && operation_ids.is_empty()
                && !decision.trace.iter().any(|event| {
                    matches!(event, AlbumTraceEvent::ProtectedError { .. })
                })
        }
        _ => false,
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
                    operation_ids: vec![op.id],
                    artist: None,
                    developer_only: false,
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
            let artist = match d {
                DecisionV2::Album { decision, .. } => decision.artist_name.clone(),
                DecisionV2::Single { artists, .. } if !artists.is_empty() => {
                    Some(artists.join("; "))
                }
                _ => None,
            };
            let operation_ids: Vec<usize> = operations_by_source
                .get(&read.source.reference)
                .into_iter()
                .flatten()
                .map(|op| op.id)
                .collect();
            let developer_only = routine_inspection(d, &operation_ids);
            let mut destinations = vec![];
            let mut blockers = vec![];
            if let DecisionV2::Album { decision, .. } = d {
                if decision
                    .trace
                    .iter()
                    .any(|event| matches!(event, AlbumTraceEvent::ProtectedError { .. }))
                {
                    blockers.push("ALBUM_READ_ERROR".into());
                }
            }
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
                operation_ids,
                artist,
                developer_only,
                source: entry_display(&read.source),
                kind: kind.into(),
                destination: (!destinations.is_empty()).then(|| destinations.join("; ")),
                reason,
                blockers,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sorter_core::{
        album::AlbumDecision,
        discovery::{Capabilities, EntryKind},
        job_plan::ReadPoint,
    };

    #[test]
    fn only_routine_negative_album_inspections_are_developer_only() {
        let read = ReadPoint {
            source: EntryObservation {
                reference: StorageRef::TestOpaque {
                    id: "owned".into(),
                },
                identity: None,
                parent_identity: None,
                name: "owned".into(),
                kind: EntryKind::Directory,
                size: None,
                modified: None,
                capabilities: Capabilities::default(),
            },
            after_operations: 0,
            depends_on: vec![],
            virtually_present: true,
        };
        let mut album = AlbumDecision {
            is_album: false,
            should_process: false,
            album_name: None,
            artist_name: None,
            trace: vec![],
        };
        let decision = DecisionV2::Album {
            read: read.clone(),
            decision: album.clone(),
        };
        assert!(routine_inspection(&decision, &[]));
        assert!(!routine_inspection(&decision, &[7]));
        album.is_album = true;
        assert!(!routine_inspection(
            &DecisionV2::Album {
                read: read.clone(),
                decision: album.clone(),
            },
            &[],
        ));
        album.is_album = false;
        album.trace.push(AlbumTraceEvent::ProtectedError {
            kind: "metadata".into(),
        });
        assert!(!routine_inspection(
            &DecisionV2::Album {
                read: read.clone(),
                decision: album,
            },
            &[],
        ));
        assert!(!routine_inspection(
            &DecisionV2::ItemError {
                read: read.clone(),
                error: "owned error".into(),
            },
            &[],
        ));
        assert!(!routine_inspection(
            &DecisionV2::Vanished { read: read.clone() },
            &[],
        ));
        assert!(!routine_inspection(
            &DecisionV2::Single {
                read,
                artists: vec![],
            },
            &[],
        ));
    }
}
