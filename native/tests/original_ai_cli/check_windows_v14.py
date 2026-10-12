#!/usr/bin/env python3
"""Diagnostic paired actual-Windows v14 regression; OPEN is never a parity pass.

Only authored media in freshly owned trees. No original library or supplied path
is mutated. Every case report is retained even when another case mismatches.
"""
from __future__ import annotations
import argparse
import hashlib
from importlib import metadata
import itertools
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import traceback

from check_cli import ASSETS, BASELINE, BUNDLE_ID, NATIVE, authored_wav, digest, original_oracle, require
from windows_v14_trace import execute_original, relative

SOURCE_SHA256 = 'ec4a98ce29c431b5d46b59f11a531eaf8fb4f3c8c16248372771de630086fffa'
SETTING_NAMES = ('copy_mode', 'main_folder_only', 'abc_sort', 'singles_folder', 'exist_only')
SUMMARY_FIELDS = ('songs_sorted', 'artist_folders_created', 'albums_processed', 'top_artists')
EXISTING_ARTIST = 'אברהם פריד'
MISSING_ARTIST = 'מוטי שטיינמץ'
DUET = 'ישראל כהן וחיים לוי - שיר חדש.wav'
FIXED_MTIME_NS = 1_700_000_000_000_000_000
NORMALIZATION = {
    'root': 'Strip only the exact owned-root prefix, including its known Windows local-drive \\\\?\\ spelling from handle observations; raw UTF-16 references retained. No child spelling changes.',
    'names': 'No filename, case, Unicode, separator, artist, destination or order normalization.',
    'timestamps': 'Authored file mtimes use a fixed epoch and are compared exactly. Directory mtimes are recorded but excluded because creation/transfer wall times differ. Access times are not sampled.',
    'runtime': 'Wall-clock duration, PID, journal/job IDs, OS object IDs and plan hashes are retained where emitted but are not compared as business semantics.',
    'ordering': 'Directory encounter lists, decisions, operation traces and top artists retain engine order. No sorting to force equality.',
    'serialization': 'Original tuples use JSON arrays, exactly as persisted by the report; no values or sequence positions change.',
    'counters': 'Only final native legacy_summary is actual v14 summary evidence. Predictions and independently observed physical counts are separate.',
}


def cases():
    result = []
    for bits in itertools.product((False, True), repeat=len(SETTING_NAMES)):
        settings = dict(zip(SETTING_NAMES, bits))
        result.append({'id': 'settings-' + ''.join(str(int(bit)) for bit in bits),
                       'group': 'settings', 'settings': {**settings, 'duet_mode': False}, 'ai': False})
    for copying, duet in itertools.product((False, True), repeat=2):
        result.append({'id': f'duet-{"copy" if copying else "move"}-{"all" if duet else "first"}',
                       'group': 'duet', 'settings': dict(copy_mode=copying, main_folder_only=False,
                       abc_sort=False, singles_folder=False, exist_only=False, duet_mode=duet), 'ai': True})
    return result


def snapshot(root):
    """Preserve OS encounter order; all raw relative names and file bytes are exact."""
    rows = []
    def visit(path):
        info = path.lstat()
        require(not path.is_symlink() and not getattr(info, 'st_file_attributes', 0) & 0x400,
                'Fixture snapshot encountered a link/reparse point')
        row = {'path': relative(root, path), 'absolute_path': str(path), 'name': path.name,
               'kind': 'directory' if path.is_dir() else 'file', 'mode': info.st_mode,
               'mtime_ns': info.st_mtime_ns}
        if path.is_file():
            row.update(bytes=info.st_size, sha256=digest(path))
        rows.append(row)
        if path.is_dir():
            with os.scandir(path) as entries:
                for entry in entries:
                    visit(Path(entry.path))
    for name in ('source', 'target'):
        visit(root/name)
    return rows


def comparable_snapshot(rows):
    return [{key: value for key, value in row.items()
             if key not in ('absolute_path',) and not (key == 'mtime_ns' and row['kind'] == 'directory')}
            for row in rows]


