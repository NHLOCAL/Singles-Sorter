use super::*;

fn ensure_children(
    state: &mut State,
    index: usize,
    storage: &mut dyn ReadStorage,
) -> Result<(), JobError> {
    let entry = state.source.nodes[index].entry.clone();
    let actual = storage.children(&entry.reference)?;
    let expected = state.source.nodes[index].children.clone();
    if state.source.nodes[index].listed {
        let old = expected
            .iter()
            .map(|&i| &state.source.nodes[i].entry)
            .collect::<Vec<_>>();
        if old.iter().zip(&actual).any(|(a, b)| **a != *b) || old.len() != actual.len() {
            return Err(JobError {
                code: "CHANGED_ENUMERATION".into(),
                detail: "Source listing changed between collection and logical read point".into(),
            });
        }
        return Ok(());
    }
    state.source.nodes[index].listed = true;
    for child in actual {
        let mut path = state.source.nodes[index].path.clone();
        path.push(child.name.clone());
        let c = state.source.nodes.len();
        state.plan.observations.push(child.clone());
        state.source.nodes.push(Node {
            entry: child,
            path,
            children: vec![],
            listed: false,
        });
        state.source.nodes[index].children.push(c);
    }
    Ok(())
}
fn subtree(state: &mut State, index: usize, storage: &mut dyn ReadStorage) -> Result<(), JobError> {
    if state.source.nodes[index].entry.kind == EntryKind::Directory {
        ensure_children(state, index, storage)?;
        for c in state.source.nodes[index].children.clone() {
            subtree(state, c, storage)?;
        }
    }
    Ok(())
}
pub(super) fn sort(
    state: &mut State,
    visits: Vec<usize>,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
) -> Result<(), JobError> {
    let enabled = !matches!(state.plan.request.ai_profile, AiProfile::Disabled);
    let mut singles = vec![];
    for i in visits {
        let read = state.read(i);
        if !read.virtually_present {
            state.plan.decisions.push(DecisionV2::Vanished { read });
            continue;
        }
        let entry = state.source.nodes[i].entry.clone();
        let children = if entry.kind == EntryKind::Directory {
            ensure_children(state, i, storage)?;
            let direct = state.source.nodes[i]
                .children
                .iter()
                .filter(|c| !state.removed.contains_key(c))
                .map(|&c| state.source.nodes[c].entry.clone())
                .collect();
            let decision = analyze_album(
                &FolderObservation {
                    entry: entry.clone(),
                    direct_children: direct,
                },
                metadata,
            );
            state.plan.decisions.push(DecisionV2::Album {
                read: read.clone(),
                decision: decision.clone(),
            });
            if decision.is_album {
                if decision.should_process
                    && let Err(error) = album(state, i, &decision, storage)
                {
                    state
                        .plan
                        .decisions
                        .push(DecisionV2::ItemError { read, error });
                }
                continue;
            }
            state.source.nodes[i].children.clone()
        } else if entry.kind == EntryKind::File {
            vec![i]
        } else {
            vec![]
        };
        for c in children {
            let file = state.source.nodes[c].entry.clone();
            if state.removed.contains_key(&c) || file.kind != EntryKind::File || !audio(&file.name)
            {
                continue;
            }
            let read = state.read(c);
            match single_decision(
                &file,
                &state.plan.request.alias_rows,
                metadata,
                recognition,
                enabled,
            ) {
                Ok(artists) => {
                    state.plan.decisions.push(DecisionV2::Single {
                        read,
                        artists: artists.clone(),
                    });
                    if !artists.is_empty() {
                        singles.push((c, artists));
                    }
                }
                Err(error) => {
                    state
                        .plan
                        .decisions
                        .push(DecisionV2::ItemError { read, error });
                    break;
                } // release catches per scan item, not per song
            }
        }
    }
    // Do not interleave these transfers with collection/album effects.
    'single_files: for (i, mut artists) in singles {
        if !state.plan.request.settings.duet_mode {
            artists.truncate(1);
        }
        let duet = state.plan.request.settings.duet_mode && artists.len() > 1;
        let mut operations = vec![];
        for artist in artists {
            if artist.is_empty() && state.plan.request.settings.abc_sort {
                state.plan.decisions.push(DecisionV2::ItemError {
                    read: state.read(i),
                    error: "IndexError: empty artist has no first character".into(),
                });
                continue 'single_files;
            }
            let settings = &state.plan.request.settings;
            let mut directory = artist_directory(&artist, settings.abc_sort);
            if settings.singles_folder {
                directory.push("סינגלים".into());
            }
            if settings.exist_only
                && !matches!(
                    state.target.get(&directory),
                    Some((EntryKind::Directory, _))
                )
            {
                continue;
            }
            if matches!(state.target.get(&directory),Some((kind,_)) if *kind!=EntryKind::Directory)
            {
                continue;
            }
            if !state.can_mkdir(&directory) {
                continue;
            }
            let Some(filename) = crate::sanitize(&state.source.nodes[i].entry.name) else {
                continue;
            };
            let mut destination = directory.clone();
            destination.push(filename.clone());
            let mut counter = 1;
            while state.target.contains_key(&destination) {
                destination.pop();
                destination.push(unique_name(&filename, counter));
                counter += 1;
            }
            let mut blockers = state.destination_blockers(&destination);
            let file = state.source.nodes[i].entry.clone();
            if file.identity.is_none() {
                blockers.push("MISSING_IDENTITY".into());
            }
            let copy = settings.copy_mode || duet;
            let intent = TransferIntent {
                source: file.clone(),
                destination: state.destination(destination.clone()),
                copy,
                predicted_success: true,
                blockers,
            };
            let new_directory = !state.target.contains_key(&directory);
            let id = state.op(OperationKind::SingleTransfer {
                source: file,
                artist: artist.clone(),
                transfer: Box::new(intent),
            });
            operations.push(id);
            state.mkdir(&directory, id);
            if new_directory {
                state.created_artists.insert(artist.clone());
            }
            state
                .target
                .insert(destination.clone(), (EntryKind::File, Some(id)));
            state
                .plan
                .predicted_effects
                .push(PredictedEffect::TargetCreated {
                    operation: id,
                    destination: state.destination(destination),
                    kind: EntryKind::File,
                });
            if !copy {
                state.remove(i, id);
            }
            state.plan.predicted_legacy_counters.songs_sorted += 1;
            state.count(&artist, 1);
        }
        if duet && !state.plan.request.settings.copy_mode {
            state
                .plan
                .predicted_effects
                .push(PredictedEffect::DuetDeleteIntention {
                    source: state.source.nodes[i].entry.clone(),
                    after_operations: operations,
                    blocked: true,
                });
            state
                .plan
                .blockers
                .push("HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS".into());
        }
    }
    Ok(())
}
fn artist_directory(artist: &str, abc: bool) -> Vec<String> {
    let mut p = vec![];
    if abc {
        p.push(
            artist
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default(),
        );
    }
    if !artist.is_empty() {
        p.push(artist.into());
    }
    p
}
fn unique_name(name: &str, n: usize) -> String {
    let stem = singles::stem(name);
    format!("{stem}_{n}{}", &name[stem.len()..])
}
fn album(
    state: &mut State,
    index: usize,
    decision: &AlbumDecision,
    storage: &mut dyn ReadStorage,
) -> Result<(), String> {
    let (Some(name), Some(artist)) = (&decision.album_name, &decision.artist_name) else {
        return Ok(());
    };
    if name.is_empty() || artist.is_empty() || !crate::check_artist(artist) {
        return Ok(());
    }
    let Some(name) = crate::sanitize(name) else {
        return Ok(());
    };
    let final_artist = singles::alias(artist, &state.plan.request.alias_rows)?
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| artist.clone());
    let directory = artist_directory(&final_artist, state.plan.request.settings.abc_sort);
    let mut destination = directory.clone();
    destination.push(name);
    if state.target.contains_key(&destination) {
        return Ok(());
    }
    if state.plan.request.settings.exist_only && !state.target.contains_key(&directory) {
        return Ok(());
    }
    if matches!(state.target.get(&directory),Some((kind,_)) if *kind!=EntryKind::Directory) {
        return Ok(());
    }
    if !state.can_mkdir(&destination) {
        return Ok(());
    }
    subtree(state, index, storage).map_err(|e| e.detail)?;
    let copy = state.plan.request.settings.copy_mode;
    let id = state.plan.operations.len();
    state.mkdir(&destination, id);
    let mut children = vec![];
    for child in state.source.nodes[index].children.clone() {
        let entry = state.source.nodes[child].entry.clone();
        let Some(name) = crate::sanitize(&entry.name) else {
            continue;
        };
        let mut path = destination.clone();
        path.push(name);
        let mut blockers = state.destination_blockers(&path);
        let collision = state.target.get(&path).copied();
        if collision.is_some() {
            blockers.push("ALBUM_SANITIZATION_COLLISION_NO_SUFFIX".into());
        }
        let success = match collision {
            None => entry.kind == EntryKind::File || entry.kind == EntryKind::Directory,
            Some((EntryKind::File, _)) => entry.kind == EntryKind::File,
            _ => false,
        };
        if entry.kind == EntryKind::LinkOrReparse {
            blockers.push("LINK_OR_REPARSE".into());
        }
        if collision.is_some_and(|(kind, _)| kind == EntryKind::Directory) && !copy {
            blockers.push("PLATFORM_DEPENDENT_DIRECTORY_MOVE_COLLISION".into());
        }
        children.push(TransferIntent {
            source: entry,
            destination: state.destination(path.clone()),
            copy,
            predicted_success: success,
            blockers,
        });
        if success {
            copy_subtree(state, child, &path, id);
            if !copy {
                state.remove(child, id);
            }
        }
    }
    let remove_empty_source = !copy
        && state.source.nodes[index]
            .children
            .iter()
            .all(|c| state.removed.contains_key(c));
    let source = state.source.nodes[index].entry.clone();
    state.op(OperationKind::AlbumTransfer {
        source,
        destination: state.destination(destination.clone()),
        children,
        remove_empty_source,
    });
    if remove_empty_source {
        state.remove(index, id);
    }
    let files = state
        .target
        .iter()
        .filter(|(p, (kind, _))| {
            p.len() == destination.len() + 1
                && p.starts_with(&destination)
                && *kind == EntryKind::File
        })
        .count();
    state.plan.predicted_legacy_counters.albums_processed += 1;
    state.count(artist, files);
    Ok(())
}
fn copy_subtree(state: &mut State, index: usize, path: &[String], id: usize) {
    let kind = state.source.nodes[index].entry.kind;
    state.target.insert(path.to_vec(), (kind, Some(id)));
    state
        .plan
        .predicted_effects
        .push(PredictedEffect::TargetCreated {
            operation: id,
            destination: state.destination(path.to_vec()),
            kind,
        });
    for c in state.source.nodes[index].children.clone() {
        let mut p = path.to_vec();
        p.push(state.source.nodes[c].entry.name.clone());
        copy_subtree(state, c, &p, id);
    }
}
pub(super) fn fix(state: &mut State, metadata: &mut dyn MetadataProvider) -> Result<(), JobError> {
    let parents = if state.plan.request.settings.main_folder_only {
        vec![0]
    } else {
        state.source.visits.clone()
    };
    let files = parents
        .iter()
        .flat_map(|&i| state.source.nodes[i].children.clone())
        .filter(|&i| audio(&state.source.nodes[i].entry.name))
        .collect::<Vec<_>>();
    state.plan.precollected_visits = files
        .iter()
        .map(|&i| state.source.nodes[i].entry.reference.clone())
        .collect();
    let mut occupied = state
        .source
        .nodes
        .iter()
        .map(|n| (n.path.clone(), n.entry.kind))
        .collect::<BTreeMap<_, _>>();
    for i in files {
        let node = state.source.nodes[i].clone();
        let mut parent = node.path.clone();
        parent.pop();
        let names = occupied
            .keys()
            .filter(|p| p.len() == parent.len() + 1 && p.starts_with(&parent))
            .filter_map(|p| p.last().cloned())
            .collect::<Vec<_>>();
        let unavailable = state.removed.contains_key(&i);
        let mut result = if unavailable {
            state.plan.decisions.push(DecisionV2::Vanished {
                read: state.read(i),
            });
            struct Missing;
            impl crate::metadata::MetadataProvider for Missing {
                fn open(
                    &mut self,
                    _: &EntryObservation,
                ) -> Result<Box<dyn crate::metadata::MetadataFields>, crate::metadata::MetadataError>
                {
                    Err(crate::metadata::MetadataError::LegacyOpen{kind:"FileNotFoundError".into(),detail:"Original logical path vanished after an earlier predicted directory rename".into()})
                }
            }
            crate::fix_plan::fix_file(&node.entry, &names, &mut Missing)
        } else {
            crate::fix_plan::fix_file(&node.entry, &names, metadata)
        };
        if result.intended_name.as_ref().is_some_and(|name| {
            names
                .iter()
                .any(|other| other != name && other.to_lowercase() == name.to_lowercase())
        }) {
            result
                .blockers
                .push(crate::fix_plan::FixBlocker::CaseCollisionRequiresProviderResolution);
        }
        let renamed = !unavailable && result.rename_intended;
        if renamed && let Some(name) = &result.intended_name {
            let mut p = parent;
            p.push(name.clone());
            // Rename moves the entire directory namespace; precollected old
            // descendant paths do not magically rebind to their new location.
            let moved = occupied
                .iter()
                .filter(|(path, _)| path.starts_with(&node.path))
                .map(|(p, k)| (p.clone(), *k))
                .collect::<Vec<_>>();
            for (old, kind) in moved {
                occupied.remove(&old);
                let mut new = p.clone();
                new.extend_from_slice(&old[node.path.len()..]);
                occupied.insert(new, kind);
            }
        }
        let renamed_name = result.intended_name.clone();
        let op = state.op(OperationKind::FixFile(result));
        if renamed {
            state
                .plan
                .predicted_effects
                .push(PredictedEffect::SourceRenamed {
                    operation: op,
                    source: node.entry.clone(),
                    intended_name: renamed_name.expect("rename requires a name"),
                });
        }
        if renamed && node.entry.kind == EntryKind::Directory {
            for child in node.children {
                state.remove(child, op);
            }
        }
    }
    Ok(())
}
