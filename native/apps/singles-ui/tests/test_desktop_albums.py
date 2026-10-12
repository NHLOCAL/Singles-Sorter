"""Portable fixture/receipt-contract tests, not a Windows execution claim."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock
from types import SimpleNamespace
from desktop_albums import digest, ALBUM, ARTIST, CHECKS, album_audio, fixture, files, metadata, snapshot
from desktop_repair import read_size


class AlbumFixtures(unittest.TestCase):
    def test_album_frames_wrap_exact_original_fixture_audio(self):
        audio = (Path(__file__).resolve().parents[3] / 'tests/fixtures/media/blank.mp3').read_bytes()
        for track in (1, 2, 3):
            data = album_audio(audio, track)
            self.assertEqual(data[:6], b'ID3\x04\x00\x00')
            end = 10 + read_size(data[6:10]); at = 10; fields = {}
            while at < end:
                name = data[at:at+4]; length = read_size(data[at+4:at+8])
                self.assertEqual(data[at+8:at+11], b'\x00\x00\x03')
                fields[name] = data[at+11:at+10+length].decode('utf-8')
                at += 10 + length
            self.assertEqual(fields, {b'TPE1': ARTIST, b'TPE2': ARTIST, b'TALB': ALBUM, b'TRCK': str(track)})
            self.assertEqual(data[end:], audio)

    def test_fixture_has_nested_bytes_sidecar_and_optional_single(self):
        with tempfile.TemporaryDirectory() as directory:
            for root_album in (False, True):
                source, target, album = fixture(Path(directory) / str(root_album), b'owned fixture bytes', root_album)
                self.assertEqual(len(files(album)), 5)
                self.assertEqual(len(files(source)), 5 if root_album else 6)
                self.assertTrue((album / 'booklet/nested/readme.txt').is_file())
                self.assertEqual(list(target.iterdir()), [])

    def test_copy_metadata_excludes_fresh_album_container(self):
        root = Mock()
        child = Mock()
        child.relative_to.return_value.as_posix.return_value = 'booklet'
        child.stat.return_value = SimpleNamespace(st_mtime_ns=123, st_file_attributes=1)
        root.rglob.return_value = [child]
        self.assertEqual(metadata(root), {'booklet': (123, True)})
        root.stat.assert_not_called()

    def test_no_effect_snapshot_detects_empty_directories_and_metadata(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            file = root / 'track'; file.write_bytes(b'owned')
            before = snapshot(root)
            (root / 'empty').mkdir()
            self.assertNotEqual(snapshot(root), before)
            self.assertIn('empty', snapshot(root))
            before = snapshot(root)
            os.utime(file, ns=(1_600_000_000_000_000_000, 1_600_000_000_000_000_000))
            self.assertNotEqual(snapshot(root), before)
            self.assertIn('.', before)

    def test_generic_album_digest_streams_instead_of_reading_whole_media(self):
        from unittest.mock import patch
        import hashlib
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'owned-media'
            path.write_bytes(b'owned content' * 100)
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('whole-file read')):
                self.assertEqual(digest(path), hashlib.sha256(b'owned content' * 100).hexdigest())

    def test_every_actual_check_is_mandatory_in_packaging(self):
        import importlib.util
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('album_package_contract', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.ALBUM_CHECKS)


if __name__ == '__main__':
    unittest.main()