def artist_directory(root, artist, settings, singles=True):
    path = root/'target'
    if settings['abc_sort']:
        path /= artist[0]
    path /= artist
    if singles and settings['singles_folder']:
        path /= 'סינגלים'
    return path


def author_case(root, case):
    for name in ('source', 'target', 'journal', 'app'):
        (root/name).mkdir(parents=True)
    shutil.copyfile(NATIVE/'tests/fixtures/full_contract/released-singer-list.csv', root/'app/singer-list.csv')
    (root/'target/sentinel.bin').write_bytes(b'owned unchanged target\x00\xff')
    specs = []
    if case['group'] == 'duet':
        specs.append((DUET, '', '', '', ''))
    else:
        # The deliberate creation order is retained, not alphabetically sorted.
        specs.extend([
            ('root song.wav', EXISTING_ARTIST, 'root title', 'singles', ''),
            ('direct/child song.wav', MISSING_ARTIST, 'child title', 'singles', ''),
            ('direct/deeper/deep song.wav', EXISTING_ARTIST, 'deep title', 'singles', ''),
            ('unknown.wav', '', '', '', ''),
        ])
        for number in (2, 1, 3):
            specs.append((f'processable/track {number}.wav', MISSING_ARTIST, f'album {number}', 'אלבום בדיקה', str(number)))
        for number, artist in enumerate(('אבי אילסון', 'אבי בבאי', 'אבי ביר'), 1):
            specs.append((f'protected/track {number}.wav', artist, f'protected {number}', 'אלבום מוגן', str(number)))
        artist_directory(root, EXISTING_ARTIST, case['settings']).mkdir(parents=True)
        (root/'source/processable/notes.txt').parent.mkdir(parents=True, exist_ok=True)
        (root/'source/processable/notes.txt').write_bytes(b'authored album sidecar\n')
        (root/'source/unsupported.txt').write_bytes(b'not audio\n')
    from mutagen.id3 import TALB, TRCK
    from mutagen.wave import WAVE
    import music_tag
    for filename, artist, title, album, track in specs:
        path = root/'source'/filename
        path.parent.mkdir(parents=True, exist_ok=True)
        authored_wav(path, artist, title)
        if album or track:
            audio = WAVE(path)
            if audio.tags is None:
                audio.add_tags()
            if album:
                audio.tags.add(TALB(encoding=3, text=[album]))
            if track:
                audio.tags.add(TRCK(encoding=3, text=[track]))
            audio.save()
            tags = music_tag.load_file(str(path))
            require(tags['album'].value == album, 'Authored album did not round-trip')
            if track:
                require(str(tags['tracknumber'].value) == track, 'Authored track did not round-trip')
    # Deterministic file mtimes: copied source bytes and exact metadata can be compared.
    for path in root.rglob('*'):
        if path.is_file():
            os.utime(path, ns=(FIXED_MTIME_NS, FIXED_MTIME_NS))


def subprocess_record(binary, root, arguments, *, payload=None):
    env = dict(os.environ, PATH=str(Path(os.environ['SystemRoot'])/'System32'))
    options = {'input': json.dumps(payload, ensure_ascii=False).encode('utf-8')} if payload is not None else {'stdin': subprocess.DEVNULL}
    result = subprocess.run([str(binary), *arguments], cwd=root, env=env,
                            capture_output=True, timeout=180, **options)
    return {'arguments': arguments, 'exit': result.returncode,
            'stdout': result.stdout.decode('utf-8'), 'stderr': result.stderr.decode('utf-8'),
            'closed_stdin': payload is None, 'developer_path_removed': True}


def native_request(root, bundle, case):
    return {'schema_version': 2, 'mode': 'Sort', 'source': str(root/'source'),
            'target': str(root/'target'), 'settings': case['settings'], 'personal_aliases': None,
            'ai_profile': {'Configured': {'bundle_id': BUNDLE_ID, 'model_directory': str(bundle)}} if case['ai'] else 'Disabled'}


