"""End-to-end reference tooling checks, independent of Rust or its outputs."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = Path(__file__).with_name('export_full_contract.py')
INPUTS = ROOT / 'native/tests/fixtures/full_contract/inputs.json'


def export(tmp_path):
    output = tmp_path / 'export'
    result = subprocess.run([sys.executable, str(SCRIPT), '--baseline', os.environ['SINGLES_RELEASE_BASELINE'],
                             '--inputs', str(INPUTS), '--output', str(output)], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return output


def test_exports_release_observations_and_separates_unsafe_cases(tmp_path):
    output = export(tmp_path)
    cases = {row['id']: row for row in map(json.loads, (output / 'cases.jsonl').read_text().splitlines())}
    assert cases['album-track-113']['result'] == [True, True, 'album', 'אברהם פריד']
    assert cases['album-filename-arabic-indic']['result'][0] is True
    assert cases['album-filename-fullwidth']['result'][0] is True
    assert cases['album-no-readable-protected']['result'] == [True, False, None, None]
    tie = cases['album-first-tie-wins']
    encounter = next(event['children'] for event in tie['ordered_trace'] if event['kind'] == 'glob')
    fixture = next(case for case in json.loads(INPUTS.read_text())['cases'] if case['id'] == tie['id'])
    names = {'source/' + file['path']: file['metadata']['album'] for file in fixture['files']}
    assert tie['result'][2] == names[encounter[0]['path']]
    assert cases['single-open-error-survives']['result'] == ['אברהם פריד']
    assert cases['single-title-value-error-after-match']['error']['type'] == 'ValueError'
    assert cases['single-lazy-personal-error']['error']['type'] == 'ValueError'
    assert cases['scan-album-nested-copy']['result']['songs_sorted'] == 1
    assert cases['scan-album-nested-move']['result']['songs_sorted'] == 0
    unsafe = list(map(json.loads, (output / 'historical-unsafe.jsonl').read_text().splitlines()))
    assert len(unsafe) == 2
    assert all(row['historical_only'] and row['future_acceptance'] is False for row in unsafe)
    assert not any(row['op'] == 'unsafe' for row in cases.values())
    manifest = json.loads((output / 'manifest.json').read_text())
    assert manifest['runtime']['python'] == '3.12.14'
    assert manifest['runtime']['unicode'] == '15.0.0'
    assert manifest['baseline']['revision'] == '78d001aee5ce49628bdffebd34540fa83ba32c12'
    assert manifest['actual_metadata_decoding'] is False
    assert manifest['actual_model_inference'] is False


def test_mutated_or_unpinned_release_is_refused(tmp_path):
    result = subprocess.run([sys.executable, str(SCRIPT), '--baseline', str(ROOT),
                             '--inputs', str(INPUTS), '--output', str(tmp_path / 'bad')], capture_output=True, text=True)
    assert result.returncode != 0
    assert 'original pinned' in result.stderr
    assert not (tmp_path / 'bad').exists()


def test_owned_tree_guard_rejects_traversal_and_external_symlinks(tmp_path):
    from export_full_contract import guard_owned
    import pytest
    root = tmp_path / 'owned'
    root.mkdir()
    guard_owned(root, root / 'source' / 'new.mp3')
    with pytest.raises(ValueError):
        guard_owned(root, root / '..' / 'outside.mp3')
    (root / 'escape').symlink_to(tmp_path, target_is_directory=True)
    with pytest.raises(ValueError):
        guard_owned(root, root / 'escape' / 'outside.mp3')


def test_fresh_release_exports_repeat_and_match_committed_artifacts(tmp_path):
    first = export(tmp_path / 'first')
    second = export(tmp_path / 'second')
    frozen = INPUTS.parent
    for name in ['cases.jsonl', 'historical-unsafe.jsonl', 'csv-dialect.json']:
        assert (first / name).read_bytes() == (second / name).read_bytes()
        assert (first / name).read_bytes() == (frozen / name).read_bytes()
    assert (first / 'manifest.json').read_bytes() == (second / 'manifest.json').read_bytes()


def _reference_case(id, alias):
    return {'id': id, 'op': 'sort', 'files': [{'path': 'אברהם פריד.mp3', 'metadata': {}}],
            'aliases': [['אברהם פריד', alias]], 'settings': {'singles_folder': False}}


def _outside_snapshot(root):
    # Owned reviewer/test tree, separate from run_case's temporary root.
    return sorted((path.relative_to(root).as_posix(), 'directory' if path.is_dir() else path.read_bytes())
                  for path in root.rglob('*'))


def test_run_case_blocks_absolute_alias_before_any_outside_mutation(tmp_path):
    from export_full_contract import release_module, run_case
    baseline = Path(os.environ['SINGLES_RELEASE_BASELINE'])
    (tmp_path / 'marker').write_bytes(b'unchanged outside case')
    before = _outside_snapshot(tmp_path)
    with release_module(baseline) as released:
        result = run_case(released, _reference_case('absolute-escape', str(tmp_path / 'escape')), [])
    assert _outside_snapshot(tmp_path) == before
    assert any(row['path'] == 'source/אברהם פריד.mp3' for row in result['after']['source'])
    assert any(event['kind'] == 'mutation_blocked' for event in result['ordered_trace'])


def test_run_case_blocks_traversal_alias_before_any_outside_mutation(tmp_path, monkeypatch):
    import export_full_contract as harness
    original_temporary_directory = harness.tempfile.TemporaryDirectory
    (tmp_path / 'marker').write_bytes(b'unchanged outside case')
    before = _outside_snapshot(tmp_path)
    with harness.release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        # The reference case root is a child of this reviewer-owned temp tree.
        monkeypatch.setattr(harness.tempfile, 'TemporaryDirectory',
                            lambda *args, **kwargs: original_temporary_directory(*args, dir=tmp_path, **kwargs))
        result = harness.run_case(released, _reference_case('traversal-escape', '../../escape'), [])
    assert _outside_snapshot(tmp_path) == before
    assert any(row['path'] == 'source/אברהם פריד.mp3' for row in result['after']['source'])
    assert any(event['kind'] == 'mutation_blocked' for event in result['ordered_trace'])


def test_owned_rename_failure_records_real_shutil_fallback(tmp_path, monkeypatch):
    from export_full_contract import release_module, run_case
    def fail_rename(*args, **kwargs):
        raise OSError('injected owned rename failure')
    with release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        monkeypatch.setattr(Path, 'rename', fail_rename)
        result = run_case(released, _reference_case('owned-fallback', 'אברהם פריד'), [])
    assert result['result']['songs_sorted'] == 1
    assert not any(row['path'] == 'source/אברהם פריד.mp3' for row in result['after']['source'])
    assert any(row['path'] == 'target/אברהם פריד/אברהם פריד.mp3' for row in result['after']['target'])
    events = result['ordered_trace']
    assert any(event['kind'] == 'rename' and event['outcome'] == 'error' for event in events)
    assert any(event['kind'] == 'shutil_move' and event['outcome'] == 'success' for event in events)
    assert any(event['kind'] == 'os_rename' and event['outcome'] == 'success' for event in events)


def test_recursive_fix_records_complete_collection_before_processing():
    from export_full_contract import release_module, run_case
    case = {'id': 'recursive-fix-order', 'op': 'fix', 'files': [
        {'path': 'אברהם_פריד.mp3', 'metadata': {}}, {'path': 'אברהם פריד.mp3', 'metadata': {}},
        {'path': 'nested/מוטי_שטיינמץ.mp3', 'metadata': {}}, {'path': 'nested/cover.jpg'},
        {'path': 'empty', 'kind': 'directory'}, {'path': 'nested/deeper/note.txt'},
        {'path': 'nested/deeper/track.MP3', 'metadata': {}}]}
    with release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        result = run_case(released, case, [['אברהם פריד', 'אברהם פריד']])
    trace = result['ordered_trace']
    collection = next(event for event in trace if event['kind'] == 'rglob')
    assert collection['completed'] is True
    expected = {row['path'] for row in result['before']['source'] if row['path'] != 'source'}
    assert {row['path'] for row in collection['children']} == expected
    end = next(index for index, event in enumerate(trace) if event['kind'] == 'collection_complete' and event['method'] == 'rglob')
    start_processing = next(index for index, event in enumerate(trace) if event['kind'] in ['rename', 'metadata_open'])
    assert end < start_processing
    processed = [event['source_observation']['path'] for event in trace if event['kind'] == 'metadata_open']
    assert len(processed) == 4
    renames = {event['source_observation']['path']: event['destination'] for event in trace if event['kind'] == 'rename'}
    audio_order = [row['path'] for row in collection['children'] if Path(row['path']).suffix.lower() == '.mp3']
    assert processed == [renames.get(path, path) for path in audio_order]
    assert any(event['kind'] == 'rename' and event['destination'].endswith('_1.mp3') for event in trace)


def test_owned_cross_device_fallback_records_copy_and_source_removal(monkeypatch):
    import errno
    from export_full_contract import release_module, run_case
    def cross_device_rename(*args, **kwargs):
        raise OSError(errno.EXDEV, 'injected cross-device rename')
    with release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        monkeypatch.setattr(os, 'rename', cross_device_rename)
        result = run_case(released, _reference_case('owned-cross-device', 'אברהם פריד'), [])
    assert result['result']['songs_sorted'] == 1
    assert not any(row['path'] == 'source/אברהם פריד.mp3' for row in result['after']['source'])
    assert any(row['path'] == 'target/אברהם פריד/אברהם פריד.mp3' for row in result['after']['target'])
    for kind in ['shutil_move', 'open_write', 'os_unlink']:
        assert any(event['kind'] == kind and event['outcome'] == 'success' for event in result['ordered_trace'])


def test_owned_directory_fallback_retains_fd_relative_confinement(monkeypatch):
    import errno
    from export_full_contract import release_module, run_case
    def cross_device_rename(*args, **kwargs):
        raise OSError(errno.EXDEV, 'injected cross-device rename')
    case = {'id': 'owned-directory-fallback', 'op': 'sort', 'settings': {'singles_folder': False}, 'files': [
        {'path': f'album/track_{letter}.mp3', 'metadata': {'artist': 'אברהם פריד', 'album': 'אלבום'}}
        for letter in ['a', 'b', 'c']] + [{'path': 'album/extra/keep.txt'}]}
    with release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        monkeypatch.setattr(os, 'rename', cross_device_rename)
        result = run_case(released, case, [['אברהם פריד', 'אברהם פריד']])
    assert result['result']['albums_processed'] == 1
    assert any(row['path'] == 'target/אברהם פריד/אלבום/extra/keep.txt' for row in result['after']['target'])
    assert not any(row['path'] == 'source/album' for row in result['after']['source'])
    assert any(event['kind'] == 'os_unlink' and event['source_observation']['path'] == 'source/album/extra/keep.txt'
               and event['outcome'] == 'success' for event in result['ordered_trace'])
    assert not any(event['kind'] == 'mutation_blocked' for event in result['ordered_trace'])


def test_lower_level_rename_escape_cannot_bypass_outer_owned_operation(tmp_path, monkeypatch):
    from export_full_contract import release_module, run_case
    destination = tmp_path / 'external.mp3'
    destination.write_bytes(b'outside existing object remains unchanged')
    before = _outside_snapshot(tmp_path)
    def insecure_rename(path, ignored_requested_destination):
        # Simulates a lower-level callback that bypasses Path's requested target.
        return os.rename(path, destination)
    with release_module(Path(os.environ['SINGLES_RELEASE_BASELINE'])) as released:
        monkeypatch.setattr(Path, 'rename', insecure_rename)
        result = run_case(released, _reference_case('low-level-escape', 'אברהם פריד'), [])
    assert _outside_snapshot(tmp_path) == before
    assert result['error']['type'] == 'MutationConfinementError'
    assert any(row['path'] == 'source/אברהם פריד.mp3' for row in result['after']['source'])
    assert any(event['kind'] == 'mutation_blocked' and event['entrypoint'] == 'os_rename'
               for event in result['ordered_trace'])
    assert any(event['kind'] == 'rename' and event['outcome'] == 'blocked' for event in result['ordered_trace'])
