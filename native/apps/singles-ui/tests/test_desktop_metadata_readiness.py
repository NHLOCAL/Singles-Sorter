"""Fixture/harness contracts; actual Windows/WebView proof runs separately."""
import ast
from pathlib import Path
import subprocess
import sys
import unittest

import desktop_metadata_readiness as subject


class MetadataReadiness(unittest.TestCase):
    def test_each_authored_representation_retains_identical_audio(self):
        media = b'owned-mpeg-payload\xff\xfb'
        for version, flags in [(2, 0), (3, 0), (3, 128), (3, 64), (3, 192)]:
            value = subject.audio_fixture(media, version, flags)
            self.assertEqual(value[:6], b'ID3' + bytes([version, 0, flags]))
            size = 0
            for byte in value[6:10]:
                self.assertLess(byte, 128)
                size = (size << 7) | byte
            self.assertEqual(value[10 + size:], media)
            body = value[10:10 + size]
            if flags & 128:
                self.assertIn(b'\xff\0\xfe', body)
                body = body.replace(b'\xff\0', b'\xff')
            if flags & 64:
                self.assertEqual(body[:10], b'\0\0\0\x06\0\0\0\0\0\0')
                body = body[10:]
            fields = {}
            while body:
                width, header = (3, 6) if version == 2 else (4, 10)
                name = body[:width]
                length = int.from_bytes(body[width:width + (3 if version == 2 else 4)], 'big')
                value = body[header:header + length]
                self.assertEqual(value[:3], b'\x01\xff\xfe')
                fields[name] = value[3:-2].decode('utf-16le')
                body = body[header + length:]
            self.assertEqual(len(fields), 6)
            self.assertEqual(fields[b'TP1' if version == 2 else b'TPE1'], subject.ARTIST)
            self.assertEqual(fields[b'TRK' if version == 2 else b'TRCK'], '2/9')

    def test_existing_id3_header_is_replaced_not_nested(self):
        first = subject.audio_fixture(b'media', 3, 0)
        second = subject.audio_fixture(first, 2, 0)
        self.assertEqual(second, subject.audio_fixture(b'media', 2, 0))

    def test_optimization_is_refused_before_files_or_ui(self):
        code = 'from desktop_metadata_readiness import exercise_metadata_readiness; exercise_metadata_readiness(None,None,None,None,None,None,None,None)'
        result = subprocess.run([sys.executable, '-O', '-c', code], cwd=Path(subject.__file__).parent,
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Readiness acceptance requires Python assertions enabled', result.stderr)

    def test_native_fixture_gate_remains_no_apply_and_is_bound_into_both_package_lanes(self):
        root = Path(subject.__file__).parents[1]
        tree = ast.parse(Path(subject.__file__).read_text(encoding='utf-8'))
        clicks = [node.args[0].value for node in ast.walk(tree) if isinstance(node, ast.Call)
                  and isinstance(node.func, ast.Name) and node.func.id == 'click'
                  and node.args and isinstance(node.args[0], ast.Constant)]
        self.assertIn('#review-apply', clicks); self.assertIn('#apply-back', clicks)
        self.assertNotIn('#confirm-apply', clicks)
        source = (root / 'tests/desktop.py').read_text(encoding='utf-8')
        self.assertIn('metadata_readiness_checks = exercise_metadata_readiness(', source)
        self.assertIn("'metadata_readiness_checks':metadata_readiness_checks", source)
        package = (root.parents[1] / 'packaging/windows-desktop-preview/package_desktop.py').read_text(encoding='utf-8')
        for check in subject.CHECKS:
            self.assertIn(check, package)
        self.assertIn("METADATA_READINESS_CHECKS <= set(report.get('metadata_readiness_checks', []))", package)


if __name__ == '__main__':
    unittest.main()