def native_args(bundle, case):
    args = ['source', 'target', '--journal-directory', 'journal', '--log_level', 'DEBUG']
    for name, flag in [('copy_mode', '-c'), ('main_folder_only', '-m'), ('abc_sort', '-a'),
                       ('exist_only', '-e'), ('duet_mode', '-d')]:
        if case['settings'][name]:
            args.append(flag)
    if not case['settings']['singles_folder']:
        args.append('-n')
    args.extend(['--model-directory', str(bundle)] if case['ai'] else ['--no-ai'])
    return args


def result_record(stderr):
    rows = [json.loads(line.split(' - MusicSorter - result: ', 1)[1])
            for line in stderr.splitlines() if ' - MusicSorter - result: ' in line]
    require(len(rows) <= 1, 'Native emitted duplicate result records')
    return rows[0] if rows else None


def read_journals(root):
    result = []
    for path in (root/'journal').rglob('*'):
        if path.is_file():
            data = path.read_bytes()
            row = {'path': relative(root, path), 'bytes': len(data), 'sha256': digest(path),
                   'ends_with_newline': data.endswith(b'\n')}
            try:
                row['raw_utf8'] = data.decode('utf-8')
                row['records'] = [json.loads(line) for line in row['raw_utf8'].splitlines()]
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                row['parse_error'] = str(error)
            result.append(row)
    return result


def reference_path(root, reference):
    values = reference['WindowsPath']['utf16']
    path = b''.join(value.to_bytes(2, 'little') for value in values).decode('utf-16-le', errors='strict')
    owned_root = str(root)
    if len(owned_root) >= 3 and owned_root[0].isalpha() and owned_root[1:3] == ':\\':
        # GetFinalPathNameByHandleW(VOLUME_NAME_DOS) adds this exact local-drive
        # root spelling. Do not canonicalize any case, name, child or separator.
        for spelling in (owned_root, '\\\\?\\' + owned_root):
            if path == spelling:
                return '.'
            if path.startswith(spelling + '\\'):
                return path[len(spelling) + 1:]
        raise ValueError(f'Observed reference does not have an exact owned local-drive root: {path}')
    return relative(root, path)


def physical_commit_streams(root, journals):
    """Physical journal order, without borrowing planner success or global order."""
    streams = []
    for journal in journals:
        records = journal.get('records', [])
        if not records:
            continue
        header = records[0].get('body', {})
        if header.get('schema') != 2 or 'binding' not in header:
            continue
        require(journal.get('ends_with_newline', False), 'Physical journal has an incomplete final frame')
        bound = {op['id']: op for op in header['binding']['operations']}
        operations = []
        for record in records[1:]:
            body = record['body']
            event = body['event']
            if event['phase'] != 'Committed':
                continue
            operation = bound[event['operation']]
            source = reference_path(root, operation['source']['observation']['reference'])
            destination_root = reference_path(root, operation['destination']['root'])
            destination = str(Path(destination_root).joinpath(*operation['destination']['components']))
            operations.append({'kind': 'copy' if operation['copy_effect'] else 'move',
                               'source': source, 'destination': destination,
                               'sha256': event['sha256'], 'sequence': body['sequence'],
                               'operation': event['operation']})
        streams.append({'journal': journal['path'], 'ordered_commits': operations})
    return streams



