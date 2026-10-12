"""Portable contract checks, never a substitute for the Windows subprocess gate."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import check_execution as execution


class ExecutionContractTests(unittest.TestCase):
    def test_required_checks_match_archive_gate(self):
        self.assertEqual(execution.CHECKS, execution.package.EXECUTION_CHECKS)

    def test_native_result_requires_one_actual_result_line(self):
        result = {'status': 'complete', 'recovery_error': None}
        line = 'INFO - MusicSorter - result: ' + json.dumps(result)
        self.assertEqual(execution.result_record('INFO - MusicSorter - starting\n' + line), result)
        for invalid in ['', 'INFO - MusicSorter - progress: {}', line + '\n' + line]:
            with self.subTest(stderr=invalid), self.assertRaises(AssertionError):
                execution.result_record(invalid)

    def test_malformed_native_result_is_not_accepted(self):
        with self.assertRaises(json.JSONDecodeError):
            execution.result_record('INFO - MusicSorter - result: {')

    def test_authored_audio_and_tree_digest_detect_effects(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'בדיקה.wav'
            execution.write_wave(path)
            self.assertEqual(path.read_bytes()[:4], b'RIFF')
            before = execution.snapshot(root)
            path.read_bytes()
            self.assertEqual(execution.snapshot(root), before)
            path.write_bytes(path.read_bytes() + b'changed')
            self.assertNotEqual(execution.snapshot(root), before)

    def test_no_effect_snapshot_detects_empty_directory_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            before = execution.snapshot(root)
            (root / 'unexpected-empty-directory').mkdir()
            after = execution.snapshot(root)
            self.assertNotEqual(after, before)
            self.assertTrue(any(row[0] == 'unexpected-empty-directory' for row in after))

    def test_gate_rejects_optimized_parser_and_emits_no_report(self):
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'report.json'
            result = subprocess.run([
                sys.executable, '-O', str(Path(execution.__file__)),
                '--binary', 'unused', '--bundle', 'unused', '--report', str(report),
            ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Python assertions enabled', result.stderr)
            self.assertFalse(report.exists())

    def test_album_terminal_evidence_is_required_even_after_committed_files(self):
        import copy
        valid = dict(manifest_finished=True,incomplete_tail=False,pending_directories=[],pending_cleanup=[],pending_metadata=[],directory_metadata_status=[],
                     transfers={'append_blocked':False,'operations':[{'status':'Committed','copy_metadata':'Verified'}]},whole=None)
        execution.require_album_recovery(valid)
        for key in ('manifest_finished','incomplete_tail'):
            report=copy.deepcopy(valid);report[key]=not report[key]
            with self.assertRaises(ValueError): execution.require_album_recovery(report)
        report=copy.deepcopy(valid);report['transfers']['append_blocked']=True
        with self.assertRaises(ValueError): execution.require_album_recovery(report)

    def test_unconditional_guard(self):
        with self.assertRaisesRegex(ValueError, 'physical proof'):
            execution.require(False, 'physical proof')


if __name__ == '__main__':
    unittest.main()
