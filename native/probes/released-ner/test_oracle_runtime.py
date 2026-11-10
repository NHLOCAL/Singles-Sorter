import copy,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import check_oracle_golden as golden
from packaging.markers import default_environment
from check_oracle_runtime import HERE,check_versions
class Runtime(unittest.TestCase):
    def setUp(self):
        self.lock=json.loads((HERE/'oracle-lock.json').read_text());self.installed={n:p['version'] for n,p in self.lock['packages'].items()}
        self.env=default_environment();self.env.update(python_version='3.12',python_full_version='3.12.10',sys_platform='linux',platform_system='Linux',os_name='posix',extra='')
    def test_linux_and_windows_closure(self):
        self.assertEqual(check_versions(self.lock,'3.12.10','15.0.0',self.installed,self.env),42)
        self.env.update(sys_platform='win32',platform_system='Windows',os_name='nt')
        self.assertEqual(check_versions(self.lock,'3.12.10','15.0.0',self.installed,self.env),43)
    def test_missing_click_rejected(self):
        del self.installed['click']
        with self.assertRaisesRegex(ValueError,'click'):check_versions(self.lock,'3.12.10','15.0.0',self.installed,self.env)
    def test_missing_transitive_rejected(self):
        del self.installed['pydantic-core']
        with self.assertRaises(ValueError):check_versions(self.lock,'3.12.10','15.0.0',self.installed,self.env)
    def test_wrong_python_rejected(self):
        with self.assertRaisesRegex(ValueError,'Python'):check_versions(self.lock,'3.12.14','15.0.0',self.installed,self.env)
    def test_wrong_unicode_rejected(self):
        with self.assertRaisesRegex(ValueError,'Unicode'):check_versions(self.lock,'3.12.10','14.0.0',self.installed,self.env)
class Golden(unittest.TestCase):
    def setUp(self):
        d=tempfile.TemporaryDirectory();self.addCleanup(d.cleanup);self.root=Path(d.name);self.bundle=self.root/'bundle';self.bundle.mkdir()
        self.f={'id':0,'text':'','tokens':[],'features':[],'hashes':[],'buckets':[],'states':[],'spans':[]}
        (self.root/'linux-reference.json').write_text(json.dumps({'fixtures':[{'text':'','actions':[],'spans':[],'tokens':[],'lexical':[]}]}))
    def run_check(self,fixtures):
        (self.bundle/'fixtures.json').write_text(json.dumps({'fixtures':fixtures}))
        with patch.object(golden,'HERE',self.root):golden.check(self.bundle)
    def test_owned_empty_case(self):self.run_check([self.f])
    def test_action_drift_rejected(self):
        f=copy.deepcopy(self.f);f['states']=[{'action_name':'O'}]
        with self.assertRaises(ValueError):self.run_check([f])
    def test_missing_document_rejected(self):
        with self.assertRaises(ValueError):self.run_check([])
if __name__=='__main__':unittest.main()
