"""Structural contract tests only; never substitute for actual Windows acceptance."""
import importlib.util
import json
import contextlib
import io
import subprocess
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch
import zipfile


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path); module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

p=load('desktop_package',Path(__file__).with_name('package_desktop.py'))
fixture=load('cli_fixture',Path(__file__).parents[1]/'windows-preview/test_package_preview.py')

class DesktopPackageTests(unittest.TestCase):
    def setUp(self):
        temp=tempfile.TemporaryDirectory(prefix='desktop חבילה ');self.addCleanup(temp.cleanup)
        self.root=Path(temp.name);self.pkg=self.root/'package';self.pkg.mkdir()
        self.binary=self.pkg/p.EXE;self.binary.write_bytes(fixture.example_pe())
        (self.pkg/'SHA256SUMS.txt').write_text('fixture manifest')
        self.report=self.root/'report.json'
        self.data={'medium_integrity_verified':True,'status':'passed','platform':'windows-native-tauri','ipc_mocked':False,'ai_mode':'on',
                   'binary_sha256':p.digest(self.binary),'package_sha256sums_sha256':p.digest(self.pkg/'SHA256SUMS.txt'),
                   'original_ai_checks':sorted(p.REQUIRED_CHECKS),'copied_move_checks':sorted(p.COPIED_MOVE_CHECKS),'apply_mode':'on','apply_checks':sorted(p.APPLY_CHECKS),'recovery_checks':sorted(p.RECOVERY_CHECKS),'repair_checks':sorted(p.REPAIR_CHECKS),'flac_checks':sorted(p.FLAC_CHECKS),'ogg_checks':sorted(p.OGG_CHECKS),'mp4_checks':sorted(p.MP4_CHECKS),'album_checks':sorted(p.ALBUM_CHECKS),'cross_album_checks':sorted(p.CROSS_ALBUM_CHECKS)}
        self.save()
    def save(self): self.report.write_text(json.dumps(self.data),encoding='utf-8')
    def test_each_cross_album_check_is_mandatory(self):
        for missing in p.CROSS_ALBUM_CHECKS:
            self.data['cross_album_checks'] = sorted(p.CROSS_ALBUM_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'cross-volume album'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)

    def test_each_copied_move_check_is_mandatory(self):
        for missing in p.COPIED_MOVE_CHECKS:
            self.data['copied_move_checks'] = sorted(p.COPIED_MOVE_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'copied-Move'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)

    def test_each_mp4_check_is_mandatory(self):
        for missing in p.MP4_CHECKS:
            self.data['mp4_checks'] = sorted(p.MP4_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'MP4'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)

    def test_each_ogg_check_is_mandatory(self):
        for missing in p.OGG_CHECKS:
            self.data['ogg_checks'] = sorted(p.OGG_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'Ogg'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)

    def test_each_flac_check_is_mandatory(self):
        for missing in p.FLAC_CHECKS:
            self.data['flac_checks'] = sorted(p.FLAC_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'FLAC'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)

    def test_each_album_check_is_mandatory(self):
        for missing in p.ALBUM_CHECKS:
            self.data['album_checks'] = sorted(p.ALBUM_CHECKS - {missing}); self.save()
            with self.assertRaisesRegex(ValueError, 'album'):
                p.report_gate(self.report, self.binary, 'on', self.root)

    def test_exact_acceptance(self): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_repair_proof_is_mandatory_and_complete(self):
        for missing in p.REPAIR_CHECKS:
            self.data['repair_checks'] = sorted(p.REPAIR_CHECKS - {missing});self.save()
            with self.assertRaisesRegex(ValueError, 'actual FixNames'):
                p.report_gate(self.report, self.binary, 'on', self.pkg)
    def test_off_acceptance(self):
        self.data['ai_mode']='off';self.save();p.report_gate(self.report,self.binary,'off')
    def test_bad_reports(self):
        for key,value in [('medium_integrity_verified',False),('status','failed'),('platform','mock'),('ipc_mocked',True),('ai_mode','off'),('binary_sha256','0'*64),('package_sha256sums_sha256','0'*64)]:
            with self.subTest(key=key):
                old=self.data[key];self.data[key]=value;self.save()
                with self.assertRaises(ValueError): p.report_gate(self.report,self.binary,'on',self.pkg)
                self.data[key]=old
    def test_every_ai_case_required(self):
        for missing in p.REQUIRED_CHECKS:
            self.data['original_ai_checks']=sorted(p.REQUIRED_CHECKS-{missing});self.save()
            with self.assertRaisesRegex(ValueError,'incomplete'): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_every_actual_apply_case_required(self):
        for missing in p.APPLY_CHECKS:
            self.data['apply_checks']=sorted(p.APPLY_CHECKS-{missing});self.save()
            with self.assertRaisesRegex(ValueError,'actual selected Copy/Move'): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_read_only_binary_report_cannot_publish_selected_package(self):
        self.data['apply_mode']='off';self.save()
        with self.assertRaisesRegex(ValueError,'actual selected Copy/Move'): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_every_restart_recovery_case_required(self):
        for missing in p.RECOVERY_CHECKS:
            self.data['recovery_checks']=sorted(p.RECOVERY_CHECKS-{missing});self.save()
            with self.assertRaisesRegex(ValueError,'restart recovery'): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_changed_package_rejected(self):
        (self.pkg/'SHA256SUMS.txt').write_text('different')
        with self.assertRaisesRegex(ValueError,'exact package'): p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_new_windows_ui_import(self):
        self.binary.write_bytes(fixture.example_pe(dll=b'DWMAPI.dll'))
        self.assertEqual(p.cli.pe_imports(self.binary)[0]['dll'],'dwmapi.dll')
    def test_no_python_or_webview_dll_copy_fallback(self):
        for dll in (b'python312.dll',b'WebView2Loader.dll',b'arbitrary.dll'):
            self.binary.write_bytes(fixture.example_pe(dll=dll))
            with self.assertRaisesRegex(ValueError,'unreviewed'): p.cli.pe_imports(self.binary)
    def test_cli_dependency_allowlist_unchanged(self):
        self.assertNotIn('dwmapi.dll',fixture.p.SYSTEM_DLLS)
    def test_archive_gate_and_hashes(self):
        files={p.EXE:p.digest(self.binary)}
        output=self.root/'owned.zip'; args=types.SimpleNamespace(package=self.pkg,report=self.report,output=output)
        with patch.object(p,'validate_shape',return_value=files): p.archive(args)
        with zipfile.ZipFile(output) as z:
            self.assertEqual(set(z.namelist()),{p.EXE,'SHA256SUMS.txt','examples/target/'})
            self.assertEqual(z.read(p.EXE),self.binary.read_bytes())
        self.assertTrue(output.with_suffix('.zip.sha256').read_text().startswith(p.digest(output)))
        with patch.object(p,'validate_shape',return_value=files), self.assertRaisesRegex(ValueError,'unsafe'):p.archive(args)
    def test_archive_refuses_failed_acceptance(self):
        self.data['status']='failed';self.save()
        args=types.SimpleNamespace(package=self.pkg,report=self.report,output=self.root/'never.zip')
        with patch.object(p,'validate_shape',return_value={}):
            with self.assertRaises(ValueError):p.archive(args)
        self.assertFalse(args.output.exists())
    def test_unsafe_archive_inside_package(self):
        args=types.SimpleNamespace(package=self.pkg,report=self.report,output=self.pkg/'bad.zip')
        with patch.object(p,'validate_shape',return_value={}), self.assertRaisesRegex(ValueError,'unsafe'):
            p.archive(args)
    def test_unchanged_cli_model_pins(self):
        self.assertEqual(p.cli.ASSETS,fixture.p.ASSETS)
        self.assertEqual(len(p.cli.ASSETS),5)
    def test_missing_medium_proof(self):
        del self.data['medium_integrity_verified'];self.save()
        with self.assertRaisesRegex(ValueError,'Medium'):p.report_gate(self.report,self.binary,'on',self.pkg)
    def test_publication_recreates_nested_files_with_default_inheritance(self):
        nested = self.pkg / 'licenses' / 'nested'
        nested.mkdir(parents=True)
        (nested / 'notice.txt').write_bytes(b'public notice')
        (self.pkg / 'empty').mkdir()
        output = self.root / 'published'
        original_identity = (nested / 'notice.txt').stat().st_ino
        with patch.object(p, 'validate_shape', return_value={'fixed': 'hash'}) as validate:
            p.publish_inherited_copy(self.pkg, output)
        self.assertEqual(validate.call_count, 2)
        self.assertEqual((output / 'licenses/nested/notice.txt').read_bytes(), b'public notice')
        self.assertNotEqual((output / 'licenses/nested/notice.txt').stat().st_ino, original_identity)
        self.assertTrue((output / 'empty').is_dir())
        self.assertTrue(self.pkg.exists())
        self.assertEqual(list(self.root.glob('desktop-preview-publish-*')), [])
        with self.assertRaisesRegex(ValueError, 'overwrite'):
            p.publish_inherited_copy(self.pkg, output)

    def test_publication_validation_failure_is_atomic(self):
        output = self.root / 'never'
        with patch.object(p, 'validate_shape', side_effect=[{'a': 'old'}, {'a': 'new'}]):
            with self.assertRaisesRegex(ValueError, 'bytes changed'):
                p.publish_inherited_copy(self.pkg, output)
        self.assertFalse(output.exists())
        self.assertTrue(self.pkg.exists())
        self.assertEqual(list(self.root.glob('desktop-preview-publish-*')), [])

    def test_publication_copy_failure_cleans_owned_partial_only(self):
        output = self.root / 'never'
        sentinel = self.root / 'unrelated'
        sentinel.write_bytes(b'unchanged')
        with patch.object(p, 'validate_shape', return_value={'fixed': 'hash'}), \
             patch.object(p.shutil, 'copyfile', side_effect=OSError('copy failed')):
            with self.assertRaisesRegex(OSError, 'copy failed'):
                p.publish_inherited_copy(self.pkg, output)
        self.assertFalse(output.exists())
        self.assertEqual(sentinel.read_bytes(), b'unchanged')
        self.assertTrue(self.pkg.exists())
        self.assertEqual(list(self.root.glob('desktop-preview-publish-*')), [])

    def test_acl_failure_preserves_diagnostics_before_raising(self):
        result = subprocess.CompletedProcess(['powershell.exe'], 1, b'owned stdout', b'exact owned failure')
        output, error = io.StringIO(), io.StringIO()
        with patch.object(p.subprocess, 'run', return_value=result) as run, \
             contextlib.redirect_stdout(output), contextlib.redirect_stderr(error):
            with self.assertRaises(subprocess.CalledProcessError):
                p.read_windows_acl_evidence([self.pkg])
        self.assertIn('owned stdout', output.getvalue())
        self.assertIn('exact owned failure', error.getvalue())
        self.assertFalse(run.call_args.kwargs['check'])
        self.assertEqual(run.call_args.kwargs['timeout'], 30)

    def test_acl_child_rebuilds_only_its_standard_module_path(self):
        original = {'PSModulePath': 'incompatible ps7 modules', 'KEEP_OWNED': 'value'}
        result = subprocess.CompletedProcess(['powershell.exe'], 0, b'[]', b'')
        with patch.dict(p.os.environ, original, clear=True), \
             patch.object(p.subprocess, 'run', return_value=result) as run, \
             contextlib.redirect_stdout(io.StringIO()):
            # Windows os.environ normalizes keys to uppercase at insertion.
            before = dict(p.os.environ)
            p.read_windows_acl_evidence([self.pkg])
            self.assertEqual(dict(p.os.environ), before)
        child = run.call_args.kwargs['env']
        self.assertFalse(any(key.upper() == 'PSMODULEPATH' for key in child))
        self.assertEqual(child['KEEP_OWNED'], 'value')
        self.assertEqual(json.loads(child['SINGLES_STAGING_ACL_PATHS']), [str(self.pkg)])

    def test_linux_cannot_build_windows_claim(self):
        with patch.object(p.sys,'platform','linux'),self.assertRaisesRegex(ValueError,'actual Windows'):
            p.build(types.SimpleNamespace())

if __name__=='__main__':unittest.main()
