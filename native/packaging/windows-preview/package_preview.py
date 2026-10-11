#!/usr/bin/env python3
"""Build-time only, fail-closed Windows ORIGINAL-AI test-artifact packager.

Never packages a directory wholesale. No download, Python or network at app time.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import wave
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BASELINE = '78d001aee5ce49628bdffebd34540fa83ba32c12'
ASSETS = {
 'manifest.json': None,  # Exporter platform/runtime provenance is validated semantically below.
 'tensors.f32': '84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745',
 'tokenizer.json': '33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574',
 'classifier-v14.weights.bin': 'aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33',
 'classifier-v14.vocab.txt': '35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3',
}
SYSTEM_DLLS = {'kernel32.dll', 'ntdll.dll', 'advapi32.dll', 'bcrypt.dll', 'userenv.dll',
 'ws2_32.dll', 'dbghelp.dll', 'ole32.dll', 'oleaut32.dll', 'shell32.dll', 'user32.dll',
 'bcryptprimitives.dll', 'secur32.dll', 'crypt32.dll', 'rpcrt4.dll', 'version.dll', 'winmm.dll', 'shlwapi.dll',
 'vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'ucrtbase.dll'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    require(path.is_file() and not path.is_symlink(), f'not an ordinary file: {path}')
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pe_imports(path):
    """Read actual AMD64 PE32+ DLL imports AND symbols, without a build-tool PATH.

    Unsupported delayed imports are rejected rather than omitted from evidence.
    Every access is bounded; this is inspection of our build output, not a loader.
    """
    data = path.read_bytes()
    def unpack(fmt, off):
        require(0 <= off <= len(data) - struct.calcsize(fmt), 'truncated PE structure')
        return struct.unpack_from(fmt, data, off)
    require(data[:2] == b'MZ', 'expected Windows MZ executable')
    pe, = unpack('<I', 0x3c)
    require(data[pe:pe + 4] == b'PE\0\0', 'bad PE signature')
    machine, sections, _, _, _, optional_size, flags = unpack('<HHIIIHH', pe + 4)
    require(machine == 0x8664 and not flags & 0x2000, 'expected x64 executable, not DLL')
    optional = pe + 24
    magic, = unpack('<H', optional)
    require(magic == 0x20b and optional_size >= 240, 'expected PE32+ optional header')
    count, = unpack('<I', optional + 108)
    require(count >= 14, 'missing PE data directories')
    directory = optional + 112
    imports_rva, imports_size = unpack('<II', directory + 8)
    delay_rva, delay_size = unpack('<II', directory + 13 * 8)
    require(not delay_rva and not delay_size, 'delayed imports require explicit inspection support')
    require(imports_rva and imports_size, 'PE has no import directory')
    mappings = []
    for i in range(sections):
        off = optional + optional_size + i * 40
        virtual_size, rva, raw_size, raw_off = unpack('<IIII', off + 8)
        mappings.append((rva, raw_size, raw_off))
    def offset(rva, size=1):
        for start, length, raw in mappings:
            if start <= rva and rva + size <= start + length:
                value = raw + rva - start
                require(value + size <= len(data), 'PE RVA outside file')
                return value
        raise ValueError('unmapped PE RVA')
    def string(rva):
        off = offset(rva)
        end = data.find(b'\0', off, min(off + 4096, len(data)))
        require(end >= off, 'unterminated PE name')
        offset(rva, end - off + 1)
        return data[off:end].decode('ascii')
    result = []
    terminated = False
    for i in range(min(imports_size // 20, 4096)):
        original, stamp, forwarder, name_rva, first = unpack('<IIIII', offset(imports_rva + i * 20, 20))
        if not any((original, stamp, forwarder, name_rva, first)):
            terminated = True
            break
        name = string(name_rva).lower()
        require(name in SYSTEM_DLLS or re.fullmatch(r'api-ms-win-(?:crt|core)-[a-z0-9-]+\.dll', name),
                f'unreviewed DLL dependency: {name}')
        symbols = []
        thunk = original or first
        for j in range(65536):
            value, = unpack('<Q', offset(thunk + j * 8, 8))
            if value == 0:
                break
            symbols.append({'ordinal': value & 0xffff} if value >> 63 else {'name': string(value + 2)})
        else:
            raise ValueError('unterminated PE thunk table')
        require(symbols, 'empty import thunk table')
        result.append({'dll': name, 'symbols': symbols})
    require(terminated and result, 'unterminated/empty PE import directory')
    return result


def crosscheck_dumpbin(imports, report):
    names = {name.lower() for name in re.findall(r'^\s+([A-Za-z0-9_.-]+\.dll)\s*$', report, re.MULTILINE | re.IGNORECASE)}
    require(names == {entry['dll'] for entry in imports}, 'PE DLL imports disagree with official MSVC dumpbin /dependents')


def check_windows_manifest(path):
    # Only exporter provenance varies by platform. Never loosen the frozen model,
    # graph, tensor layout or exact reviewed Windows wheel source-byte contract.
    digest(path)  # Reject absent, nonregular and symlink inputs first.
    require(path.stat().st_size <= 262144, 'oversized original manifest')
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate manifest JSON key')
            result[key] = value
        return result
    manifest = json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=unique_object)
    source = ROOT / 'native/probes/released-ner/released-ner-export'
    lock = json.loads((source / 'release-lock.json').read_text(encoding='utf-8'))
    windows = json.loads((source / 'windows-runtime-sources.json').read_text(encoding='utf-8'))
    actions = ['U-', 'O'] + [f'{action}-{label}' for label in ['SINGER', 'SONG', 'MISC', 'ALBUM', 'GENRE'] for action in ['B', 'I', 'L', 'U']]
    fixed = {'schema': 'released-ner-v1', 'release': BASELINE,
             'model_sha256': lock['source_files']['ner/model'], 'dtype': '<f4', 'order': 'C',
             'tensor_file': 'tensors.f32', 'graph': lock['graph'], 'actions': actions,
             'unseen_classes': [], 'features': ['NORM', 'PREFIX', 'SUFFIX', 'SHAPE'],
             'tensor_sha256': ASSETS['tensors.f32'], 'parameter_bytes': 3829976}
    require(set(manifest) == {*fixed, 'tensors', 'runtime', 'files'}, 'unreviewed manifest fields')
    def exact(actual, expected):
        return json.dumps(actual, sort_keys=True, separators=(',', ':'), allow_nan=False) == json.dumps(expected, sort_keys=True, separators=(',', ':'), allow_nan=False)
    for key, expected in fixed.items():
        require(exact(manifest[key], expected), f'original manifest mismatch: {key}')
    require(len(manifest['tensors']) == len(lock['parameters']), 'wrong manifest tensor count')
    offset = 0
    for tensor, param in zip(manifest['tensors'], lock['parameters']):
        expected = {key: param[key] for key in ('node', 'name', 'param', 'shape', 'sha256')}
        expected.update(dtype='<f4', offset_bytes=offset, nbytes=param['bytes'])
        require(exact(tensor, expected), 'original manifest tensor layout changed')
        offset += param['bytes']
    require(offset == 3829976, 'wrong manifest tensor extent')
    runtime = {'python': '3.12.10', 'spacy': '3.7.5', 'thinc': '8.2.5',
               'numpy': '1.26.4', 'unicode': '15.0.0', 'source_platform': 'win32',
               'source_byte_provenance': {}}
    require(set(windows['files']) == set(lock['runtime_sources']), 'Windows runtime source lock mismatch')
    for name, reference in lock['runtime_sources'].items():
        item = windows['files'][name]
        require(item['original_sha256'] == reference, 'Windows source reference mismatch')
        runtime['source_byte_provenance'][name] = {
            'raw_sha256': item['windows_sha256'], 'original_reference_sha256': reference,
            'verification': 'exact-official-windows-wheel-raw-and-original-lf'}
    require(exact(manifest['runtime'], runtime), 'manifest lacks exact approved Windows exporter provenance')
    require(set(manifest['files']) == {'tensors.f32', 'tokenizer.json', 'fixtures.json', 'forced_states.json'},
            'unreviewed exporter file inventory')
    for name, hash_ in manifest['files'].items():
        require(isinstance(hash_, str) and re.fullmatch(r'[0-9a-f]{64}', hash_), 'invalid export file digest')
        if name in ASSETS:
            require(hash_ == ASSETS[name], f'manifest asset digest mismatch: {name}')


def check_assets(bundle):
    for name, expected in ASSETS.items():
        if expected is None:
            check_windows_manifest(bundle / name)
        else:
            require(digest(bundle / name) == expected, f'original asset mismatch: {name}')
    return {name: digest(bundle / name) for name in ASSETS}


def safe_name(name):
    p = PurePosixPath(name)
    return (not p.is_absolute() and '\\' not in name and ':' not in name and
            all(v not in ('', '.', '..') for v in name.split('/')))


def verify_package(root):
    require(root.is_dir() and not root.is_symlink(), 'package root must be ordinary directory')
    rows = (root / 'SHA256SUMS.txt').read_text(encoding='utf-8').splitlines()
    expected = {}
    for row in rows:
        require(re.fullmatch(r'[0-9a-f]{64}  .+', row) is not None, 'malformed SHA256SUMS entry')
        hash_, name = row.split('  ', 1)
        require(safe_name(name) and name != 'SHA256SUMS.txt' and name.casefold() not in
                {n.casefold() for n in expected}, 'unsafe/duplicate manifest path')
        expected[name] = hash_
    actual = set()
    for path in root.rglob('*'):
        require(not path.is_symlink(), 'symlinks are not distributable')
        require(path.is_dir() or path.is_file(), 'special filesystem object in package')
        if path.is_file() and path.relative_to(root).as_posix() != 'SHA256SUMS.txt':
            actual.add(path.relative_to(root).as_posix())
    require(actual == set(expected), 'package contains missing or extra files')
    require(expected, 'empty package manifest')
    for name, hash_ in expected.items():
        require(digest(root / name) == hash_, f'package hash mismatch: {name}')
    return expected


def gate_report(path, binary):
    report = json.loads(path.read_text(encoding='utf-8'))
    require(report.get('result') == 'PASS' and report.get('cli_invocations', 0) >= 44,
            'full original-AI release CLI proof is required')
    require(report.get('cli_sha256') == digest(binary) and report.get('baseline') == BASELINE,
            'release proof is not bound to this executable and original baseline')
    return report


def run_text(command):
    return subprocess.check_output(command, cwd=ROOT, text=True, encoding='utf-8').strip()


def build(args):
    require(sys.platform == 'win32' and platform.machine().upper() in ('AMD64', 'X86_64'),
            'actual Windows x64 runner required; no cross-platform placeholder package')
    binary, bundle, out = args.binary.resolve(), args.bundle.resolve(), args.output.resolve()
    require(not out.exists(), 'output must be new; refusing overwrite')
    asset_hashes = check_assets(bundle)
    gate = gate_report(args.cli_report, binary)
    imports = pe_imports(binary)
    dumpbin = args.dumpbin_report.read_text(encoding='utf-8-sig')
    crosscheck_dumpbin(imports, dumpbin)
    source_commit = run_text(['git', 'rev-parse', 'HEAD'])
    require(re.fullmatch(r'[0-9a-f]{40}', source_commit), 'invalid source commit')
    require(not run_text(['git', 'status', '--porcelain', '--untracked-files=no']), 'tracked source is dirty')
    require(re.fullmatch(r'https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/actions/runs/[0-9]+', args.ci_url),
            'verified GitHub CI run URL required')
    receipt = json.loads(args.pcre2_receipt.read_text(encoding='utf-8'))
    require(re.fullmatch(r'[0-9a-f]{64}', receipt.get('library_sha256', '')), 'missing actual PCRE2 archive hash')
    pcre2_lock = json.loads((ROOT / 'native/probes/released-ner/pcre2-lock.json').read_text(encoding='utf-8'))
    require(all(receipt.get(key) == value for key, value in pcre2_lock.items()), 'PCRE2 source receipt differs from reviewed lock')
    require(receipt.get('library_bytes', 0) > 0, 'missing PCRE2 archive byte count')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='preview-build-', dir=out.parent) as temp:
        stage = Path(temp) / 'package'
        stage.mkdir()
        shutil.copyfile(binary, stage / 'singles-sorter.exe')
        models = stage / 'models/original'
        models.mkdir(parents=True)
        for name in ASSETS:
            shutil.copyfile(bundle / name, models / name)
        for name in ('README-preview.txt', 'smoke.ps1'):
            shutil.copyfile(HERE / name, stage / name)
        examples = stage / 'examples/source'
        examples.mkdir(parents=True)
        (stage / 'examples/target').mkdir()
        # Two newly authored silent PCM waveforms. No music/private/oracle corpus.
        for name in ('ישראל כהן - שיר חדש.wav', 'אברהם פריד - בדיקה.wav'):
            with wave.open(str(examples / name), 'wb') as audio:
                audio.setnchannels(1)
                audio.setsampwidth(2)
                audio.setframerate(8000)
                audio.writeframes(b'\0\0' * 800)
        subprocess.run([sys.executable, str(HERE.parent / 'licenses/collect_licenses.py'),
                        '--repo-root', str(ROOT), '--output-dir', str(stage / 'licenses'),
                        '--target', 'x86_64-pc-windows-msvc'], check=True)
        provenance = {
            'schema_version': 1, 'kind': 'original-ai-windows-x64-read-only-cli-preview',
            'source_commit': source_commit, 'source_tree': run_text(['git', 'rev-parse', 'HEAD^{tree}']),
            'ci_url': args.ci_url, 'target': 'x86_64-pc-windows-msvc', 'features': ['original-ai'],
            'rustc': run_text(['rustc', '+1.99.0', '-vV']),
            'cargo': run_text(['cargo', '+1.99.0', '--version']),
            'native_build': args.build_info.read_text(encoding='utf-8-sig'),
            'executable_sha256': digest(binary), 'pe_imports': imports,
            'msvc_dumpbin_dependents': dumpbin,
            'crt_linkage': 'dynamic MSVC; official Microsoft Visual C++ x64 Redistributable prerequisite',
            'clean_machine_without_prerequisite_tested': False,
            'offline_runtime': True, 'physical_apply': False,
            'baseline_commit': BASELINE, 'bundle_id': 'original-v14-ner-3.7.5',
            'ner_model': {'name': 'singer_ner_he', 'version': '2.0.0', 'spacy_compatibility': '>=3.7.5,<3.8.0'},
            'assets_sha256': asset_hashes,
            'assets_bytes': {name: (models / name).stat().st_size for name in ASSETS},
            'size_summary': {
                'executable_bytes': binary.stat().st_size,
                'model_assets_bytes': sum((models / name).stat().st_size for name in ASSETS),
                'notices_and_source_bytes': sum(p.stat().st_size for p in (stage / 'licenses').rglob('*') if p.is_file())},
            'pcre2_build': receipt,
            'release_cli_proof': {'sha256': digest(args.cli_report), 'invocations': gate['cli_invocations']},
            'samples': 'Two authored silent WAVs; no copyrighted recording or external corpus.',
            'manifest': 'SHA256SUMS.txt covers every package file except itself.'}
        (stage / 'provenance.json').write_text(json.dumps(provenance, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        paths = sorted(p for p in stage.rglob('*') if p.is_file())
        (stage / 'SHA256SUMS.txt').write_text(''.join(f'{digest(p)}  {p.relative_to(stage).as_posix()}\n' for p in paths), encoding='utf-8')
        verify_package(stage)
        stage.rename(out)
    print(f'Prepared inspected Windows package, smoke still required: {out}')


def archive(args):
    root = args.package.resolve()
    files = verify_package(root)
    check_assets(root / 'models/original')
    require(set(p.name for p in (root / 'models/original').iterdir()) == set(ASSETS), 'extra model asset')
    pe_imports(root / 'singles-sorter.exe')
    report = json.loads(args.smoke_report.read_text(encoding='utf-8-sig'))
    require(report.get('result') == 'PASS' and
            report.get('executable_sha256') == digest(root / 'singles-sorter.exe') and
            report.get('sha256sums_sha256') == digest(root / 'SHA256SUMS.txt'),
            'successful Windows smoke bound to this exact package is required')
    require(not args.output.resolve().is_relative_to(root), 'ZIP output must be outside package')
    require(not args.output.exists(), 'ZIP output already exists')
    with zipfile.ZipFile(args.output, 'x', compression=zipfile.ZIP_DEFLATED) as output:
        output.writestr('examples/target/', b'')
        for name in sorted([*files, 'SHA256SUMS.txt']):
            output.write(root / name, name)
    with zipfile.ZipFile(args.output) as result:
        require(result.testzip() is None and set(result.namelist()) == {*files, 'SHA256SUMS.txt', 'examples/target/'}, 'ZIP verification failed')
        for name, expected in files.items():
            require(hashlib.sha256(result.read(name)).hexdigest() == expected, 'ZIP content mismatch')
    args.output.with_suffix(args.output.suffix + '.sha256').write_text(
        f'{digest(args.output)}  {args.output.name}\n', encoding='ascii')
    print(f'Verified smoke-tested Windows preview ZIP: {args.output} ({args.output.stat().st_size} bytes)')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    b = commands.add_parser('build')
    for name in ('binary', 'bundle', 'cli-report', 'pcre2-receipt', 'build-info', 'dumpbin-report', 'output'):
        b.add_argument('--' + name, type=Path, required=True)
    b.add_argument('--ci-url', required=True)
    a = commands.add_parser('archive')
    for name in ('package', 'smoke-report', 'output'):
        a.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    (build if args.command == 'build' else archive)(args)

if __name__ == '__main__':
    main()
