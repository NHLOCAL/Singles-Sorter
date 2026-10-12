"""Portable owned fixture/parser checks, not native mutation acceptance."""
import importlib.util
from pathlib import Path
import unittest
from desktop_flac import CHECKS, FIELDS, UNKNOWN, fixture, generated_flac, inspect_flac


class FlacFixtures(unittest.TestCase):
    def test_all_fields_unknown_pairs_blocks_and_audio(self):
        for duplicate, opaque in [(False, False), (True, False), (True, True)]:
            data = generated_flac(duplicate=duplicate, opaque=opaque)
            vendor, values, blocks, offset, audio = inspect_flac(data)
            expected = UNKNOWN.copy()
            for key in FIELDS:
                expected.append(key.upper() + '=à')
                if duplicate:
                    expected.append(key + '=á')
            self.assertEqual(values, expected)
            self.assertEqual(vendor, fixture.VENDOR)
            self.assertEqual(audio, fixture.AUDIO)
            self.assertEqual(data[offset:], audio)
            self.assertEqual([b[0] & 127 for b in blocks], [0, 4, 3, 5, 6] + ([2] if opaque else []))

    def test_independent_parser_rejects_truncated_metadata(self):
        data = generated_flac(duplicate=True)
        _, _, _, offset, _ = inspect_flac(data)
        for end in (3, 7, 30, offset - 1):
            with self.assertRaises(AssertionError):
                inspect_flac(data[:end])

    def test_unchanged_fixture_has_no_repairable_text(self):
        _, values, _, _, _ = inspect_flac(generated_flac(unchanged=True))
        self.assertEqual(values, UNKNOWN + [key.upper() + '=שלום' for key in FIELDS])

    def test_all_actual_receipts_are_mandatory_in_package(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('flac_package_contract', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(module.FLAC_CHECKS, CHECKS)


if __name__ == '__main__':
    unittest.main()
