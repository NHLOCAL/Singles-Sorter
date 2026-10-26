"""Tiny independent fixture checks; actual writer/host checks require Windows."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest

from desktop_id3_container import CASES, CHECKS, generated_container, inspect_container, verify_repaired


class ContainerFixtures(unittest.TestCase):
    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_all_formats_and_both_id3_versions_decode_at_exact_original_extent(self):
        for kind in ('wav', 'aiff', 'dsf'):
            for version in (3, 4):
                with self.subTest(kind=kind, version=version):
                    before = generated_container(kind, version)
                    after = generated_container(kind, version, repaired=True)
                    result = verify_repaired(before, after, kind)
                    self.assertEqual(result['original_bytes'], result['current_bytes'])
                    self.assertTrue(result['exact_original_extent'])
                    self.assertEqual(len(result['pcm_sha256']), 64)

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_wave_info_is_retained_and_not_used_as_the_id3_title(self):
        before = generated_container('wav')
        self.assertIn(b'INFOINAM\x04\0\0\0old\0', before)
        self.assertEqual(inspect_container(before, 'wav')[0][b'TIT2'], 'çèé')
        self.assertIn(b'INFOINAM\x04\0\0\0old\0', generated_container('wav', repaired=True))

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_insufficient_padding_fixture_contains_no_free_tag_space(self):
        data = generated_container('aiff', 3, unsupported=True)
        fields, raw, start, end, _, _ = inspect_container(data, 'aiff')
        self.assertEqual(sum(len(frame) for frame in raw.values()), end - start - 10)
        self.assertEqual(len(fields), 5)

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_media_unknown_frame_artwork_and_wrong_field_changes_are_rejected(self):
        before = generated_container('dsf'); after = generated_container('dsf', repaired=True)
        for changed in (after[:92] + bytes([after[92] ^ 1]) + after[93:],
                        after.replace(b'owned-fixture-owner', b'OWNED-fixture-owner'),
                        after.replace(b'owned-retained-artwork', b'OWNED-retained-artwork'),
                        after.replace('א'.encode(), 'ב'.encode(), 1)):
            with self.assertRaises(AssertionError): verify_repaired(before, changed, 'dsf')

    def test_optimized_process_refuses_before_fixture_or_native_activity(self):
        result = subprocess.run([sys.executable, '-O', '-c',
                                 "from desktop_id3_container import generated_container; generated_container('wav')"],
                                cwd=Path(__file__).parent, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Python assertions enabled', result.stderr)

    def test_each_required_container_check_matches_the_archive_gate(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('container_package', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.ID3_CONTAINER_CHECKS)
        self.assertEqual({kind for kind, _, _ in CASES}, {'wav', 'aiff', 'dsf'})


if __name__ == '__main__':
    unittest.main()
