"""Portable harness tests. These are not actual Windows parity evidence."""
import copy
import importlib.util
import itertools
import json
import os
from pathlib import Path, PureWindowsPath
import subprocess
import sys
import tempfile
import unittest

import check_windows_v14 as gate
from windows_v14_trace import MutationConfinementError, confine_mutations, relative


class PairedWindowsContract(unittest.TestCase):
    def test_exact_exhaustive_setting_matrix_and_four_duets(self):
        cases = gate.cases()
        matrix = [case for case in cases if case['group'] == 'settings']
        duets = [case for case in cases if case['group'] == 'duet']
        self.assertEqual(len(cases), 36)
        self.assertEqual(len({case['id'] for case in cases}), 36)
        self.assertEqual({tuple(case['settings'][key] for key in gate.SETTING_NAMES) for case in matrix},
                         set(itertools.product((False, True), repeat=5)))
        self.assertEqual({(case['settings']['copy_mode'], case['settings']['duet_mode']) for case in duets},
                         set(itertools.product((False, True), repeat=2)))
        self.assertTrue(all(case['ai'] for case in duets))

    def test_no_name_order_or_counter_tolerance(self):
        for expected, actual in [(['A', 'B'], ['B', 'A']), ('אב', 'באַ'), ('file.wav', 'File.wav'),
                                 ([['A', 2], ['B', 2]], [['B', 2], ['A', 2]]), (2, 3)]:
            self.assertEqual(gate.comparison('exact', expected, actual)['status'], 'MISMATCH')
        self.assertEqual(gate.comparison('missing', 0, None, available=False)['status'], 'OPEN')

    def test_snapshot_timestamp_normalization_is_narrow(self):
        rows = [{'path': 'source', 'absolute_path': '/owned/source', 'kind': 'directory', 'mtime_ns': 1},
                {'path': 'source/file.wav', 'absolute_path': '/owned/source/file.wav', 'kind': 'file', 'mtime_ns': 2}]
        view = gate.comparable_snapshot(rows)
        self.assertNotIn('mtime_ns', view[0])
        self.assertEqual(view[1]['mtime_ns'], 2)
        self.assertNotIn('absolute_path', view[1])
        self.assertEqual(rows[0]['mtime_ns'], 1)

    def test_confine_all_mutations_and_keep_outside_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory); root = base/'owned'; root.mkdir()
            outside = base/'outside'; outside.write_bytes(b'outside')
            inside = root/'inside'; inside.write_bytes(b'inside')
            with confine_mutations(root):
                (root/'allowed').write_bytes(b'yes')
                for action in [lambda: outside.write_bytes(b'bad'), lambda: outside.unlink(),
                               lambda: inside.rename(outside), lambda: outside.rename(inside),
                               lambda: os.open(outside, os.O_RDWR), lambda: os.utime(outside, None),
                               lambda: (root/'..'/'escape').mkdir()]:
                    with self.subTest(action=action), self.assertRaises(MutationConfinementError):
                        action()
            self.assertEqual(outside.read_bytes(), b'outside')
            self.assertEqual(inside.read_bytes(), b'inside')
            self.assertFalse((base/'escape').exists())

    def test_original_exception_catch_cannot_swallow_confinement(self):
        self.assertFalse(issubclass(MutationConfinementError, Exception))

    def test_relative_never_normalizes_case_or_unrelated_prefix(self):
        root = Path('/owned')
        self.assertEqual(relative(root, str(root)+os.sep+'A'), 'A')
        with self.assertRaises(ValueError):
            relative(root, str(root)+'2'+os.sep+'A')

    def test_result_duplicate_is_error_and_absence_not_success(self):
        line = 'INFO - MusicSorter - result: {"status":"complete"}'
        self.assertEqual(gate.result_record(line), {'status': 'complete'})
        self.assertIsNone(gate.result_record('error only'))
        with self.assertRaises(AssertionError):
            gate.result_record(line+'\n'+line)

    def test_physical_counts_are_independent_observed_files(self):
        def row(path, value):
            return {'path': path, 'kind': 'file', 'bytes': len(value), 'sha256': value}
        source = 'source'+os.sep+'song.wav'; target = 'target'+os.sep+'song.wav'
        before = [row(source, 'same')]; after = [row(target, 'same')]
        observed = gate.physical_observations(before, after)
        self.assertEqual(observed['committed_destinations'], 1)
        self.assertEqual(observed['moved_sources'], 1)
        self.assertEqual(observed['new_destinations'][0]['path'], target)
        self.assertEqual(gate.physical_observations(before, before)['committed_destinations'], 0)

    def test_matching_physical_counts_cannot_supply_legacy_summary(self):
        original = {'before': [], 'after': [], 'legacy_summary': {'songs_sorted': 1, 'artist_folders_created': 1,
                    'albums_processed': 0, 'top_artists': [['A', 1]]}, 'ordered_operations': [],
                    'ordered_decisions': [], 'error': None}
        native = {'before': [], 'after': [], 'result': {'status': 'complete', 'committed_destinations': 0, 'moved_sources': 0},
                  'preview_response': {'result': {'plan': {'predicted_legacy_counters': original['legacy_summary']}}},
                  'execution': {'exit': 0}, 'observed_physical': {'committed_destinations': 0, 'moved_sources': 0,
                                                               'changed_existing_files': []}}
        rows = {row['field']: row for row in gate.compare_case(gate.cases()[0], original, native)}
        self.assertEqual(rows['planning_only_legacy_counters']['status'], 'MATCH')
        self.assertEqual(rows['physical_committed_destinations']['status'], 'MATCH')
        self.assertEqual(rows['actual_legacy_summary']['status'], 'OPEN')
        native['result']['legacy_summary'] = copy.deepcopy(original['legacy_summary'])
        rows = {row['field']: row for row in gate.compare_case(gate.cases()[0], original, native)}
        self.assertEqual(rows['actual_legacy_summary']['status'], 'MATCH')

    def test_committed_receipts_map_exact_observed_bytes(self):
        before = [{'kind': 'file', 'path': 'source/a.wav', 'sha256': 'abc', 'bytes': 3}]
        after = [{'kind': 'file', 'path': 'target/a.wav', 'sha256': 'abc', 'bytes': 3}]
        streams = [{'journal': 'owned.jsonl', 'ordered_commits': [
            {'source': 'source/a.wav', 'destination': 'target/a.wav', 'sha256': 'abc', 'sequence': 7}]}]
        rows = gate.map_committed_destinations(Path('/owned'), before, after, streams, None)
        self.assertTrue(rows[0]['verified_bytes'])
        self.assertEqual(rows[0]['sequence'], 7)
        after[0]['sha256'] = 'tampered'
        self.assertFalse(gate.map_committed_destinations(Path('/owned'), before, after, streams, None)[0]['verified_bytes'])

    def test_whole_album_mapping_requires_unambiguous_source_and_full_hash(self):
        root = Path('/owned')
        destination = 'target'+os.sep+'a.wav'
        source = 'source'+os.sep+'a.wav'
        raw = (str(root)+os.sep+destination).encode('utf-16-le')
        reference = {'WindowsPath': {'utf16': [int.from_bytes(raw[i:i+2], 'little') for i in range(0, len(raw), 2)]}}
        before = [{'kind': 'file', 'path': source, 'sha256': 'original', 'bytes': 8}]
        after = [{'kind': 'file', 'path': destination, 'sha256': 'original', 'bytes': 8}]
        operations = [{'kind': 'move', 'source': source, 'destination': destination}]
        result = {'whole_albums': [{'operation': 0, 'status': 'complete', 'descendants': [
            [[{'Windows': [97]}], {'reference': reference, 'size': 8}]]}]}
        def verify(relations):
            rows = gate.map_committed_destinations(root, before, after, [], result, relations)
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]['operation_kind'], 'whole_directory_rename')
            return rows[0]['verified_bytes']
        self.assertTrue(verify(operations))
        after[0]['sha256'] = 'tampered'
        self.assertFalse(verify(operations), 'Equal size is not byte verification')
        after[0]['sha256'] = 'original'
        self.assertFalse(verify([]), 'A missing source relationship is not verified')
        self.assertFalse(verify(operations + operations), 'An ambiguous source relationship is not verified')
        missing = [dict(operations[0], source='source'+os.sep+'missing.wav')]
        self.assertFalse(verify(missing))
        after[0]['bytes'] = 7
        self.assertFalse(verify(operations))

    def test_observed_windows_extended_root_has_only_one_narrow_alias(self):
        root = PureWindowsPath(r'C:\owned\case\native')
        def reference(path):
            data = path.encode('utf-16-le')
            return {'WindowsPath': {'utf16': [int.from_bytes(data[i:i+2], 'little') for i in range(0, len(data), 2)]}}
        self.assertEqual(gate.reference_path(root, reference(r'\\?\C:\owned\case\native\target\Album\Song.WAV')),
                         r'target\Album\Song.WAV')
        for path in [r'\\?\c:\owned\case\native\target\Song.WAV',
                     r'\\?\C:\owned\case\native2\target\Song.WAV',
                     r'\\?\C:\Owned\case\native\target\Song.WAV',
                     r'\\server\share\case\native\target\Song.WAV']:
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.reference_path(root, reference(path))

    def test_native_planning_projection_retains_artist_and_decision_order(self):
        root = Path('/owned')
        def reference(name):
            data = (str(root)+os.sep+name).encode('utf-16-le')
            return {'WindowsPath': {'utf16': [int.from_bytes(data[i:i+2], 'little') for i in range(0, len(data), 2)]}}
        plan = {'decisions': [
            {'Single': {'read': {'source': {'reference': reference('z.wav')}}, 'artists': ['B', 'A']}},
            {'Single': {'read': {'source': {'reference': reference('a.wav')}}, 'artists': []}}]}
        rows = gate.planning_decisions(root, plan)
        self.assertEqual(rows, [{'kind': 'single', 'source': 'z.wav', 'result': ['B', 'A']},
                                {'kind': 'single', 'source': 'a.wav', 'result': None}])

    @unittest.skipUnless(importlib.util.find_spec('music_tag'), 'Metadata fixture authoring requires pinned oracle packages')
    def test_authored_pairs_have_identical_bytes_and_required_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory); case = gate.cases()[0]
            gate.author_case(base/'original', case); gate.author_case(base/'native', case)
            before = gate.snapshot(base/'original')
            self.assertEqual(gate.comparable_snapshot(before), gate.comparable_snapshot(gate.snapshot(base/'native')))
            paths = {row['path'] for row in before}
            for path in ['source/root song.wav', 'source/direct/child song.wav', 'source/direct/deeper/deep song.wav',
                         'source/protected/track 1.wav', 'source/processable/track 1.wav']:
                self.assertIn(str(Path(path)), paths)
            self.assertTrue(any(row.get('bytes', 0) > 1600 and row['path'].endswith('.wav') for row in before))

    @unittest.skipIf(os.name == 'nt', 'Non-Windows refusal checked only on a non-Windows host')
    def test_non_windows_never_produces_parity_report(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory)/'report.json'
            result = subprocess.run([sys.executable, gate.__file__, '--binary', 'unused', '--bundle', 'unused',
                                     '--baseline', 'unused', '--report', str(report)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('actual Windows CI runner', result.stderr)
            self.assertFalse(report.exists())


if __name__ == '__main__':
    unittest.main()
