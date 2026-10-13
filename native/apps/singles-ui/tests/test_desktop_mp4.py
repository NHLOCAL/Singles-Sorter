"""Independent tiny-fixture tests, not execution of the native writer."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest

from desktop_mp4 import CHECKS, fixture, generated_mp4, verify_repaired, expected_repair


class Mp4ProductFixtures(unittest.TestCase):
    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_aac_alac_and_fast_start_exact_extent_decode(self):
        for alac, multiple in [(False, False), (True, False), (False, True)]:
            before = generated_mp4(alac=alac, multiple=multiple, fast_start=multiple)
            report = verify_repaired(before, fixture().repair(before, multiple), multiple)
            self.assertEqual(report['original_bytes'], report['current_bytes'])
            self.assertTrue(report['exact_original_extent'])
            self.assertEqual(len(report['pcm_sha256']), 64)
            if multiple:
                self.assertEqual(fixture().children(before)[1][0], b'moov')

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_outer_duplicate_after_unchanged_first_value_is_consolidated(self):
        data = generated_mp4(duplicate_outer=True)
        self.assertEqual(sum(kind == b'\xa9ART' for kind, _ in fixture().list_items(data)), 2)
        after, _ = expected_repair(data)
        self.assertEqual(sum(kind == b'\xa9ART' for kind, _ in fixture().list_items(after)), 1)
        self.assertEqual(verify_repaired(data, after)['fields']['artist'], 'שלום, ב')

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_opaque_refusal_fixture_keeps_intact_movie_header_and_all_fields(self):
        data = generated_mp4(opaque_structure=True)
        movie = next(box for kind, box in fixture().children(data) if kind == b'moov')
        kinds = [kind for kind, _ in fixture().children(movie[8:])]
        self.assertIn(b'mvhd', kinds); self.assertIn(b'uuid', kinds)
        _, fields = expected_repair(data)
        self.assertEqual(set(fields), set(fixture().KEYS))

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_corrupt_media_or_wrong_field_is_not_accepted(self):
        before = generated_mp4(); after = fixture().repair(before)
        for changed in (after.replace('א'.encode(), 'ב'.encode(), 1), after + b'changed'):
            with self.assertRaises(AssertionError): verify_repaired(before, changed)

    def test_optimized_process_refuses_before_fixture_or_native_activity(self):
        code = "from desktop_mp4 import generated_mp4; generated_mp4()"
        result = subprocess.run([sys.executable, '-O', '-c', code], cwd=Path(__file__).parent,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Python assertions enabled', result.stderr)

    def test_every_actual_mp4_check_is_mandatory_in_package(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('mp4_package', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.MP4_CHECKS)


if __name__ == '__main__':
    unittest.main()
