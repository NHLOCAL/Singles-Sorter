"""Actual two-volume album acceptance using the same native UI and owned media."""
import json
import os
from pathlib import Path
import re
import tempfile

from desktop_albums import ALBUM, ARTIST, fixture, files, metadata
from desktop_ai import tree_snapshot as snapshot
from desktop_copied_move import AUXILIARY, auxiliary, digest
from desktop_polish import set_developer_mode
from desktop_recovery import journal_snapshot

CHECKS = {'actual-cross-volume-root-album-move', 'actual-cross-volume-mixed-album-move',
          'cross-album-main-and-named-streams', 'cross-album-descendant-metadata',
          'cross-album-exact-source-cleanup', 'cross-album-observed-physical-and-legacy-counts',
          'cross-album-review-no-effects', 'cross-album-stale-stream-no-effects',
          'cross-album-stale-membership-no-effects'}
STAMP = 1_600_000_000_000_000_000


def stream_snapshot(root):
    """Known fixture stream bytes supplement whole-tree no-effect evidence."""
    return {path.relative_to(root).as_posix(): digest(auxiliary(path))
            for path in root.rglob('*') if path.is_file()}


def displayed_counts(details):
    match = re.fullmatch(r'(\d+) יעדים נכתבו · (\d+) קבצי מקור הועברו · (\d+) נכשלו · (\d+) לא בוצעו', details)
    if not match:
        raise ValueError('Missing actual terminal execution counters: ' + details)
    return dict(zip(('committed_destinations', 'moved_sources', 'failed_operations', 'blocked_operations'),
                    map(int, match.groups())))


