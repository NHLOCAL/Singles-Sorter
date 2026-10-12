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
        line = '2026-10-12 02:00:00 - MusicSorter - INFO - result: ' + json.dumps(result)
        self.assertEqual(execution.result_record('INFO - MusicSorter - starting\n' + line), result)
        for invalid in ['', 'INFO - MusicSorter - progress: {}', line + '\n' + line]:
            with self.subTest(stderr=invalid), self.assertRaises(AssertionError):
                execution.result_record(invalid)

    def test_malformed_native_result_is_not_accepted(self):
        with self.assertRaises(json.JSONDecodeError):
            execution.result_record('2026-10-12 02:00:00 - MusicSorter - INFO - result: {')

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
        valid = dict(manifest_finished=True,incomplete_tail=False,pending_directories=[],pending_cleanup=[],pending_metadata=[],directory_metadata_status=[],directories=[{'status':'SameIdentity'}],
                     transfers={'append_blocked':False,'operations':[{'status':'Committed','copy_metadata':'Verified'}]},whole=None)
        execution.require_album_recovery(valid)
        for key in ('manifest_finished','incomplete_tail'):
            report=copy.deepcopy(valid);report[key]=not report[key]
            with self.assertRaises(ValueError): execution.require_album_recovery(report)
        report=copy.deepcopy(valid);report['transfers']['append_blocked']=True
        with self.assertRaises(ValueError): execution.require_album_recovery(report)
        for status in ('Absent','Replaced','Unverified'):
            report=copy.deepcopy(valid);report['directories'][0]['status']=status
            with self.assertRaises(ValueError): execution.require_album_recovery(report)

    def test_copied_move_recovery_requires_terminal_metadata_evidence(self):
        import copy
        valid = dict(directories=dict(complete=True,incomplete_tail=False,pending=[],rejected=[],observed=[['target/artist', {'Test': {'id': 'owned'}}]]),
                     transfers=dict(append_blocked=False,operations=[dict(status='Committed',copy_metadata='Verified')]))
        execution.require_copied_recovery(valid)
        for key, value in [('complete',False),('incomplete_tail',True),('pending',['target/artist']),('rejected',['target/artist'])]:
            report=copy.deepcopy(valid);report['directories'][key]=value
            with self.assertRaises(ValueError): execution.require_copied_recovery(report)
        for status in ('Mismatch','Unverified'):
            report=copy.deepcopy(valid);report['transfers']['operations'][0]['copy_metadata']=status
            with self.assertRaises(ValueError): execution.require_copied_recovery(report)
        report=copy.deepcopy(valid);report['transfers']['append_blocked']=True
        with self.assertRaises(ValueError): execution.require_copied_recovery(report)

    def test_negative_trial_exempts_only_its_new_log_and_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'source').mkdir();(root/'source/track').write_bytes(b'owned')
            before=execution.snapshot(root)
            (root/'logs').mkdir();log=root/'logs/music_sorter_20261012_020000_123.log'
            text='2026-10-12 02:00:00 - MusicSorter - ERROR - refused\n';log.write_bytes(text.encode('utf-8'))
            receipt=execution.verify_new_log(root,{},text)
            execution.no_effect_except_observed_log(before,execution.snapshot(root),receipt)
            (root/'source/extra').mkdir()
            with self.assertRaises(ValueError): execution.no_effect_except_observed_log(before,execution.snapshot(root),receipt)

    def test_log_fixture_uses_raw_utf8_not_platform_text_newline_translation(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'logs').mkdir();log=root/'logs/music_sorter_20261012_020000_123.log'
            text='2026-10-12 02:00:00 - MusicSorter - INFO - שלום\n'
            # Reproduce Windows Path.write_text's default translation on every OS.
            # The native sink uses write_all on UTF-8 bytes, not TextIOWrapper.
            with log.open('w',encoding='utf-8',newline='\r\n') as stream: stream.write(text)
            self.assertEqual(log.read_bytes(),text.replace('\n','\r\n').encode('utf-8'))
            with self.assertRaisesRegex(ValueError,'UTF-8 file and stderr'):
                execution.verify_new_log(root,{},text)
            log.write_bytes(text.encode('utf-8'))
            self.assertEqual(execution.verify_new_log(root,{},text)['bytes'],len(text.encode('utf-8')))

    def test_diagnostic_log_never_exempts_changes_to_existing_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'logs').mkdir();old=root/'logs/old.log';old.write_bytes(b'keep')
            before=execution.log_inventory(root)
            (root/'logs/music_sorter_20261012_020000_123.log').write_text('new',encoding='utf-8')
            old.write_bytes(b'changed')
            with self.assertRaises(ValueError): execution.verify_new_log(root,before,'new')

    def test_unconditional_guard(self):
        with self.assertRaisesRegex(ValueError, 'physical proof'):
            execution.require(False, 'physical proof')


if __name__ == '__main__':
    unittest.main()