def map_committed_destinations(root, before, after, streams, result, original_operations=()):
    """Link actual journal/whole-rename receipts to separately hashed file bytes.

    Groups retain their own order; their concatenation is not a global timeline.
    """
    old = {row['path']: row for row in before if row['kind'] == 'file'}
    new = {row['path']: row for row in after if row['kind'] == 'file'}
    mapped = []
    for stream in streams:
        for commit in stream['ordered_commits']:
            source, destination = old.get(commit['source']), new.get(commit['destination'])
            verified = source is not None and destination is not None and (
                source['sha256'] == commit['sha256'] == destination['sha256']
                and source['bytes'] == destination['bytes'])
            mapped.append({'receipt_group': stream['journal'], 'sequence': commit['sequence'],
                           'source': commit['source'], 'destination': commit['destination'],
                           'recorded_sha256': commit['sha256'], 'source_before': source,
                           'destination_after': destination, 'verified_bytes': verified})
    for album in (result or {}).get('whole_albums', []):
        if album['status'] != 'complete':
            continue
        for components, observed in album['descendants']:
            if observed['size'] is None:
                continue
            destination = reference_path(root, observed['reference'])
            actual = new.get(destination)
            expected = [row for row in original_operations
                        if row['kind'] in ('copy', 'move') and row['destination'] == destination]
            source = old.get(expected[0]['source']) if len(expected) == 1 else None
            # The original actual transfer supplies the expected byte relationship;
            # the native receipt supplies its actual observed destination.
            # No fake per-file transfer or globally ordered receipt is invented.
            mapped.append({'receipt_group': 'whole_album_' + str(album['operation']),
                           'operation_kind': 'whole_directory_rename', 'descendant_components': components,
                           'destination': destination, 'destination_after': actual,
                           'reported_bytes': observed['size'], 'expected_original_transfers': expected,
                           'expected_source_before': source,
                           'verified_bytes': actual is not None and source is not None
                           and actual['bytes'] == observed['size'] == source['bytes']
                           and actual['sha256'] == source['sha256']})
    return mapped


def planning_decisions(root, plan):
    """Lossless transport projection, kept explicitly separate from execution."""
    rows = []
    for value in plan['decisions']:
        if 'Album' in value:
            item = value['Album']; decision = item['decision']
            rows.append({'kind': 'album', 'source': reference_path(root, item['read']['source']['reference']),
                         'result': [decision[key] for key in ('is_album', 'should_process', 'album_name', 'artist_name')]})
        elif 'Single' in value:
            item = value['Single']
            rows.append({'kind': 'single', 'source': reference_path(root, item['read']['source']['reference']),
                         'result': item['artists'] or None})
        else:
            # An engine-only error/vanished decision is retained as a difference.
            rows.append(value)
    return rows


def physical_observations(before, after):
    old = {row['path']: row for row in before if row['kind'] == 'file'}
    new = {row['path']: row for row in after if row['kind'] == 'file'}
    added = [row for row in after if row['kind'] == 'file' and row['path'].startswith('target'+os.sep)
             and row['path'] not in old]
    removed = [row for row in before if row['kind'] == 'file' and row['path'].startswith('source'+os.sep)
               and row['path'] not in new]
    changed = [row['path'] for row in after if row['kind'] == 'file' and row['path'] in old
               and (row['sha256'], row['bytes']) != (old[row['path']]['sha256'], old[row['path']]['bytes'])]
    return {'new_destinations': added, 'removed_sources': removed, 'changed_existing_files': changed,
            'committed_destinations': len(added), 'moved_sources': len(removed)}


def comparison(name, expected, actual, *, available=True):
    return {'field': name, 'status': 'OPEN' if not available else 'MATCH' if actual == expected else 'MISMATCH',
            'expected': expected, 'actual': actual}


