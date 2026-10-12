"""Validate independent fixture generation/parsing, not native repair execution."""
from pathlib import Path
import unittest
import tempfile
from desktop_repair import FIELDS, PRIVATE, generated_mp3, inspect_mp3, sync_size, read_size, assert_observed_filename


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



class ObservedFilenameAssertion(unittest.TestCase):
    def test_actual_gate_checks_exact_current_label_title_and_accessible_name(self):
        with tempfile.TemporaryDirectory() as directory:
            fixed = Path(directory) / 'שם מתוקן.mp3'
            fixed.write_bytes(b'owned test evidence')
            answers = [[fixed.name], fixed.name, 'פרטים: ' + fixed.name]
            def check(values):
                replies = iter(values)
                assert_observed_filename(lambda _script: next(replies), fixed)
            check(answers)
            for index, wrong in [(0, ['old_name.mp3']), (1, 'old_name.mp3'), (2, 'פרטים: old_name.mp3')]:
                changed = answers.copy(); changed[index] = wrong
                with self.assertRaises(AssertionError): check(changed)
            fixed.unlink()
            with self.assertRaises(AssertionError): check(answers)


if __name__ == '__main__':
    unittest.main()
