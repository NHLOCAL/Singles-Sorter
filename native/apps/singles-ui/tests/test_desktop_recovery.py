"""Portable checks of the restart acceptance helper; never native runtime proof."""
from pathlib import Path
import os
import tempfile
import unittest
from desktop_recovery import selected_cases, copied_cases, copied_snapshot, repair_cases, tree_snapshot, journal_snapshot


def snapshot(root):
    return tree_snapshot(root, permissions=lambda path: "portable-test-permissions")


class RecoveryAcceptanceHelpers(unittest.TestCase):
    def test_journal_snapshot_does_not_read_live_webview_contents(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / 'EBWebView'; cache.mkdir()
            locked = cache / 'locked.log'; locked.write_bytes(b'live-cache')
            journal = root / 'owned.transfers.jsonl'; journal.write_bytes(b'journal')
            original_open = Path.open
            def opened(path, *args, **kwargs):
                if cache in path.parents:
                    raise PermissionError('owned simulation of live WebView lock')
                return original_open(path, *args, **kwargs)
            with patch.object(Path, 'open', opened):
                before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            locked.write_bytes(b'changed-live-cache')
            (cache / 'new-live-cache').mkdir()
            info = cache.stat(); os.utime(cache, ns=(info.st_atime_ns, info.st_mtime_ns + 1000000000))
            self.assertEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))
            self.assertEqual(before['live_webview_subtree']['path'], 'EBWebView')
            self.assertNotIn('modified_ns', before['live_webview_subtree'])
            self.assertTrue(any(row['path'] == 'owned.transfers.jsonl' and row['sha256'] for row in before['protected_namespace']))

    def test_journal_snapshot_keeps_absent_cache_and_new_namespace_entries_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'owned.repair.jsonl').write_bytes(b'journal')
            before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            self.assertIsNone(before['live_webview_subtree'])
            original = root.stat()
            (root / 'unexpected-empty-directory').mkdir()
            os.utime(root, ns=(original.st_atime_ns, original.st_mtime_ns))
            self.assertNotEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))
            (root / 'unexpected-empty-directory').rmdir()
            (root / 'EBWebView').mkdir()
            os.utime(root, ns=(original.st_atime_ns, original.st_mtime_ns))
            self.assertNotEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))

    def test_journal_snapshot_keeps_profile_and_nested_cache_names_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'profile-v1' / 'EBWebView').mkdir(parents=True)
            profile = root / 'profile-v1' / 'EBWebView' / 'profile.json'; profile.write_bytes(b'profile')
            journal = root / 'owned.album.jsonl'; journal.write_bytes(b'journal')
            before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            profile.write_bytes(b'changed')
            self.assertNotEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))
            before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            journal.write_bytes(b'changed')
            self.assertNotEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))
            before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            info = journal.stat(); os.utime(journal, ns=(info.st_atime_ns, info.st_mtime_ns + 1000000000))
            self.assertNotEqual(before, journal_snapshot(root, permissions=lambda _: 'test-acl'))

    def test_journal_snapshot_retains_webview_boundary_identity_and_permissions(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as retained:
            root = Path(directory); cache = root / 'EBWebView'; cache.mkdir()
            before = journal_snapshot(root, permissions=lambda _: 'test-acl')
            original = root.stat()
            cache.rename(Path(retained) / 'old-live-boundary'); cache.mkdir()
            os.utime(root, ns=(original.st_atime_ns, original.st_mtime_ns))
            after = journal_snapshot(root, permissions=lambda _: 'test-acl')
            self.assertNotEqual(before['live_webview_subtree']['identity'], after['live_webview_subtree']['identity'])
            self.assertNotEqual(before, after)
            self.assertNotEqual(after, journal_snapshot(root, permissions=lambda path: 'changed-acl' if path == cache else 'test-acl'))

    def test_journal_snapshot_refuses_webview_files_and_all_links(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        import stat
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); cache = root / 'EBWebView'; cache.write_bytes(b'not-a-directory')
            with self.assertRaises(AssertionError):
                journal_snapshot(root, permissions=lambda _: 'test-acl')
            cache.unlink()
            cache.mkdir()
            original_lstat = Path.lstat
            for mode, attributes in [(stat.S_IFLNK | 0o777, 0), (stat.S_IFDIR | 0o700, 0x400)]:
                def observed(path):
                    if path == cache:
                        return SimpleNamespace(st_mode=mode, st_file_attributes=attributes)
                    return original_lstat(path)
                with patch.object(Path, 'lstat', observed), self.assertRaises(AssertionError):
                    journal_snapshot(root, permissions=lambda _: 'test-acl')

    def test_journal_snapshot_has_no_generic_cache_or_locked_file_exemption(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); other = root / 'Cache'; other.mkdir()
            (other / 'locked.log').write_bytes(b'not the documented live subtree')
            original_open = Path.open
            def opened(path, *args, **kwargs):
                if other in path.parents:
                    raise PermissionError('unexpected locked data')
                return original_open(path, *args, **kwargs)
            with patch.object(Path, 'open', opened), self.assertRaises(PermissionError):
                journal_snapshot(root, permissions=lambda _: 'test-acl')

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
