"""Owned temporary files must not broaden the optional original-process trial."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from cold_original import prepare_runtime_temp
from windows_v14_trace import MutationConfinementError, confine_mutations


class ColdOriginalTemporaryFiles(unittest.TestCase):
    def test_runtime_temp_is_created_inside_the_existing_mutation_guard(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            with patch.dict(os.environ), patch.object(tempfile, 'tempdir', None):
                with confine_mutations(root):
                    expected = prepare_runtime_temp(root)
                    self.assertEqual(Path(tempfile.gettempdir()), expected)
                    for name in ('TMP', 'TEMP', 'TMPDIR'):
                        self.assertEqual(os.environ[name], str(expected))
                    with tempfile.NamedTemporaryFile(delete=False) as stream:
                        stream.write(b'owned import-time scratch')
                        observed = Path(stream.name)
                    self.assertEqual(observed.parent, expected)
                    observed.unlink()
                    self.assertEqual(list(expected.iterdir()), [])
                    with self.assertRaises(MutationConfinementError):
                        (root.parent / 'outside-cold-original-trial').write_bytes(b'refused')

    def test_existing_runtime_directory_is_not_adopted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / 'runtime-temporary').mkdir()
            (root / 'runtime-temporary' / 'sentinel').write_bytes(b'kept')
            with patch.dict(os.environ), patch.object(tempfile, 'tempdir', None):
                with confine_mutations(root):
                    with self.assertRaises(FileExistsError):
                        prepare_runtime_temp(root)
                self.assertIsNone(tempfile.tempdir)
            self.assertEqual((root / 'runtime-temporary' / 'sentinel').read_bytes(), b'kept')


if __name__ == '__main__':
    unittest.main()
