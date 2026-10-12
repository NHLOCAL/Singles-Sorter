"""Portable contract checks; actual Windows consumers remain required."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from desktop_whole_album_recovery import WholeAlbumRecovery, selected_whole_case, require_only_owned_append
from desktop_ai import tree_snapshot
import hashlib


class WholeAlbumRecoveryHelpers(unittest.TestCase):
    def test_root_parent_baseline_tracks_only_the_owned_target_append(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'target'; target.mkdir()
            song = target / 'song.mp3'; song.write_bytes(b'original')
            unrelated = root / 'unrelated'; unrelated.write_bytes(b'kept')
            snapshot = lambda: tree_snapshot(root, permissions=lambda _: 'owned')
            before = snapshot(); song.write_bytes(b'original-added'); after = snapshot()
            expected = hashlib.sha256(b'original-added').hexdigest()
            require_only_owned_append(before, after, 'target/song.mp3', expected, 6)
            with self.assertRaises(ValueError): require_only_owned_append(before, after, 'target/song.mp3', 'wrong', 6)
            unrelated.write_bytes(b'changed')
            with self.assertRaises(ValueError): require_only_owned_append(before, snapshot(), 'target/song.mp3', expected, 6)

    def test_only_completed_same_volume_move_receipts_are_eligible(self):
        row = dict(case='root-move', source_retained=False, source='source', target='target', journal_path='journal', file_count=5)
        self.assertEqual(selected_whole_case(row), row)
        for change in [dict(case='copy'), dict(source_retained=True), dict(cross_volume=True), dict(file_count=0), dict(file_count=True), dict(journal_path='')]:
            with self.assertRaises(ValueError): selected_whole_case(dict(row, **change))
        for key in row:
            incomplete = dict(row); incomplete.pop(key)
            with self.assertRaises(ValueError): selected_whole_case(incomplete)

    def test_incomplete_scopes_cannot_write_a_pass_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            owner = WholeAlbumRecovery(None, None, None, None, None, None, None, directory)
            owner.cases = [dict(case='root-move')]
            with self.assertRaises(ValueError): owner.finish()
            self.assertFalse(list(Path(directory).iterdir()))

    def test_optimized_python_refuses_before_native_interaction(self):
        result = subprocess.run([sys.executable, '-O', '-c', 'from desktop_whole_album_recovery import WholeAlbumRecovery; WholeAlbumRecovery(None,None,None,None,None,None,None,None)'], cwd=Path(__file__).parent, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('requires Python assertions', result.stderr)


if __name__ == '__main__': unittest.main()
