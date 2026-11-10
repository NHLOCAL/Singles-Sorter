"""Small all-source byte fixtures and negative tests; no Windows binaries executed."""
import importlib.util,json,unittest
from pathlib import Path
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('runtime_sources',HERE/'released-ner-export/runtime_sources.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class SourceBytes(unittest.TestCase):
    def setUp(self):
        self.name='spacy/owned.py';self.lf=b'# owned synthetic\nvalue = 1\n';self.crlf=self.lf.replace(b'\n',b'\r\n');self.h=m.digest(self.lf)
        self.lock={'files':{self.name:{'original_sha256':self.h,'windows_sha256':m.digest(self.crlf)}}}
    def validate(self,raw,platform='win32',reference=None):
        with patch.object(m,'WINDOWS',self.lock):return m.validate(self.name,raw,reference or self.h,platform)
    def test_exact_windows_raw_and_original_reference(self):self.assertEqual(self.validate(self.crlf)['raw_sha256'],m.digest(self.crlf))
    def test_linux_stays_byte_strict(self):
        self.validate(self.lf,'linux')
        with self.assertRaises(ValueError):self.validate(self.crlf,'linux')
    def test_windows_rejects_lf_even_if_reference_matches(self):
        with self.assertRaises(ValueError):self.validate(self.lf)
    def test_mixed_line_endings_rejected(self):
        with self.assertRaises(ValueError):self.validate(self.crlf.replace(b'\r\n',b'\n',1))
    def test_code_change_rejected(self):
        with self.assertRaises(ValueError):self.validate(self.crlf.replace(b'1',b'2'))
    def test_wrong_reference_rejected(self):
        with self.assertRaises(ValueError):self.validate(self.crlf,reference='0'*64)
    def test_changed_raw_allowlist_cannot_authorize_semantic_drift(self):
        raw=self.crlf.replace(b'1',b'2');self.lock['files'][self.name]['windows_sha256']=m.digest(raw)
        with self.assertRaises(ValueError):self.validate(raw)
    def test_every_pinned_windows_source_keeps_original_reference(self):
        release=json.loads((HERE/'released-ner-export/release-lock.json').read_text())
        windows=m.WINDOWS['files'];self.assertEqual(set(windows),set(release['runtime_sources']))
        for path,h in release['runtime_sources'].items():
            self.assertEqual(windows[path]['original_sha256'],h);self.assertEqual(windows[path]['lf_sha256'],h);self.assertEqual(windows[path]['upstream']['sha256'],h)
        oracle=json.loads((HERE/'oracle-lock.json').read_text())
        for name,w in m.WINDOWS['wheels'].items():
            self.assertTrue(any(x['filename']==w['filename'] and x['sha256']==w['sha256'] and x['platforms']==['win32'] for x in oracle['packages'][name]['wheels']))
if __name__=='__main__':unittest.main()
