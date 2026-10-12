"""Two-volume album trial shared by the staged and consumer CLI gate."""
import json
import os
from pathlib import Path

from desktop_albums import ARTIST, ALBUM, fixture, files, metadata
from desktop_ai import tree_snapshot as snapshot
from desktop_copied_move import AUXILIARY, auxiliary
from desktop_cross_albums import STAMP, stream_snapshot

CHECKS = {'positional-cross-volume-root-album-move', 'positional-cross-volume-mixed-album-move',
          'positional-cross-album-streams-metadata', 'positional-cross-album-observed-credits',
          'positional-cross-album-dry-run-no-effects', 'positional-cross-album-fresh-reconciliation'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def validate_effect_counts(actual, source_count, mixed):
    require(actual['status'] == 'complete' and actual.get('recovery_error') is None,
            'Cross-volume album did not complete and reconcile')
    require(actual['source_items'] == source_count and actual['committed_destinations'] == source_count
            and actual['moved_sources'] == source_count, 'Cross-volume album physical counts differ')
    require(actual['albums_completed'] == 1 and actual['artist_files'] == [[ARTIST, 4]],
            'Cross-volume original album credits differ')
    legacy = actual['legacy_summary']
    # Frozen v14 handles the album before deferred singles. With -n, this
    # same-artist single reuses the parent created by the album, so its loop
    # never performs the mkdir that increments artist_folders_created.
    expected = dict(songs_sorted=int(mixed), albums_processed=1,
                    artist_folders_created=0, top_artists=[[ARTIST, 5 if mixed else 4]])
    require(legacy == expected,
            f'Cross-volume original single and album counters differ: expected={expected!r}, actual={legacy!r}')
    effects = actual['effect_receipts']
    require(sum(effect['kind'] == 'source_retired' for effect in effects) == source_count,
            'Cross-volume album lacks one retirement acknowledgement per file')
    require(sum(effect['kind'] == 'file_published' for effect in effects) == source_count,
            'Cross-volume album lacks one publication acknowledgement per file')
    require(not any(effect['kind'] in ('file_renamed', 'directory_renamed') for effect in effects),
            'Cross-volume album fabricated an atomic rename effect')


def identity_key(identity):
    return json.dumps(identity, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def require_effect_bindings(actual, pairs):
    """Every fixture source/destination identity needs its own ordered receipts."""
    publications = {}; retirements = {}
    for index, receipt in enumerate(actual['effect_receipts']):
        require(receipt['kind'] in ('file_published', 'source_retired'), 'Unexpected copied-album effect kind')
        target = publications if receipt['kind'] == 'file_published' else retirements
        key = identity_key(receipt['identity'])
        require(key not in target, 'Duplicate acknowledged identity')
        target[key] = (index, receipt['operation'])
    expected_sources = {identity_key(source) for source, _ in pairs}
    expected_destinations = {identity_key(destination) for _, destination in pairs}
    require(len(expected_sources) == len(pairs) and len(expected_destinations) == len(pairs),
            'Fixture does not provide unique file identities')
    require(set(retirements) == expected_sources and set(publications) == expected_destinations,
            'Acknowledged identities do not exactly cover observed files')
    completed = {}
    for item in actual['transfers']['items']:
        require(item['status'] == 'complete', 'Incomplete copied-album item')
        identities = {identity_key(observation['identity']) for observation in item['observations']}
        matched = identities & expected_destinations
        require(len(matched) == 1, 'Completed item has missing or ambiguous destination evidence')
        key = matched.pop()
        require(key not in completed, 'Destination appears in more than one completed item')
        completed[key] = item['operation']
    require(set(completed) == expected_destinations, 'Completed item evidence omits a destination')
    for source, destination in pairs:
        published = publications[identity_key(destination)]; retired = retirements[identity_key(source)]
        require(published[0] < retired[0] and published[1] == retired[1] == completed[identity_key(destination)],
                'Source retired before its matching acknowledged publication')


def windows_reference(path):
    raw = str(path).encode('utf-16-le', errors='surrogatepass')
    return {'WindowsPath': {'utf16': [int.from_bytes(raw[i:i+2], 'little') for i in range(0, len(raw), 2)]}}


def require_cleanup_bindings(report, expected):
    """Bind the journal's source-cleanup roles to independently observed roots."""
    recorded = report['removed_directory_references']
    expected_keys = {identity_key(pair) for pair in expected}
    recorded_keys = {identity_key(pair) for pair in recorded}
    require(len(expected_keys) == len(expected) and len(recorded_keys) == len(recorded),
            'Duplicate source-directory cleanup evidence')
    require(recorded_keys == expected_keys,
            'Acknowledged source cleanup does not exactly cover the observed original album subtree')


def windows_identity(path):
    from desktop_ai import windows_attribute_observation
    observation = windows_attribute_observation(path)
    return {'Windows': {'volume': observation['win32_volume_serial'],
                        'file_id': list(observation['win32_file_id'].to_bytes(16, 'little'))}}


def exercise(invoke, result_record, require_album_recovery, scratch, other, audio, receipts):
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
    require(scratch.stat().st_dev != other.stat().st_dev, 'Cross-album trial requires actual distinct volumes')
    for root_album, label in [(True, 'cross-root-album-move'), (False, 'cross-mixed-album-move')]:
        case = scratch / label; (case / 'journal').mkdir(parents=True); (case / 'app').mkdir()
        source, unused_target, album = fixture(case, audio, root_album)
        require(not list(unused_target.iterdir()), 'Unexpected authored target content')
        target = other / label; target.mkdir()
        for path in source.rglob('*'):
            if path.is_file():
                auxiliary(path).write_bytes(AUXILIARY)
                os.utime(path, ns=(STAMP, STAMP))
        original = files(source); album_before = files(album); metadata_before = metadata(album)
        source_identities = {path.relative_to(source).as_posix(): windows_identity(path)
                             for path in source.rglob('*') if path.is_file()}
        album_directories = {path.relative_to(album).as_posix() for path in album.rglob('*') if path.is_dir()}
        expected_cleanup = [[windows_reference(path), windows_identity(path)]
                            for path in [album, *sorted(p for p in album.rglob('*') if p.is_dir())]]
        args = [str(source), str(target), '-n', '--no-ai', '--journal-directory', 'journal']
        before, target_before, streams_before = snapshot(case), snapshot(target), stream_snapshot(source)
        invoke(case, [*args, '--dry-run'])
        require(snapshot(case) == before and snapshot(target) == target_before
                and stream_snapshot(source) == streams_before, 'Cross-album dry-run changed owned data')
        completed = invoke(case, args); actual = result_record(completed.stderr)
        validate_effect_counts(actual, len(original), not root_album)
        require_album_recovery(actual['recovery'])
        require_cleanup_bindings(actual['recovery'], expected_cleanup)
        destination = target / ARTIST / ALBUM
        require(metadata(destination) == metadata_before, 'Cross-album descendant metadata differs')
        require(files(destination) == album_before, 'Cross-album content or layout differs')
        require(all(auxiliary(destination / relative).read_bytes() == AUXILIARY for relative in album_before),
                'Cross-album named stream bytes differ')
        require(not album.exists(), 'Exact empty source album was not removed')
        if not root_album:
            single = target / ARTIST / f'{ARTIST} - שיר בודד.mp3'
            require(single.read_bytes() == audio and auxiliary(single).read_bytes() == AUXILIARY
                    and single.stat().st_mtime_ns == STAMP, 'Cross-volume mixed single differs')
            require(source.is_dir() and not list(source.iterdir()), 'Mixed source container was not retained empty')
        expected_files = {f'{ARTIST}/{ALBUM}/{relative}': sha for relative, sha in album_before.items()}
        if not root_album:
            expected_files[f'{ARTIST}/{ARTIST} - שיר בודד.mp3'] = original[f'{ARTIST} - שיר בודד.mp3']
        expected_directories = {ARTIST, f'{ARTIST}/{ALBUM}'} | {f'{ARTIST}/{ALBUM}/{relative}' for relative in album_directories}
        require(files(target) == expected_files, 'Cross-album target has unexpected or changed files')
        require({path.relative_to(target).as_posix() for path in target.rglob('*') if path.is_dir()} == expected_directories,
                'Cross-album target directory set differs')
        pairs = []
        for relative in original:
            if root_album:
                destination_file = destination / relative
            elif relative.startswith('input-album/'):
                destination_file = destination / relative.removeprefix('input-album/')
            else:
                destination_file = target / ARTIST / relative
            pairs.append((source_identities[relative], windows_identity(destination_file)))
        require_effect_bindings(actual, pairs)
        receipts[-1].update(case=label, source_sha256=original, destination_sha256=files(target),
                            source_auxiliary_sha256=streams_before, destination_auxiliary_sha256=stream_snapshot(target),
                            observed_counts={key: actual[key] for key in ('source_items', 'committed_destinations', 'moved_sources', 'albums_completed')},
                            observed_legacy_summary=actual['legacy_summary'], cross_volume=True,
                            observed_effect_receipts=actual['effect_receipts'],
                            expected_source_cleanup=expected_cleanup,
                            observed_source_cleanup=actual['recovery']['removed_directory_references'],
                            descendant_metadata_verified=True)
