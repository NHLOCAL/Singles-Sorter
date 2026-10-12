"""Portable fixture contracts; actual copied Move requires the Windows renderer."""
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import desktop_copied_move as gate


class CopiedMoveFixtures(unittest.TestCase):
    def test_streaming_hash_and_fixed_auxiliary_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'owned.wav'
            data = b'chunk' * 20000
            path.write_bytes(data)
            self.assertEqual(gate.digest(path), hashlib.sha256(data).hexdigest())
            self.assertEqual(str(gate.auxiliary(path)), str(path) + ':notes')

    def test_cancellation_fixture_is_real_allocated_audio(self):
        with patch.object(gate.wave, 'open') as opened:
            writer = opened.return_value.__enter__.return_value
            gate.large_wave(Path('owned.wav'))
            self.assertEqual(writer.writeframesraw.call_count, 512)
            self.assertEqual({len(call.args[0]) for call in writer.writeframesraw.call_args_list}, {1024 * 1024})
            writer.setnchannels.assert_called_once_with(1)
            writer.setsampwidth.assert_called_once_with(2)

    def test_optimized_interpreter_refuses_before_creating_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'must-not-exist'
            code = f"import desktop_copied_move as g; g.exercise_copied_move(None,None,None,None,None,{str(root)!r},None,None)"
            result = subprocess.run([sys.executable, '-O', '-c', code], cwd=Path(gate.__file__).parent,
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Python assertions enabled', result.stderr)
            self.assertFalse(root.exists())

    def test_all_actual_receipts_are_mandatory_in_package(self):
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('copied_package_contract', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(module.COPIED_MOVE_CHECKS, gate.CHECKS)


if __name__ == '__main__':
    unittest.main()
