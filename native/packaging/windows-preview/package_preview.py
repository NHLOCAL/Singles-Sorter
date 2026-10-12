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
EXECUTION_CHECKS = {'positional-wave-id3-five-fields', 'positional-aiff-id3-five-fields', 'positional-dsf-id3-five-fields',
          'positional-container-id3-exact-extent-decoded-audio', 'positional-container-id3-insufficient-padding-partial', 'positional-cross-volume-root-album-move', 'positional-cross-volume-mixed-album-move',
          'positional-cross-album-streams-metadata', 'positional-cross-album-observed-credits',
          'positional-cross-album-dry-run-no-effects', 'positional-cross-album-fresh-reconciliation',
          'positional-m4a-five-fields', 'positional-alac-five-fields', 'positional-mp4-fast-start-values',
          'positional-mp4-exact-extent-decoded-audio', 'positional-mp4-duplicate-outer-consolidation', 'positional-mp4-structural-partial',
          'positional-cross-volume-move', 'positional-original-ai-duet-move', 'positional-mixed-duet-and-rename',
          'positional-copied-move-named-streams', 'positional-copied-move-fresh-reconciliation',
          'positional-copy', 'positional-move', 'positional-original-ai-copy',
                    'positional-fix-id3v23', 'positional-fix-id3v24', 'positional-fix-partial',
                    'positional-album-copy', 'positional-root-album-move', 'positional-mixed-album-move',
                    'positional-album-descendant-metadata', 'positional-album-dry-run-no-effects',
                    'positional-flac-five-fields', 'positional-flac-shifted-audio', 'positional-flac-opaque-partial',
                    'positional-primary-resource-order',
                    'positional-vorbis-five-fields', 'positional-opus-five-fields',
                    'positional-ogg-packets-backup-preserved', 'positional-ogg-chained-refused',
                    'legacy-utf8-file-logging', 'legacy-log-levels', 'diagnostic-overlap-no-effects',
                    'retained-original-and-media', 'closed-stdin', 'no-developer-path',
                    'explicit-dry-run-no-effects', 'overlap-rejected-no-effects', 'legacy-argument-exits'}
CLI_FEATURES = ['original-ai', 'selected-windows-execution']
CLI_NOTICE_FEATURES = ['sorter-cli/' + feature for feature in CLI_FEATURES]

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
    return check_original_manifest(path, "win32")


def read_original_manifest(path):
    require(path.is_file() and not path.is_symlink(), f'not an ordinary file: {path}')
    with path.open('rb') as stream:
        raw = stream.read(262145)
    require(len(raw) <= 262144, 'oversized original manifest')
    return raw


def check_original_manifest(path, source_platform):
    return check_original_manifest_bytes(read_original_manifest(path), source_platform)


def check_original_manifest_bytes(raw, source_platform):
    require(source_platform in ("win32", "linux"), "unsupported exporter platform")
    # Validate the same immutable bytes that a caller can hash for its runtime pin.
    require(isinstance(raw, bytes) and len(raw) <= 262144, 'oversized original manifest')
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'duplicate manifest JSON key')
            result[key] = value
        return result
    manifest = json.loads(raw.decode('utf-8'), object_pairs_hook=unique_object)
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
               'numpy': '1.26.4', 'unicode': '15.0.0', 'source_platform': source_platform,
               'source_byte_provenance': {}}
    require(set(windows['files']) == set(lock['runtime_sources']), 'Windows runtime source lock mismatch')
    for name, reference in lock['runtime_sources'].items():
        item = windows['files'][name]
        require(item['original_sha256'] == reference, 'Windows source reference mismatch')
        runtime['source_byte_provenance'][name] = {
            'raw_sha256': item['windows_sha256'] if source_platform == 'win32' else reference,
            'original_reference_sha256': reference,
            'verification': ('exact-official-windows-wheel-raw-and-original-lf'
                             if source_platform == 'win32' else 'exact-original-source-raw')}
    require(exact(manifest['runtime'], runtime), 'manifest lacks exact approved exporter provenance')
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



def verify_cli_notices(root):
    inventory = json.loads((root / 'licenses/inventory.json').read_text(encoding='utf-8'))
    require(inventory.get('profile') == 'cli' and inventory.get('features') == CLI_NOTICE_FEATURES,
            'selected CLI notice profile must match the executable features')
    rows = inventory.get('files', [])
    names = [row['path'] for row in rows]
    actual = {p.relative_to(root / 'licenses').as_posix() for p in (root / 'licenses').rglob('*')
              if p.is_file() and p.relative_to(root / 'licenses').as_posix() != 'inventory.json'}
    require(names and len(names) == len(set(names)) and set(names) == actual,
            'CLI notice inventory does not cover exact packaged files')
    require('SOURCE-AVAILABILITY.md' in actual, 'CLI source availability notice is missing')
    for row in rows:
        name = row['path']
        require(safe_name(name) and digest(root / 'licenses' / name) == row['sha256']
                and (root / 'licenses' / name).stat().st_size == row['bytes'],
                'CLI notice inventory hash or size mismatch')