def compare_case(case, original, native):
    comparisons = [comparison('post_execution_tree_in_encounter_order',
                              comparable_snapshot(original['after']), comparable_snapshot(native['after']))]
    summary = (native.get('result') or {}).get('legacy_summary')
    comparisons.append(comparison('actual_legacy_summary', original['legacy_summary'], summary,
                                  available=summary is not None))
    plan = (native.get('preview_response') or {}).get('result', {}).get('plan')
    if 'planning_decisions' in native:
        comparisons.append(comparison('planning_only_decisions_in_encounter_order',
                                      original['ordered_decisions'], native['planning_decisions']))
    comparisons.append(comparison('planning_only_legacy_counters', original['legacy_summary'],
                                  plan.get('predicted_legacy_counters') if plan else None, available=plan is not None))
    expected_order = [{key: row[key] for key in ('kind', 'source', 'destination')}
                      for row in original['ordered_operations'] if row['kind'] in ('copy', 'move')]
    streams = native.get('physical_commit_streams', [])
    native_order = [{key: row[key] for key in ('kind', 'source', 'destination')}
                    for stream in streams for row in stream['ordered_commits']]
    has_whole = bool((native.get('result') or {}).get('whole_albums'))
    order_available = len(streams) == 1 and not has_whole
    # An empty successful job has an observable empty transfer order.
    if not expected_order and (native.get('result') or {}).get('status') == 'complete' and not streams and not has_whole:
        order_available = True
    comparisons.append(comparison('actual_transfer_order', expected_order, native_order,
                                  available=order_available))
    if not order_available:
        comparisons[-1]['gap'] = 'No single globally ordered physical transfer stream; no ordering fabricated across journals or whole-directory renames.'
    if 'committed_destination_mapping' in native:
        mappings = native['committed_destination_mapping']
        comparisons.append(comparison('committed_receipt_bytes', [],
                                      [row for row in mappings if not row['verified_bytes']]))
        destination_paths = [row['destination'] for row in mappings]
        comparisons.append(comparison('committed_receipt_coverage', [],
                                      [row['path'] for row in native['observed_physical']['new_destinations']
                                       if row['path'] not in destination_paths]))
        comparisons.append(comparison('unique_committed_receipts', len(destination_paths), len(set(destination_paths))))
    if native.get('result'):
        for field in ('committed_destinations', 'moved_sources'):
            comparisons.append(comparison('physical_' + field, native['observed_physical'][field],
                                          native['result'].get(field), available=field in native['result']))
    else:
        comparisons.append(comparison('actual_execution_result', 'complete', None, available=False))
    if case['group'] == 'duet':
        recognized = [row['result'] for row in original['ordered_decisions'] if row['kind'] == 'single']
        require(len(recognized) == 1 and len(recognized[0] or []) >= 2,
                'Duet did not exercise multiple actual original-model artists')
        require({row['method'] for row in original['model_calls']} == {'process_with_ner', 'verify_artist_with_sklearn'},
                'Duet must execute both original models')
        # Recognition order is a read-only planning comparison, never execution proof.
        singles = [row['Single']['artists'] for row in plan['decisions'] if 'Single' in row] if plan else None
        comparisons.append(comparison('planning_only_duet_artist_order', recognized, singles, available=plan is not None))
    if original['error']:
        comparisons.append(comparison('original_error', None, original['error']))
    if native['execution']['exit'] != 0:
        move_all = (case['group'] == 'duet' and not case['settings']['copy_mode'] and case['settings']['duet_mode']
                    and 'EXECUTION_REFUSED: Unsupported' in native['execution']['stderr']
                    and comparable_snapshot(native['before']) == comparable_snapshot(native['after']))
        comparisons.append(comparison('move_all_execution' if move_all else 'native_exit', 0,
                                      native['execution']['exit'], available=not move_all))
        if move_all:
            comparisons[-1]['gap'] = 'Move-all is OPEN until an actual backend executes the cohort; refusal is not passing parity.'
    if native['observed_physical']['changed_existing_files']:
        comparisons.append(comparison('existing_file_bytes', [], native['observed_physical']['changed_existing_files']))
    # Actual skip receipts are distinct from read-only planner decisions.
    actual_decisions = (native.get('result') or {}).get('legacy_decisions')
    comparisons.append(comparison('actual_decision_trace', original['ordered_decisions'],
                                  actual_decisions, available=actual_decisions is not None))
    if actual_decisions is None:
        comparisons[-1]['gap'] = 'Original actual decisions/destination eligibility and native raw preview decisions are retained. Native final result has no execution-derived skip-decision trace.'
    return comparisons



