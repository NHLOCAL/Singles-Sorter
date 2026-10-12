"""Portable owned Ogg fixture checks, never a native execution receipt."""
import importlib.util
from pathlib import Path
import unittest
from desktop_ogg import CHECKS, FIELDS, UNKNOWN, fixture, generated_ogg, inspect_ogg, verify_repaired


class OggFixtures(unittest.TestCase):
    def test_both_codecs_have_exact_five_fields_artwork_and_encoded_packets(self):
        for opus in (False, True):
            for duplicate in (False, True):
                before = generated_ogg(opus, duplicate=duplicate)
                vendor, values, suffix, packets, timing = inspect_ogg(before)
                expected = UNKNOWN.copy()
                for key in FIELDS:
                    expected.append(key.upper() + '=à')
                    if duplicate: expected.append(key + '=á')
                self.assertEqual(values, expected)
                self.assertEqual(vendor, b'Owned Ogg probe vendor')
                replacement = 'א, ב' if duplicate else 'א'
                after = fixture.replace_headers(before, fixture.comment(opus, UNKNOWN + [f'{key}={replacement}' for key in FIELDS], suffix), isolate=True)
                verify_repaired(before, after, duplicate)
                self.assertEqual(packets[2:], inspect_ogg(after)[3][2:])
                self.assertEqual(timing, inspect_ogg(after)[4])

    def test_parser_refuses_corrupt_and_chained_data(self):
        data = generated_ogg(False)
        for changed in (data + data, data[:-1], data[:-1] + bytes([data[-1] ^ 1])):
            with self.assertRaises(AssertionError): inspect_ogg(changed)

    def test_every_actual_receipt_is_required(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('ogg_package_contract', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(module.OGG_CHECKS, CHECKS)


if __name__ == '__main__': unittest.main()