def execution_gate(path, binary, package_root=None):
    report = json.loads(path.read_text(encoding='utf-8-sig'))
    require(report.get('result') == 'PASS' and report.get('schema_version') == 1,
            'successful actual CLI execution report required')
    require(EXECUTION_CHECKS <= set(report.get('checks', [])), 'incomplete actual CLI execution coverage')
    require(report.get('executable_sha256') == digest(binary), 'execution proof bound to a different executable')
    require(all(report.get(key) is True for key in ('closed_stdin', 'developer_path_removed', 'real_subprocess')),
            'real closed-stdin application process without developer PATH required')
    if package_root is not None:
        require(report.get('sha256sums_sha256') == digest(package_root / 'SHA256SUMS.txt'),
                'execution proof bound to a different package')
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
    execution_gate(args.execution_report, binary)
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
        verify_cli_notices(stage)
        provenance = {
            'schema_version': 1, 'kind': 'original-ai-windows-x64-selected-cli',
            'source_commit': source_commit, 'source_tree': run_text(['git', 'rev-parse', 'HEAD^{tree}']),
            'ci_url': args.ci_url, 'target': 'x86_64-pc-windows-msvc', 'features': CLI_FEATURES,
            'rustc': run_text(['rustc', '+1.99.0', '-vV']),
            'cargo': run_text(['cargo', '+1.99.0', '--version']),
            'native_build': args.build_info.read_text(encoding='utf-8-sig'),
            'executable_sha256': digest(binary), 'pe_imports': imports,
            'msvc_dumpbin_dependents': dumpbin,
            'crt_linkage': 'dynamic MSVC; official Microsoft Visual C++ x64 Redistributable prerequisite',
            'clean_machine_without_prerequisite_tested': False,
            'offline_runtime': True, 'physical_apply': True,
            'execution_scope': ['selected singles Copy', 'same-volume Move', 'bounded cross-volume singles and complete duet Move with named streams', 'source-only FixNames', 'bounded album Copy, same-volume whole-directory Move and quiescent NTFS cross-volume file/stream Move with exact empty-source cleanup', 'bounded native FLAC, single-stream Ogg Vorbis/Opus and exact-extent UTF-8 MP4 AAC/ALAC and existing ID3 in ordinary WAVE/AIFF/DSF tag repair'],
            'unsupported_execution': ['incomplete eligible duet Move cohorts', 'copied Move outside ordinary quiescent local NTFS', 'changed tags outside bounded MP3 ID3v2.3/v2.4, native FLAC, single-stream Ogg Vorbis/Opus and exact-extent UTF-8 MP4 AAC/ALAC, existing ID3 in ordinary WAVE/AIFF/DSF'],
            'automatic_replay': False, 'automatic_backup_cleanup': False,
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
            'execution_cli_proof': {'sha256': digest(args.execution_report), 'checks': sorted(EXECUTION_CHECKS)},
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
    verify_cli_notices(root)
    check_assets(root / 'models/original')
    require(set(p.name for p in (root / 'models/original').iterdir()) == set(ASSETS), 'extra model asset')
    pe_imports(root / 'singles-sorter.exe')
    report = json.loads(args.smoke_report.read_text(encoding='utf-8-sig'))
    require(report.get('result') == 'PASS' and
            report.get('executable_sha256') == digest(root / 'singles-sorter.exe') and
            report.get('sha256sums_sha256') == digest(root / 'SHA256SUMS.txt'),
            'successful Windows smoke bound to this exact package is required')
    execution_gate(args.execution_report, root / 'singles-sorter.exe', root)
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
    for name in ('binary', 'bundle', 'cli-report', 'execution-report', 'pcre2-receipt', 'build-info', 'dumpbin-report', 'output'):
        b.add_argument('--' + name, type=Path, required=True)
    b.add_argument('--ci-url', required=True)
    a = commands.add_parser('archive')
    for name in ('package', 'smoke-report', 'execution-report', 'output'):
        a.add_argument('--' + name, type=Path, required=True)
    check = commands.add_parser('verify-execution')
    check.add_argument('--package', type=Path, required=True)
    check.add_argument('--execution-report', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'verify-execution':
        verify_package(args.package)
        verify_cli_notices(args.package)
        execution_gate(args.execution_report, args.package / 'singles-sorter.exe', args.package)
    else:
        (build if args.command == 'build' else archive)(args)

if __name__ == '__main__':
    main()
