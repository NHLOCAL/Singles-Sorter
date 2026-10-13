"""Portable shape/guard tests, never actual restarted Windows acceptance."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from desktop_album_recovery import AlbumRecovery, CHECKS, selected_case
from desktop_recovery import file_hash


class AlbumRecoveryHelpers(unittest.TestCase):
    def test_requires_retained_actual_paths_and_positive_file_count(self):
        row = dict(case='copy', source='source', target='target', journal_path='journal', file_count=6)
        self.assertEqual(selected_case(row), row)
        for key in row:
            value = dict(row); value.pop(key)
            with self.assertRaises(ValueError): selected_case(value)
        for count in (0, -1, True, '6'):
            with self.assertRaises(ValueError): selected_case(dict(row, file_count=count))

    def test_move_restart_cannot_substitute_same_volume_whole_move(self):
        row = dict(case='root-move', source='source', target='target', journal_path='journal', file_count=5)
        for value in (None, False):
            with self.assertRaises(ValueError): selected_case(dict(row, cross_volume=value))
        self.assertEqual(selected_case(dict(row, cross_volume=True))['case'], 'root-move')

    def test_optimised_python_refuses_before_browser_or_report(self):
        command = [sys.executable, '-O', '-c', 'from desktop_album_recovery import AlbumRecovery; AlbumRecovery(None,None,None,None,None,None,None,None)']
        result = subprocess.run(command, cwd=Path(__file__).parent, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('requires Python assertions', result.stderr)

    def test_all_three_owned_scopes_required_before_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            owner = AlbumRecovery(None, None, None, None, None, None, None, directory)
            owner.cases = [dict(case='copy'), dict(case='root-move')]
            with self.assertRaises(ValueError): owner.finish()
            self.assertFalse(list(Path(directory).iterdir()))
            owner.cases = [dict(case='copy'), dict(case='root-move'), dict(case='root-move')]
            with self.assertRaises(ValueError): owner.finish()

    def test_streaming_digest_never_uses_whole_file_read_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'large-fixture'; data = b'owned' * 50000; path.write_bytes(data)
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('Unbounded read')):
                self.assertEqual(file_hash(path), hashlib.sha256(data).hexdigest())

    def test_consumer_and_package_require_exact_same_recovery_checks(self):
        import importlib.util
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('album_recovery_package', path)
        package = importlib.util.module_from_spec(spec); spec.loader.exec_module(package)
        self.assertEqual(CHECKS, package.ALBUM_RECOVERY_CHECKS)


if __name__ == '__main__': unittest.main()
