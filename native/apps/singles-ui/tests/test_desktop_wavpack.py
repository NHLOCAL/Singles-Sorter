"""Independent WavPack fixture/oracle checks, not a Rust execution claim."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest

from desktop_wavpack import CHECKS, generated_wavpack, inspect_wavpack, verify_repaired


class WavpackFixtures(unittest.TestCase):
    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_header_footer_and_nul_scalars_decode_at_exact_original_extent(self):
        for header, multiple in [(False, False), (True, False), (True, True)]:
            before = generated_wavpack(header, multiple)
            result = verify_repaired(before, generated_wavpack(header, multiple, repaired=True))
            self.assertEqual(result['original_bytes'], result['current_bytes'])
            self.assertEqual(len(result['pcm_sha256']), 64)
            self.assertEqual(result['fields']['Artist'], 'אבג\0גבא' if multiple else 'אבג')

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_readonly_fixture_preserves_real_text_and_layout(self):
        ordinary = inspect_wavpack(generated_wavpack(True))
        readonly = inspect_wavpack(generated_wavpack(True, readonly=True))
        self.assertEqual(ordinary[1:], readonly[1:])
        self.assertEqual(readonly[0]['Artist']['flags'], 1)
        self.assertEqual(readonly[0]['Artist']['value'].decode(), 'àáâ')

    @unittest.skipUnless(__debug__, 'Structural oracle requires assertions')
    def test_media_artwork_unknown_and_join_changes_are_rejected(self):
        before = generated_wavpack(True, True)
        after = generated_wavpack(True, True, repaired=True)
        for changed in (after[:40] + bytes([after[40] ^ 1]) + after[41:],
                        after.replace(b'owned retained artwork', b'OWNED retained artwork'),
                        after.replace(b'owned unknown item', b'OWNED unknown item'),
                        after.replace('אבג\0גבא'.encode(), 'אבג, גבא'.encode())):
            with self.assertRaises(AssertionError): verify_repaired(before, changed)

    def test_optimized_process_refuses_before_fixture_or_native_activity(self):
        result = subprocess.run([sys.executable, '-O', '-c',
                                 'from desktop_wavpack import generated_wavpack; generated_wavpack()'],
                                cwd=Path(__file__).parent, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Python assertions enabled', result.stderr)

    def test_every_actual_check_is_required_by_the_package_gate(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('wavpack_package', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.WAVPACK_CHECKS)


if __name__ == '__main__':
    unittest.main()
