"""Small synthetic-only failure-containment tests; no network or build."""
import hashlib, tempfile, unittest, zipfile
from pathlib import Path
from unittest.mock import patch
import acquire_pcre2 as a
class Acquisition(unittest.TestCase):
    def fixture(self, name='pcre2-10.46/a.txt'):
        d=tempfile.TemporaryDirectory();self.addCleanup(d.cleanup);base=Path(d.name);p=base/'a.zip'
        with zipfile.ZipFile(p,'w') as z:z.writestr(name,b'owned synthetic')
        lock=dict(a.LOCK,sha256=hashlib.sha256(p.read_bytes()).hexdigest())
        return base,p,lock
    def test_valid(self):
        base,p,lock=self.fixture();a.unpack(p,base/'out',lock);self.assertEqual((base/'out/pcre2-10.46/a.txt').read_bytes(),b'owned synthetic')
    def test_hash_before_extraction(self):
        base,p,lock=self.fixture();lock['sha256']='0'*64
        with self.assertRaises(ValueError):a.unpack(p,base/'out',lock)
        self.assertFalse((base/'out').exists())
    def test_unsafe_names(self):
        for name in ['../escape','pcre2-10.46/../escape','/absolute','pcre2-10.46/x\\y','pcre2-10.46/C:ads','other/a','pcre2-10.46/bad.', 'pcre2-10.46/NUL.txt', 'pcre2-10.46/COM1', 'pcre2-10.46/LPT².txt', 'pcre2-10.46/x/./y', 'pcre2-10.46/x//y', 'pcre2-10.46/bad\x01', 'pcre2-10.46/question?', 'pcre2-10.46/directory//']:
            with self.subTest(name=name):
                base,p,lock=self.fixture(name)
                with self.assertRaises(ValueError):a.unpack(p,base/'out',lock)
                self.assertFalse((base/'out').exists())
    def test_expansion_limit(self):
        base,p,lock=self.fixture();lock['maximum_expanded_bytes']=1
        with self.assertRaises(ValueError):a.unpack(p,base/'out',lock)
    def test_existing_destination(self):
        base,p,lock=self.fixture()
        with self.assertRaises(ValueError):a.acquire(base,p)
    def test_acquire_failure_leaves_no_destination(self):
        base,p,lock=self.fixture()
        with self.assertRaises(ValueError):a.acquire(base/'out',p)
        self.assertFalse((base/'out').exists());self.assertEqual(sorted(x.name for x in base.iterdir()),['a.zip'])
    def test_caller_never_builds_after_invalid_acquisition(self):
        base,p,lock=self.fixture()
        with patch.object(a, 'build') as build:
            with self.assertRaises(ValueError):a.prepare(base/'out',p,True)
            build.assert_not_called()
        self.assertFalse((base/'out').exists())
    def test_urls(self):
        for url in ['http://github.com/a','https://github.com.evil.test/a','https://evil.test/a','https://user@github.com/a','https://github.com:444/a']:
            with self.assertRaises(ValueError):a.allowed(url)
if __name__=='__main__':unittest.main()
