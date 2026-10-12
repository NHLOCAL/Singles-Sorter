//! Frozen v14 counters projected from acknowledged effects, never predictions.
//!
//! The selected Windows driver validates the retained operation list and keeps
//! albums before deferred singles. Album credits are the driver's completed
//! immediate-file counts under the original, pre-alias artist. Namespace effects
//! alone are not success receipts; callers supply only committed operation IDs.
use serde::Serialize;
use sorter_core::{
    discovery::StorageRef,
    job_plan::{Destination, OperationKind, OperationV2},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LegacySummary {
    pub songs_sorted: usize,
    pub artist_folders_created: usize,
    pub albums_processed: usize,
    pub top_artists: Vec<(String, usize)>,
}

impl LegacySummary {
    /// This projection grants no execution authority and does no filesystem I/O.
    /// Directory references must come from synced creation acknowledgments, not
    /// planned directories or a later walk of the destination tree.
    pub fn from_receipts(
        operations: &[OperationV2],
        committed_operations: &[usize],
        created_directories: &[StorageRef],
        albums_processed: usize,
        album_artist_files: &[(String, usize)],
    ) -> Self {
        let by_id = operations.iter().map(|op| (op.id, op)).collect::<HashMap<_, _>>();
        let mut artists = Vec::<(String, usize)>::new();
        let mut positions = HashMap::<String, usize>::new();
        let mut credit = |artist: &str, count: usize| {
            let index = *positions.entry(artist.to_owned()).or_insert_with(|| {
                artists.push((artist.to_owned(), 0));
                artists.len() - 1
            });
            artists[index].1 += count;
        };
        // V14 completes album handling before processing its deferred singles.
        // A zero-file completed album still inserts its original artist key.
        for (artist, count) in album_artist_files {
            credit(artist, *count);
        }
        let mut songs_sorted = 0;
        for operation in committed_operations {
            if let Some(OperationV2 {
                kind: OperationKind::SingleTransfer { artist, .. },
                ..
            }) = by_id.get(operation).copied()
            {
                songs_sorted += 1;
                credit(artist, 1);
            }
        }
        let mut created = created_directories.iter().cloned().collect::<HashSet<_>>();
        let mut album_parents = HashSet::new();
        for operation in operations {
            if let OperationKind::AlbumTransfer { destination, .. } = &operation.kind {
                for count in 1..=destination.components.len() {
                    if let Some(reference) = windows_reference(destination, count) {
                        album_parents.insert(reference);
                    }
                }
            }
        }
        let mut folder_artists = HashSet::new();
        for operation in operations {
            if let OperationKind::SingleTransfer { artist, transfer, .. } = &operation.kind
                && let Some(parent) = transfer.destination.components.len().checked_sub(1)
                    .and_then(|count| windows_reference(&transfer.destination, count))
                && !album_parents.contains(&parent)
                && created.remove(&parent)
            {
                // Count the complete artist destination (including the optional
                // singles leaf), even if its later file transfer did not commit.
                folder_artists.insert(artist);
            }
        }
        // Stable sorting preserves Python dict insertion order for tied counts.
        artists.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        artists.truncate(5);
        Self {
            songs_sorted,
            artist_folders_created: folder_artists.len(),
            albums_processed,
            top_artists: artists,
        }
    }
}

// Match the selected Windows path spelling without opening, canonicalizing or
// case-folding a path. The validated destination contains single components;
// joining appends the same separator as Windows PathBuf::push. This pure helper
// also permits projection tests to run on non-Windows build hosts.
fn windows_reference(destination: &Destination, count: usize) -> Option<StorageRef> {
    let StorageRef::WindowsPath { utf16 } = &destination.root else {
        return None;
    };
    let mut utf16 = utf16.clone();
    for component in destination.components.iter().take(count) {
        if !utf16.last().is_some_and(|unit| *unit == 92 || *unit == 47) {
            utf16.push(92);
        }
        utf16.extend(component.encode_utf16());
    }
    Some(StorageRef::WindowsPath { utf16 })
}