def verify_original_fixture_coverage(case, original):
    if case['group'] != 'settings':
        return
    albums = {row['source']: row['result'] for row in original['ordered_decisions'] if row['kind'] == 'album'}
    singles = {row['source']: row['result'] for row in original['ordered_decisions'] if row['kind'] == 'single'}
    require(albums.get(str(Path('source/protected')), [])[:2] == [True, False], 'Protected album fixture lost its original decision')
    require(albums.get(str(Path('source/processable')), [])[:2] == [True, True], 'Processable album fixture lost its original decision')
    require(singles.get(str(Path('source/root song.wav'))) == [EXISTING_ARTIST], 'Original root alias route not exercised')
    require(singles.get(str(Path('source/direct/child song.wav'))) == [MISSING_ARTIST], 'Original direct-child route not exercised')
    require((str(Path('source/direct/deeper/deep song.wav')) in singles) == (not case['settings']['main_folder_only']),
            'Original main-only/recursive fixture depth not exercised')
    prefix = str(Path('source/protected')) + os.sep
    before = [row for row in original['before'] if row['kind'] == 'file' and row['path'].startswith(prefix)]
    after = [row for row in original['after'] if row['kind'] == 'file' and row['path'].startswith(prefix)]
    require(before == after and len(before) == 3, 'Original protected album did not preserve all authored bytes')


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def run(binary, bundle, baseline, report):
    require(os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true', 'Paired execution requires the owned actual Windows CI runner')
    require(__debug__, 'Paired execution requires Python assertions enabled')
    binary, bundle, baseline = (path.resolve(strict=True) for path in (binary, bundle, baseline))
    require(platform.python_version() == '3.12.10', 'Expected pinned CPython 3.12.10')
    require(metadata.version('music-tag') == '0.4.3' and metadata.version('mutagen') == '1.47.0', 'Wrong metadata runtime')
    report = report.resolve()
    report.parent.mkdir(parents=True, exist_ok=True)
    require(not report.exists(), 'Refuse to overwrite paired evidence')
    scratch = Path(tempfile.mkdtemp(prefix='singles-v14-paired-')).resolve()
    bundle_before = {name: digest(bundle/name) for name in ASSETS}
    binary_before = digest(binary)
    result = {'schema_version': 1, 'result': 'OPEN', 'incomplete': True, 'actual_windows': True,
              'baseline': BASELINE, 'binary_sha256': binary_before,
              'runner': {'platform': platform.platform(), 'python': platform.python_version(),
                         'ci_run_id': os.environ.get('GITHUB_RUN_ID'), 'source_revision': os.environ.get('GITHUB_SHA')},
              'fixture_root': str(scratch), 'normalization': NORMALIZATION,
              'coverage': {'settings_combinations': 32, 'real_model_duets': 4}, 'cases': [],
              'historical_unsafe_exceptions': [
                  {'id': 'historical-duet-skipped-source-loss', 'scope': 'v14 Move-all deletes a source even when every destination is skipped; never required future behavior.', 'actual_windows_exercised_here': False},
                  {'id': 'historical-failed-move-false-success', 'scope': 'v14 increments song/artist counters after a failed move; never relabel this as observed physical success.', 'actual_windows_exercised_here': False}],
              'historical_fixture_sha256': digest(NATIVE/'tests/fixtures/full_contract/historical-unsafe.jsonl')}
    write_json(report, result)
    try:
        with original_oracle(baseline, bundle, scratch, include_sorter=True) as (_, sources, template):
            require(sources['singles_sorter_v5.py'] == SOURCE_SHA256, 'Wrong frozen v14 sorter source')
            result['original_source_sha256'] = sources
            for case in cases():
                row = {**case, 'status': 'OPEN', 'phase': 'running'}
                result['cases'].append(row)
                evidence = report.parent/(case['id']+'.json')
                data = {'case': case, 'incomplete': True}
                row['evidence'] = {'filename': evidence.name, 'sha256': None}
                write_json(evidence, data)
                write_json(report, result)
                try:
                    paired = scratch/case['id']
                    original_root, native_root = paired/'original', paired/'native'
                    author_case(original_root, case)
                    author_case(native_root, case)
                    alias_hash = digest(NATIVE/'tests/fixtures/full_contract/released-singer-list.csv')
                    data['primary_aliases'] = {side: digest(root/'app/singer-list.csv') for side, root in [('original', original_root), ('native', native_root)]}
                    require(set(data['primary_aliases'].values()) == {alias_hash}, 'Paired primary alias lists differ from frozen v14')
                    original_before, native_before = snapshot(original_root), snapshot(native_root)
                    data['paired_inputs'] = {'original': original_before, 'native': native_before,
                                             'encounter_order_equal': comparable_snapshot(original_before) == comparable_snapshot(native_before)}
                    write_json(evidence, data)
                    require(data['paired_inputs']['encounter_order_equal'],
                            'Fresh paired input trees or encounter order differ; this is input evidence, not an engine-policy mismatch')
                    original = json.loads(json.dumps(execute_original(original_root, case['settings'], template, ai=case['ai']), ensure_ascii=False))
                    original.update(before=original_before, after=snapshot(original_root))
                    data['original'] = original
                    write_json(evidence, data)
                    verify_original_fixture_coverage(case, original)
                    native = {'before': native_before}
                    data['native'] = native
                    native['preview'] = subprocess_record(binary, native_root, ['plan-files', '--json'], payload=native_request(native_root, bundle, case))
                    native['preview_response'] = json.loads(native['preview']['stdout'])
                    write_json(evidence, data)
                    require(snapshot(native_root) == native_before, 'Read-only preview changed owned input')
                    native['execution'] = subprocess_record(binary, native_root, native_args(bundle, case))
                    native['result'] = result_record(native['execution']['stderr'])
                    native['after'] = snapshot(native_root)
                    native['journals'] = read_journals(native_root)
                    native['physical_commit_streams'] = physical_commit_streams(native_root, native['journals'])
                    native['observed_physical'] = physical_observations(native_before, native['after'])
                    native['committed_destination_mapping'] = map_committed_destinations(native_root, native_before, native['after'], native['physical_commit_streams'], native['result'], original['ordered_operations'])
                    plan = native['preview_response'].get('result', {}).get('plan')
                    if plan is not None:
                        native['planning_decisions'] = planning_decisions(native_root, plan)
                    data['comparisons'] = compare_case(case, original, native)
                    statuses = [item['status'] for item in data['comparisons']]
                    row['status'] = 'MISMATCH' if 'MISMATCH' in statuses else 'OPEN' if 'OPEN' in statuses else 'MATCH'
                    row['open_outcomes'] = [item['field'] for item in data['comparisons'] if item['status'] == 'OPEN']
                    row['comparisons'] = [{'field': item['field'], 'status': item['status']} for item in data['comparisons']]
                except Exception as error:
                    row['status'] = 'ERROR'
                    data['error'] = {'type': type(error).__name__, 'message': str(error), 'traceback': traceback.format_exc()}
                finally:
                    row['phase'] = 'finished'
                    data['incomplete'] = False
                    write_json(evidence, data)
                    row['evidence'] = {'filename': evidence.name, 'sha256': digest(evidence)}
                    write_json(report, result)
                    print(case['id'], row['status'], flush=True)
        require(digest(binary) == binary_before, 'Selected executable changed')
        require({name: digest(bundle/name) for name in ASSETS} == bundle_before, 'Original model bundle changed')
        require(len(result['cases']) == 36, 'Incomplete paired matrix')
        result['incomplete'] = False
        statuses = [row['status'] for row in result['cases']]
        result['result'] = 'ERROR' if 'ERROR' in statuses else 'MISMATCH' if 'MISMATCH' in statuses else 'OPEN' if 'OPEN' in statuses else 'MATCH'
        result['totals'] = {status: statuses.count(status) for status in ('MATCH', 'OPEN', 'MISMATCH', 'ERROR')}
    except Exception as error:
        result['result'] = 'ERROR'
        result['fatal'] = {'type': type(error).__name__, 'message': str(error), 'traceback': traceback.format_exc()}
    finally:
        write_json(report, result)
    return 0 if result['result'] == 'MATCH' else 2 if result['result'] == 'OPEN' else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'bundle', 'baseline', 'report'):
        parser.add_argument('--'+name, type=Path, required=True)
    args = parser.parse_args()
    try:
        return run(args.binary, args.bundle, args.baseline, args.report)
    except Exception as error:
        # Retain Windows preflight failures too, without overwriting earlier evidence.
        if os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true' and not args.report.exists():
            args.report.parent.mkdir(parents=True, exist_ok=True)
            write_json(args.report, {'schema_version': 1, 'result': 'ERROR',
                       'fatal': {'type': type(error).__name__, 'message': str(error), 'traceback': traceback.format_exc()}})
        raise


if __name__ == '__main__':
    sys.exit(main())
