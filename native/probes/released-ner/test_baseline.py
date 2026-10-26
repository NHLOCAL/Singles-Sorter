"""Small source-only tests for the isolated baseline staging contract."""
import hashlib,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import prepare_baseline as p
class Baseline(unittest.TestCase):
    def fixture(self):
        d=tempfile.TemporaryDirectory();self.addCleanup(d.cleanup);root=Path(d.name)
        files={'baseline/math-lib.rs':'original math','baseline/raw-ner.rs':'original raw',
               'released-ner-math/src/lib.rs':'optimized math','released-ner-text/src/bin/raw-ner.rs':'optimized raw',
               'released-ner-text/src/lib.rs':'pub mod inference;\npub mod tokenizer;\n',
               'released-ner-text/tests/bounded.rs':'requires bounded methods',
               'released-ner-text/tests/loading.rs':'model.forward_fast(&hashes)',
               'released-ner-math/Cargo.toml':'math manifest','released-ner-text/Cargo.toml':'text manifest',
               '.oracle/tensors.f32':'must not copy','.pcre2/source.c':'must not copy',
               'released-ner-text/target/old-binary':'must not copy'}
        for name,text in files.items():
            f=root/name;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(text.encode("utf-8"))
        hashes={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in files if name.startswith('baseline/')}
        (root/'optimization-provenance.json').write_text(json.dumps({'baseline_files':hashes}))
        return root
    def test_fresh_exact_sources_without_generated_data(self):
        root=self.fixture()
        with patch.object(p,'HERE',root):p.prepare()
        out=root/'.baseline'
        self.assertEqual((out/'released-ner-math/src/lib.rs').read_bytes(),b'original math'+p.LOADER_API_ADAPTER)
        self.assertEqual((root/'baseline/math-lib.rs').read_bytes(),b'original math')
        self.assertEqual((out/'released-ner-text/src/bin/raw-ner.rs').read_text(),'original raw')
        self.assertFalse((out/'.oracle').exists());self.assertFalse((out/'released-ner-text/target').exists())
        self.assertFalse((out/'.pcre2').exists())
        self.assertEqual((out/'released-ner-text/src/lib.rs').read_text(),'pub mod tokenizer;\n')
        self.assertFalse((out/'released-ner-text/tests/bounded.rs').exists())
        self.assertEqual((out/'released-ner-text/tests/loading.rs').read_text(),'model.forward(&hashes)')
    def test_fixture_source_bytes_under_windows_text_io(self):
        # Simulate Windows newline translation on any host. The fixture must
        # represent the LF checkout required by CI, not host-native text output.
        def windows_write_text(path,data,encoding=None,errors=None,newline=None):
            with path.open('w',encoding=encoding or 'utf-8',errors=errors,newline='\r\n') as out:
                return out.write(data)
        with patch.object(Path,'write_text',windows_write_text):root=self.fixture()
        self.assertEqual((root/'released-ner-text/src/lib.rs').read_bytes(),
                         b'pub mod inference;\npub mod tokenizer;\n')
        with patch.object(p,'HERE',root):p.prepare()
        self.assertEqual((root/'.baseline/released-ner-text/src/lib.rs').read_bytes(),
                         b'pub mod tokenizer;\n')
    def test_changed_baseline_fails_before_copy(self):
        root=self.fixture();(root/'baseline/math-lib.rs').write_text('changed')
        with patch.object(p,'HERE',root):
            with self.assertRaises(ValueError):p.prepare()
        self.assertFalse((root/'.baseline').exists())
        self.assertFalse(list(root.glob('ner-baseline-*')))
    def test_existing_tree_not_overwritten(self):
        root=self.fixture();(root/'.baseline').mkdir();(root/'.baseline/sentinel').write_text('keep')
        with patch.object(p,'HERE',root):
            with self.assertRaises(ValueError):p.prepare()
        self.assertEqual((root/'.baseline/sentinel').read_text(),'keep')
if __name__=='__main__':unittest.main()
