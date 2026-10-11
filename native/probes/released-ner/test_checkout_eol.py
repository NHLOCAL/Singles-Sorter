"""Reproduce text=auto checkout conversion without modifying any original asset."""
import subprocess,tempfile,unittest
from pathlib import Path
class Checkout(unittest.TestCase):
    def check_checkout(self,eol):
        with tempfile.TemporaryDirectory(prefix='ner-eol-test-') as temp:
            root=Path(temp);repo=root/'repo';repo.mkdir();out=root/'out';out.mkdir()
            subprocess.run(['git','init','--quiet',str(repo)],check=True)
            (repo/'.gitattributes').write_bytes(b'* text=auto\n')
            text=b'[owned]\nname = synthetic\n';binary=b'\x00owned\r\n\xff'
            (repo/'owned.cfg').write_bytes(text);(repo/'owned.bin').write_bytes(binary)
            subprocess.run(['git','-C',str(repo),'-c','core.autocrlf=false','-c','core.eol=lf','add','--','.gitattributes','owned.cfg','owned.bin'],check=True)
            subprocess.run(['git','-C',str(repo),'-c','core.autocrlf=false','-c','core.eol='+eol,'checkout-index','--prefix='+str(out.resolve()).replace('\\','/')+'/','--','owned.cfg','owned.bin'],check=True)
            self.assertEqual((out/'owned.bin').read_bytes(),binary)
            return text,(out/'owned.cfg').read_bytes()
    def test_autocrlf_false_does_not_override_crlf_text_auto(self):
        original,checked=self.check_checkout('crlf')
        self.assertNotEqual(original,checked);self.assertEqual(checked,original.replace(b'\n',b'\r\n'))
    def test_explicit_lf_preserves_original_text_and_binary(self):
        original,checked=self.check_checkout('lf');self.assertEqual(original,checked)
if __name__=='__main__':unittest.main()
