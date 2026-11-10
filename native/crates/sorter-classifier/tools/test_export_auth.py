"""Security regression: authentication cannot disappear under Python -O."""
import pathlib
import pickle
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

COMMIT = '78d001aee5ce49628bdffebd34540fa83ba32c12'
BLOB = '53945ae968c97125c890927e7ff97789351b2470'
EXPORT = pathlib.Path(__file__).with_name('export_original.py')

class ExportAuthentication(unittest.TestCase):
    def test_reject_before_unpickle(self):
        for responses in [[('0' * 40) + '\n'], [COMMIT+'\n', ('0' * 40)+'\n'], [COMMIT+'\n', BLOB+'\n', b'unauthenticated pickle bytes']]:
            with self.subTest(responses=len(responses)), tempfile.TemporaryDirectory() as output:
                with patch.object(sys, 'argv', [str(EXPORT), '--baseline', output, '--output', output]), patch.object(subprocess, 'check_output', side_effect=responses), patch.object(pickle, 'loads', side_effect=AssertionError('untrusted pickle executed')) as loads:
                    with self.assertRaises(ValueError):
                        runpy.run_path(str(EXPORT), run_name='__main__')
                    loads.assert_not_called()

if __name__ == '__main__':
    unittest.main()
