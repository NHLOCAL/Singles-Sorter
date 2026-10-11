"""Validate independent fixture generation/parsing, not native repair execution."""
from pathlib import Path
import unittest
from desktop_repair import FIELDS, PRIVATE, generated_mp3, inspect_mp3, sync_size, read_size


class RepairFixtures(unittest.TestCase):
    def test_both_owned_tag_versions_wrap_real_audio_and_all_five_fields(self):
        audio = (Path(__file__).resolve().parents[3] / 'tests/fixtures/media/blank.mp3').read_bytes()
        for version in (3, 4):
            for unsupported in (False, True):
                data = generated_mp3(audio, version, unsupported)
                fields, raw, offset, media = inspect_mp3(data)
                self.assertEqual(fields, {key: value[0] for key, value in FIELDS.items()})
                self.assertEqual(raw[b'PRIV'][10:], PRIVATE)
                self.assertEqual(media, audio)
                self.assertEqual(data[offset:], audio)

    def test_synchsafe_boundaries(self):
        for value in (0, 127, 128, 16383, 16384, (1 << 28) - 1):
            self.assertEqual(read_size(sync_size(value)), value)
        with self.assertRaises(AssertionError):
            read_size(b'\x80\x00\x00\x00')


if __name__ == '__main__':
    unittest.main()