def exercise_cross_albums(script, click, fill, wait, capture, root, output, audio_path,
                          second_volume_parent):
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
    root = Path(root); root.mkdir(); audio = Path(audio_path).read_bytes(); receipts = []
    previous = json.loads((Path(output) / 'selected-execution.json').read_text(encoding='utf-8'))
    journal_paths = {case['journal_path'] for case in previous['cases'] if case.get('journal_path')}
    assert previous['status'] == 'passed' and len(journal_paths) == 1, 'Missing prior owned journal-root receipt'
    journal_root = Path(journal_paths.pop())
    assert journal_root.is_dir()
    # This lifetime owns every target. A later restart test must run inside it or
    # receive ownership from its caller; cleaned targets are never restart proof.
    with tempfile.TemporaryDirectory(prefix='singles-ui-cross-albums-', dir=second_volume_parent) as other:
        other = Path(other)
        assert root.stat().st_dev != other.stat().st_dev, 'Two actual existing Windows volumes are required'
        for label in ('root-move', 'mixed-move', 'stale-stream', 'stale-membership'):
            root_album = label != 'mixed-move'
            source, unused_target, album = fixture(root / label, audio, root_album)
            assert not list(unused_target.iterdir())
            target = other / label; target.mkdir()
            for path in source.rglob('*'):
                if path.is_file():
                    auxiliary(path).write_bytes(AUXILIARY)
                    os.utime(path, ns=(STAMP, STAMP))
            before_files = files(source); before_album = files(album)
            album_directories = {path.relative_to(album).as_posix() for path in album.rglob('*') if path.is_dir()}
            before_streams = stream_snapshot(source); before_metadata = metadata(album)
            before_source, before_target = snapshot(source), snapshot(target)
            if script("return document.querySelector('#app').dataset.view==='results'"):
                click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
            fill('#source', str(source)); fill('#target', str(target))
            for key, value in dict(copy_mode=False, main_folder_only=False, singles_folder=False,
                                   abc_sort=False, exist_only=False).items():
                if script(f"return document.querySelector('#{key}').checked") != value:
                    click('#' + key)
            click('#advanced'); click('#duet-first')
            wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
            script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
            click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
            before_journal = journal_snapshot(journal_root)
            click('#sort')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
            assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), label
            assert snapshot(source) == before_source and snapshot(target) == before_target
            assert journal_snapshot(journal_root) == before_journal
            assert stream_snapshot(source) == before_streams
            click('#review-apply')
            description = script("return document.querySelector('#apply-description').textContent")
            assert 'סגור תוכנות או סנכרון' in description
            capture('cross-album-review-' + label); click('#apply-back')
            assert snapshot(source) == before_source and snapshot(target) == before_target
            assert journal_snapshot(journal_root) == before_journal
            assert stream_snapshot(source) == before_streams
            if label == 'stale-stream':
                file = album / '1-שיר.mp3'
                auxiliary(file).write_bytes(b'changed stream!')
                os.utime(file, ns=(STAMP, STAMP))
                before_source, before_streams = snapshot(source), stream_snapshot(source)
            elif label == 'stale-membership':
                late = album / 'late-owned-sidecar.txt'; late.write_bytes(b'owned after preview')
                auxiliary(late).write_bytes(AUXILIARY)
                before_source, before_streams = snapshot(source), stream_snapshot(source)
            click('#review-apply'); click('#confirm-apply')
            if label.startswith('stale-'):
                wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
                assert snapshot(source) == before_source and snapshot(target) == before_target
                assert journal_snapshot(journal_root) == before_journal
                assert stream_snapshot(source) == before_streams
                summary = script("return document.querySelector('#execution-summary').textContent")
                assert 'קבצים הועברו' not in summary and 'יעדים נכתבו' not in summary
                capture('cross-album-refused-' + label)
                receipts.append(dict(case=label, cross_volume=True, source_and_target_unchanged=True,
                                     auxiliary_unchanged=True, journal_unchanged=True, native_result_counters=None,
                                     observed_file_effects=dict(new_destinations=0, retired_sources=0),
                                     displayed_summary=summary, ipc_mocked=False))
                continue
            wait(lambda: script("return document.querySelector('#execution-title').textContent==='הביצוע הושלם'"), timeout=120)
            wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
            destination = target / ARTIST / ALBUM
            assert metadata(destination) == before_metadata, 'Copied Move changed descendant metadata'
            assert files(destination) == before_album
            for relative in before_album:
                assert auxiliary(destination / relative).read_bytes() == AUXILIARY
            assert not album.exists(), 'Exact emptied album source was not removed'
            if not root_album:
                single = target / ARTIST / f'{ARTIST} - שיר בודד.mp3'
                assert single.read_bytes() == audio and auxiliary(single).read_bytes() == AUXILIARY
                assert single.stat().st_mtime_ns == STAMP
                assert source.is_dir() and not list(source.iterdir()), 'Mixed source container was not retained empty'
            expected_files = {f'{ARTIST}/{ALBUM}/{relative}': sha for relative, sha in before_album.items()}
            if not root_album:
                expected_files[f'{ARTIST}/{ARTIST} - שיר בודד.mp3'] = before_files[f'{ARTIST} - שיר בודד.mp3']
            expected_directories = {ARTIST, f'{ARTIST}/{ALBUM}'} | {f'{ARTIST}/{ALBUM}/{relative}' for relative in album_directories}
            assert files(target) == expected_files, 'Target contains missing, unexpected or changed files'
            assert {path.relative_to(target).as_posix() for path in target.rglob('*') if path.is_dir()} == expected_directories, 'Target directory set differs'
            assert script("return document.querySelector('#execution-recovery').hidden"), 'Final verification remains unresolved'
            summary = script("return document.querySelector('#execution-summary').textContent")
            assert '1 אלבומים הושלמו' in summary
            assert f'{0 if root_album else 1} שירים מוינו' in summary
            assert f'{ARTIST} ({4 if root_album else 5})' in summary
            assert script("return document.querySelectorAll('#rows tr').length") == (1 if root_album else 2)
            assert 'הקובץ לא נמצא' not in script("return document.querySelector('#rows').textContent")
            assert '\u2014' not in script("return document.querySelector('#rows').textContent")
            capture('cross-album-complete-' + label)
            set_developer_mode(script, click, wait, True)
            details = script("return document.querySelector('#execution-summary').textContent")
            counters = displayed_counts(details)
            assert counters == dict(committed_destinations=len(before_files), moved_sources=len(before_files),
                                    failed_operations=0, blocked_operations=0)
            set_developer_mode(script, click, wait, False)
            receipts.append(dict(case=label, cross_volume=True, source=str(source), target=str(target),
                                 journal_path=script("return document.querySelector('#journal-path').value"),
                                 original_sha256=before_files, destination_sha256=files(target),
                                 original_auxiliary_sha256=before_streams, destination_auxiliary_sha256=stream_snapshot(target),
                                 expected_counts=dict(committed_destinations=len(before_files), moved_sources=len(before_files)),
                                 observed_displayed_counts=counters, displayed_summary=summary, displayed_details=details,
                                 original_album_absent=True, descendant_metadata_preserved=True, ipc_mocked=False))
    (Path(output) / 'cross-album-execution.json').write_text(json.dumps(
        dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
