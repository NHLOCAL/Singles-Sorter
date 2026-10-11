"""No Windows or original weights required for structural failure-path tests."""
import importlib.util
import json
import types
import zipfile
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('package_preview', Path(__file__).with_name('package_preview.py'))
p = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(p)


def example_pe(dll=b'KERNEL32.dll', symbol=b'GetLastError'):
    data = bytearray(1024)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 0x3c, 0x80)
    data[0x80:0x84] = b'PE\0\0'
    struct.pack_into('<HHIIIHH', data, 0x84, 0x8664, 1, 0, 0, 0, 240, 0x22)
    optional = 0x98
    struct.pack_into('<H', data, optional, 0x20b)
    struct.pack_into('<I', data, optional + 108, 16)
    struct.pack_into('<II', data, optional + 120, 0x1000, 40)
    section = optional + 240
    struct.pack_into('<IIII', data, section + 8, 512, 0x1000, 512, 512)
    struct.pack_into('<IIIII', data, 512, 0x1060, 0, 0, 0x1040, 0x1060)
    data[576:576 + len(dll) + 1] = dll + b'\0'
    struct.pack_into('<QQ', data, 608, 0x1080, 0)
    data[642:642 + len(symbol) + 1] = symbol + b'\0'
    return data


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='preview בדיקה ')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def binary(self, data=None):
        binary = self.root / 'example.exe'
        binary.write_bytes(example_pe() if data is None else data)
        return binary

    def manifest(self, extra=''):
        (self.root / 'one.txt').write_text('owned content', encoding='utf-8')
        (self.root / 'SHA256SUMS.txt').write_text(p.digest(self.root / 'one.txt') + '  one.txt\n' + extra, encoding='utf-8')

    def test_pe_exact_imports(self):
        self.assertEqual(p.pe_imports(self.binary()), [{'dll': 'kernel32.dll', 'symbols': [{'name': 'GetLastError'}]}])

    def test_dumpbin_matches_actual_dlls(self):
        imports = p.pe_imports(self.binary())
        p.crosscheck_dumpbin(imports, 'Dependencies:\n    KERNEL32.dll\n')
        with self.assertRaisesRegex(ValueError, 'disagree'):
            p.crosscheck_dumpbin(imports, '    KERNEL32.dll\n    python312.dll\n')

    def test_wrong_architecture(self):
        value = example_pe(); struct.pack_into('<H', value, 0x84, 0x14c)
        with self.assertRaisesRegex(ValueError, 'x64'): p.pe_imports(self.binary(value))

    def test_truncated_pe(self):
        with self.assertRaises(ValueError): p.pe_imports(self.binary(b'MZ'))

    def test_non_pe(self):
        with self.assertRaises(ValueError): p.pe_imports(self.binary(b'\x7fELF' + b'\0' * 1000))

    def test_delayed_imports_rejected(self):
        value = example_pe(); struct.pack_into('<II', value, 0x98 + 112 + 13 * 8, 0x1100, 32)
        with self.assertRaisesRegex(ValueError, 'delayed'): p.pe_imports(self.binary(value))

    def test_unexpected_dll(self):
        for name in (b'python312.dll', b'pcre2-32.dll', b'tag.dll', b'evil.dll'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'unreviewed'):
                p.pe_imports(self.binary(example_pe(dll=name)))

    def test_ordinals_recorded(self):
        value = example_pe(); struct.pack_into('<Q', value, 608, 0x8000000000000012)
        self.assertEqual(p.pe_imports(self.binary(value))[0]['symbols'], [{'ordinal': 18}])

    def test_system_crt_dependency_allowed_and_recorded(self):
        self.assertEqual(p.pe_imports(self.binary(example_pe(dll=b'VCRUNTIME140.dll')))[0]['dll'], 'vcruntime140.dll')

    def test_manifest_success_and_corruption(self):
        self.manifest(); self.assertEqual(set(p.verify_package(self.root)), {'one.txt'})
        (self.root / 'one.txt').write_text('changed')
        with self.assertRaisesRegex(ValueError, 'mismatch'): p.verify_package(self.root)

    def test_manifest_rejects_extra(self):
        self.manifest(); (self.root / 'original-release.pkl').write_bytes(b'never ship')
        with self.assertRaisesRegex(ValueError, 'extra'): p.verify_package(self.root)

    def test_nested_manifest_not_ignored(self):
        self.manifest(); (self.root / 'nested').mkdir(); (self.root / 'nested/SHA256SUMS.txt').write_text('not excluded')
        with self.assertRaisesRegex(ValueError, 'extra'): p.verify_package(self.root)

    def test_manifest_rejects_traversal_and_duplicate(self):
        for name in ('../secret', '/tmp/secret', 'one.txt', 'ONE.TXT', 'C:/secret', 'dir\\secret'):
            with self.subTest(name=name):
                self.manifest('0' * 64 + '  ' + name + '\n')
                with self.assertRaisesRegex(ValueError, 'unsafe'): p.verify_package(self.root)

    def test_manifest_rejects_symlink(self):
        self.manifest()
        try: (self.root / 'link').symlink_to(self.root / 'one.txt')
        except OSError: self.skipTest('symlink privilege unavailable')
        with self.assertRaisesRegex(ValueError, 'symlink'): p.verify_package(self.root)

    def windows_manifest(self):
        source = p.ROOT / 'native/probes/released-ner/released-ner-export'
        lock = json.loads((source / 'release-lock.json').read_text())
        windows = json.loads((source / 'windows-runtime-sources.json').read_text())
        manifest = {'schema': 'released-ner-v1', 'release': p.BASELINE,
                    'model_sha256': lock['source_files']['ner/model'], 'dtype': '<f4', 'order': 'C',
                    'tensor_file': 'tensors.f32', 'graph': lock['graph'],
                    'actions': ['U-', 'O'] + [f'{a}-{label}' for label in ['SINGER', 'SONG', 'MISC', 'ALBUM', 'GENRE'] for a in ['B', 'I', 'L', 'U']],
                    'unseen_classes': [], 'features': ['NORM', 'PREFIX', 'SUFFIX', 'SHAPE'],
                    'tensor_sha256': p.ASSETS['tensors.f32'], 'parameter_bytes': 3829976,
                    'files': {'tensors.f32': p.ASSETS['tensors.f32'], 'tokenizer.json': p.ASSETS['tokenizer.json'],
                              'fixtures.json': '0' * 64, 'forced_states.json': '1' * 64}, 'tensors': [],
                    'runtime': {'python': '3.12.10', 'spacy': '3.7.5', 'thinc': '8.2.5', 'numpy': '1.26.4',
                                'unicode': '15.0.0', 'source_platform': 'win32', 'source_byte_provenance': {}}}
        offset = 0
        for param in lock['parameters']:
            tensor = {key: param[key] for key in ('node', 'name', 'param', 'shape', 'sha256')}
            tensor.update(dtype='<f4', offset_bytes=offset, nbytes=param['bytes'])
            manifest['tensors'].append(tensor); offset += param['bytes']
        for name, item in windows['files'].items():
            manifest['runtime']['source_byte_provenance'][name] = {
                'raw_sha256': item['windows_sha256'], 'original_reference_sha256': item['original_sha256'],
                'verification': 'exact-official-windows-wheel-raw-and-original-lf'}
        return manifest

    def test_windows_manifest_accepts_exact_semantics_not_linux_digest(self):
        manifest = self.windows_manifest(); path = self.root / 'manifest.json'
        path.write_text(json.dumps(manifest, indent=2)); first = p.digest(path)
        p.check_windows_manifest(path)
        path.write_text(json.dumps(manifest, separators=(',', ':')))
        self.assertNotEqual(first, p.digest(path)); p.check_windows_manifest(path)

    def test_manifest_rejects_duplicate_keys(self):
        manifest = self.windows_manifest(); path = self.root / 'manifest.json'
        text = json.dumps(manifest)
        path.write_text('{"release":"unreviewed duplicate",' + text[1:])
        with self.assertRaisesRegex(ValueError, 'duplicate'): p.check_windows_manifest(path)

    def test_manifest_rejects_unapproved_model_runtime_or_layout(self):
        for mutation in ('platform', 'runtime_version', 'source_hash', 'layout', 'graph', 'model', 'files', 'fields', 'bool_int', 'float_int'):
            with self.subTest(mutation=mutation):
                manifest = self.windows_manifest()
                if mutation == 'bool_int': manifest['graph'][0]['layers'][0] = True
                if mutation == 'float_int': manifest['tensors'][0]['offset_bytes'] = 0.0
                if mutation == 'platform': manifest['runtime']['source_platform'] = 'linux'
                if mutation == 'runtime_version': manifest['runtime']['python'] = '3.13.0'
                if mutation == 'source_hash': next(iter(manifest['runtime']['source_byte_provenance'].values()))['raw_sha256'] = '0' * 64
                if mutation == 'layout': manifest['tensors'][0]['offset_bytes'] = 4
                if mutation == 'graph': manifest['graph'][0]['name'] = 'candidate'
                if mutation == 'model': manifest['model_sha256'] = '0' * 64
                if mutation == 'files': manifest['files']['tokenizer.json'] = '0' * 64
                if mutation == 'fields': manifest['private_data'] = 'forbidden'
                path = self.root / 'manifest.json'; path.write_text(json.dumps(manifest))
                with self.assertRaises(ValueError): p.check_windows_manifest(path)

    def test_wrong_model_hash(self):
        with patch.object(p, 'ASSETS', {'model.bin': '0' * 64}):
            (self.root / 'model.bin').write_bytes(b'wrong')
            with self.assertRaisesRegex(ValueError, 'mismatch'): p.check_assets(self.root)

    def test_archive_smoke_binding_and_entries(self):
        stage = self.root / 'package'; stage.mkdir()
        (stage / 'singles-sorter.exe').write_bytes(example_pe())
        models = stage / 'models/original'; models.mkdir(parents=True)
        (models / 'model.bin').write_bytes(b'owned test weights, not a model')
        (stage / 'examples/target').mkdir(parents=True)
        files = sorted(f for f in stage.rglob('*') if f.is_file())
        (stage / 'SHA256SUMS.txt').write_text(''.join(p.digest(f) + '  ' + f.relative_to(stage).as_posix() + '\n' for f in files))
        report = self.root / 'smoke.json'
        receipt = {'result': 'PASS', 'executable_sha256': p.digest(stage / 'singles-sorter.exe'),
                   'sha256sums_sha256': '0' * 64}
        report.write_text(json.dumps(receipt))
        args = types.SimpleNamespace(package=stage, smoke_report=report, output=self.root / 'synthetic-unit-test.zip')
        with patch.object(p, 'ASSETS', {'model.bin': p.digest(models / 'model.bin')}):
            with self.assertRaisesRegex(ValueError, 'smoke bound'): p.archive(args)
            self.assertFalse(args.output.exists())
            receipt['sha256sums_sha256'] = p.digest(stage / 'SHA256SUMS.txt')
            report.write_text(json.dumps(receipt)); p.archive(args)
        with zipfile.ZipFile(args.output) as archive:
            self.assertEqual(set(archive.namelist()), {'singles-sorter.exe', 'models/original/model.bin',
                                                     'SHA256SUMS.txt', 'examples/target/'})
        self.assertTrue(args.output.with_suffix('.zip.sha256').is_file())

    def test_gate_bound_to_executable(self):
        binary = self.binary()
        report = self.root / 'report.json'
        data = {'result': 'PASS', 'cli_invocations': 44, 'cli_sha256': p.digest(binary), 'baseline': p.BASELINE}
        report.write_text(json.dumps(data)); p.gate_report(report, binary)
        data['cli_sha256'] = '0' * 64; report.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'bound'): p.gate_report(report, binary)

    def test_gate_rejects_insufficient_cases(self):
        binary = self.binary(); report = self.root / 'report.json'
        report.write_text(json.dumps({'result': 'PASS', 'cli_invocations': 1}))
        with self.assertRaisesRegex(ValueError, 'full'): p.gate_report(report, binary)


if __name__ == '__main__':
    unittest.main()
