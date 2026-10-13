"""Native acceptance must not run with structural assertions disabled."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent


class AssertionGuard(unittest.TestCase):
    def test_optimized_native_entrypoint_refuses_before_interaction(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'uncreated-evidence'
            env = dict(os.environ, SINGLES_UI_EVIDENCE=str(output))
            result = subprocess.run([sys.executable, '-O', str(HERE / 'desktop.py')], env=env,
                                    capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('requires Python assertions enabled', result.stderr)
            self.assertFalse(output.exists())

    def test_optimized_standalone_ogg_helper_refuses_before_file_creation(self):
        self.check_helper('ogg')

    def test_optimized_standalone_flac_helper_refuses_before_file_creation(self):
        self.check_helper('flac')

    def check_helper(self, kind):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'uncreated-source'
            code = ('import sys;sys.path.insert(0,' + repr(str(HERE)) + ');'
                    f'from desktop_{kind} import exercise_{kind};'
                    f'exercise_{kind}(None,None,None,None,None,' + repr(str(root)) + ',None)')
            result = subprocess.run([sys.executable, '-O', '-c', code],
                                    capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('requires Python assertions enabled', result.stderr)
            self.assertFalse(root.exists())


if __name__ == '__main__': unittest.main()
