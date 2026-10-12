"""Portable fixture/receipt-contract tests, not a Windows execution claim."""
from pathlib import Path
import tempfile
import unittest
from desktop_albums import ALBUM, ARTIST, CHECKS, album_audio, fixture, files
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

    def test_every_actual_check_is_mandatory_in_packaging(self):
        import importlib.util
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('album_package_contract', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.ALBUM_CHECKS)


if __name__ == '__main__':
    unittest.main()
