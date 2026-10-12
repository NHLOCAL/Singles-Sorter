use sorter_core::{
    discovery::{Capabilities, EntryKind, EntryObservation, StorageRef},
    job_plan::{Destination, OperationKind, OperationV2, TransferIntent},
};
use sorter_executor::legacy_summary::LegacySummary;

fn reference(path: &str) -> StorageRef {
    StorageRef::WindowsPath {
        utf16: path.encode_utf16().collect(),
    }
}
fn source() -> EntryObservation {
    EntryObservation {
        reference: reference(r"C:\source\song.mp3"),
        name: "song.mp3".into(),
        identity: None,
        parent_identity: None,
        kind: EntryKind::File,
        size: Some(1),
        modified: None,
        capabilities: Capabilities::default(),
    }
}
fn destination(parts: &[&str]) -> Destination {
    Destination {
        root: reference(r"C:\target"),
        components: parts.iter().map(|s| (*s).into()).collect(),
    }
}
fn single(id: usize, artist: &str, parts: &[&str]) -> OperationV2 {
    OperationV2 {
        id,
        kind: OperationKind::SingleTransfer {
            source: source(),
            artist: artist.into(),
            transfer: Box::new(TransferIntent {
                source: source(),
                destination: destination(parts),
                copy: true,
                predicted_success: true,
                blockers: vec![],
            }),
        },
        depends_on: vec![],
        stop_and_replan_on_effect_mismatch: true,
    }
}
fn album(id: usize, parts: &[&str]) -> OperationV2 {
    OperationV2 {
        id,
        kind: OperationKind::AlbumTransfer {
            source: source(),
            destination: destination(parts),
            children: vec![],
            remove_empty_source: false,
        },
        depends_on: vec![],
        stop_and_replan_on_effect_mismatch: true,
    }
}
#[test]
fn committed_singles_not_predictions_or_namespace_dispatches_supply_song_counts() {
    let operations = vec![
        single(0, "original", &["mapped", "a.mp3"]),
        single(1, "later", &["later", "b.mp3"]),
    ];
    let summary = LegacySummary::from_receipts(
        &operations,
        &[0],
        &[
            reference(r"C:\target\mapped"),
            reference(r"C:\target\later"),
        ],
        0,
        &[],
    );
    assert_eq!(summary.songs_sorted, 1);
    assert_eq!(summary.artist_folders_created, 2); // Earlier mkdir survives the later transfer failure.
    assert_eq!(summary.top_artists, [("original".into(), 1)]);
    let zero = LegacySummary::from_receipts(&operations, &[], &[], 0, &[]);
    assert_eq!(zero, LegacySummary::default());
}
#[test]
fn full_single_parent_and_deduplicated_artist_are_required_for_folder_credit() {
    let operations = vec![
        single(0, "A", &["A", "singles", "a.mp3"]),
        single(1, "A", &["A", "singles", "b.mp3"]),
    ];
    let parent_only =
        LegacySummary::from_receipts(&operations, &[0, 1], &[reference(r"C:\target\A")], 0, &[]);
    assert_eq!(parent_only.artist_folders_created, 0);
    let leaf = LegacySummary::from_receipts(
        &operations,
        &[0, 1],
        &[reference(r"C:\target\A\singles")],
        0,
        &[],
    );
    assert_eq!(leaf.artist_folders_created, 1);
    assert_eq!(leaf.songs_sorted, 2);
    assert_eq!(leaf.top_artists, [("A".into(), 2)]);
}
#[test]
fn album_mkdir_is_never_an_artist_folder_but_later_singles_leaf_is() {
    let operations = vec![
        album(0, &["A", "album"]),
        single(1, "A", &["A", "a.mp3"]),
        single(2, "B", &["A", "singles", "b.mp3"]),
    ];
    let summary = LegacySummary::from_receipts(
        &operations,
        &[0, 0, 1, 2],
        &[
            reference(r"C:\target\A"),
            reference(r"C:\target\A\album"),
            reference(r"C:\target\A\singles"),
        ],
        1,
        &[("pre-alias".into(), 3)],
    );
    assert_eq!(summary.songs_sorted, 2); // Two flattened album commits are not singles.
    assert_eq!(summary.artist_folders_created, 1);
    assert_eq!(summary.albums_processed, 1);
    assert_eq!(
        summary.top_artists,
        [("pre-alias".into(), 3), ("A".into(), 1), ("B".into(), 1)]
    );
}
#[test]
fn mixed_album_and_same_artist_single_reuse_the_album_created_parent() {
    let operations = vec![
        album(0, &["A", "album"]),
        single(1, "A", &["A", "single.mp3"]),
    ];
    let summary = LegacySummary::from_receipts(
        &operations,
        &[0, 0, 0, 0, 0, 1],
        &[reference(r"C:\target\A"), reference(r"C:\target\A\album")],
        1,
        &[("A".into(), 4)],
    );
    assert_eq!(summary.songs_sorted, 1);
    assert_eq!(summary.artist_folders_created, 0);
    assert_eq!(summary.albums_processed, 1);
    assert_eq!(summary.top_artists, [("A".into(), 5)]);
}

#[test]
fn top_five_ties_keep_actual_commit_order_after_original_album_artist() {
    let operations = (0..7)
        .map(|i| {
            single(
                i,
                &format!("artist-{i}"),
                &["existing", &format!("{i}.mp3")],
            )
        })
        .collect::<Vec<_>>();
    let summary = LegacySummary::from_receipts(
        &operations,
        &[4, 1, 6, 3, 0, 2, 5],
        &[],
        1,
        &[("album-first".into(), 1)],
    );
    assert_eq!(
        summary.top_artists,
        [
            ("album-first".into(), 1),
            ("artist-4".into(), 1),
            ("artist-1".into(), 1),
            ("artist-6".into(), 1),
            ("artist-3".into(), 1)
        ]
    );
    assert_eq!(summary.songs_sorted, 7);
}
#[test]
fn zero_receipts_serialize_the_exact_four_field_summary() {
    let summary = LegacySummary::from_receipts(&[], &[], &[], 0, &[]);
    assert_eq!(
        serde_json::to_value(summary).unwrap(),
        serde_json::json!({"songs_sorted":0,"artist_folders_created":0,"albums_processed":0,"top_artists":[]})
    );
    let zero_album =
        LegacySummary::from_receipts(&[], &[], &[], 1, &[("empty-completed-album".into(), 0)]);
    assert_eq!(
        zero_album.top_artists,
        [("empty-completed-album".into(), 0)]
    );
}

#[test]
fn one_created_directory_is_attributed_to_its_first_single_requirement_only() {
    let operations = vec![
        single(0, "first", &["shared", "a.mp3"]),
        single(1, "later", &["shared", "b.mp3"]),
    ];
    let summary =
        LegacySummary::from_receipts(&operations, &[1], &[reference(r"C:\target\shared")], 0, &[]);
    assert_eq!(summary.artist_folders_created, 1);
    assert_eq!(summary.top_artists, [("later".into(), 1)]);
}
