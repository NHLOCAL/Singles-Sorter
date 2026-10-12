"""Portable checks of the restart acceptance helper; never native runtime proof."""
from pathlib import Path
import os
import tempfile
import unittest
from desktop_recovery import selected_cases, copied_cases, copied_snapshot, repair_cases, tree_snapshot


def snapshot(root):
    return tree_snapshot(root, permissions=lambda path: "portable-test-permissions")


class RecoveryAcceptanceHelpers(unittest.TestCase):
    def test_completed_copy_and_move_are_both_required(self):
        copy, move = {'copy_mode': True}, {'copy_mode': False}
        self.assertEqual(selected_cases({'cases': [{'case': 'stale'}, move, copy]}), [copy, move])
        for missing in [[], [copy], [move]]:
            with self.assertRaises(StopIteration):
                selected_cases({'cases': missing})

    def test_copied_restart_requires_surviving_exact_owned_roots(self):
        rows = [{'case': name, 'cross_volume': False, 'source': 'owned-source', 'target': 'owned-target',
                 'journal_path': 'owned-journals', 'destinations': [{'name': 'owned.wav'}]}
                for name in ('duet', 'mixed')]
        self.assertEqual(copied_cases({'cases': rows[::-1]}), rows)
        with self.assertRaises(StopIteration):
            copied_cases({'cases': rows[:1]})
        for field in ('source', 'target', 'journal_path', 'destinations'):
            bad = [dict(row) for row in rows]
            bad[0].pop(field)
            with self.assertRaises(ValueError):
                copied_cases({'cases': bad})
        bad = [dict(row) for row in rows]
        bad[0]['cross_volume'] = True
        with self.assertRaises(ValueError):
            copied_cases({'cases': bad})

    def test_copied_snapshot_includes_generated_named_stream_bytes(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            song = root / 'owned.wav'
            song.write_bytes(b'owned-main')
            Path(str(song) + ':notes').write_bytes(b'owned-ADS')
            with patch('desktop_recovery.tree_snapshot', return_value={'portable': 'fixed'}):
                before = copied_snapshot(root)
                Path(str(song) + ':notes').write_bytes(b'changed-ADS')
                self.assertNotEqual(before, copied_snapshot(root))
                self.assertEqual(song.read_bytes(), b'owned-main')

    def test_fix_restart_requires_completed_partial_and_exact_native_roots(self):
        complete = {'case': 'id3v24', 'source': 'owned-source', 'journal_path': 'owned-journals'}
        partial = {'case': 'unsupported', 'source': 'owned-partial', 'journal_path': 'owned-journals'}
        self.assertEqual(repair_cases({'cases': [partial, complete]}), [complete, partial])
        with self.assertRaises(StopIteration):
            repair_cases({'cases': [complete]})
        with self.assertRaises(ValueError):
            repair_cases({'cases': [complete, {'case': 'unsupported'}]})

    def test_snapshot_detects_content_and_name_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            song = root / 'owned.mp3'
            song.write_bytes(b'original')
            before = snapshot(root)
            self.assertEqual(before, snapshot(root))
            song.write_bytes(b'changed')
            self.assertNotEqual(before, snapshot(root))
            song.write_bytes(b'original')
            song.rename(root / 'renamed.mp3')
            self.assertNotEqual(before, snapshot(root))


    def test_snapshot_detects_empty_directories_permissions_and_modified_time(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            song = root / 'owned.mp3'
            song.write_bytes(b'original')
            before = snapshot(root)
            self.assertTrue(any(row['path'] == '.' and row['directory'] for row in before))
            (root / 'empty').mkdir()
            after = snapshot(root)
            self.assertTrue(any(row['path'] == 'empty' and row['directory'] for row in after))
            self.assertNotEqual(before, after)
            before = after
            info = song.stat()
            os.utime(song, ns=(info.st_atime_ns, info.st_mtime_ns + 1000000000))
            self.assertNotEqual(before, snapshot(root))
            before = snapshot(root)
            original_mode = song.stat().st_mode
            try:
                song.chmod(0o400)
                self.assertNotEqual(before, snapshot(root))
            finally:
                song.chmod(original_mode)


if __name__ == '__main__':
    unittest.main()
