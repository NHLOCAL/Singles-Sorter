#!/usr/bin/env python3
"""Allowlisted, build-time Windows desktop AI preview packaging; no installer."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import wave
import uuid
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
spec = importlib.util.spec_from_file_location('cli_preview_contract', HERE.parent / 'windows-preview/package_preview.py')
cli = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cli)
# Additional Windows inbox UI DLLs, never DLLs copied into this distribution.
# Unknown dependencies (including WebView2Loader.dll) still fail closed.
cli.SYSTEM_DLLS |= {'gdi32.dll', 'dwmapi.dll', 'comctl32.dll', 'uxtheme.dll',
                   'propsys.dll', 'imm32.dll', 'comdlg32.dll', 'shcore.dll',
                   'windowscodecs.dll', 'msimg32.dll', 'powrprof.dll',
                   'combase.dll', 'cfgmgr32.dll', 'setupapi.dll', 'dwrite.dll', 'd2d1.dll'}
EXE = 'singles-sorter-preview.exe'
REQUIRED_CHECKS = {'original-models-executed', 'disabled-no-inference', 'cancel-no-stale-plan',
                   'back-no-stale-plan', 'reload-no-stale-plan', 'repeat-preview', 'missing-model-fail-closed',
                   'corrupt-model-fail-closed', 'protected-files-unchanged'}
require, digest = cli.require, cli.digest


def report_gate(path, binary, mode, package=None):
    report = json.loads(path.read_text(encoding='utf-8-sig'))
    require(report.get('status') == 'passed' and report.get('platform') == 'windows-native-tauri', 'actual Windows native acceptance required')
    require(report.get('medium_integrity_verified') is True, 'verified Medium-integrity acceptance required')
    require(report.get('ipc_mocked') is False and report.get('ai_mode') == mode, 'wrong AI mode or mocked IPC')
    require(report.get('binary_sha256') == digest(binary), 'acceptance not bound to executable')
    if mode == 'on':
        require(REQUIRED_CHECKS <= set(report.get('original_ai_checks', [])), 'incomplete original-AI acceptance')
        require(package is not None and report.get('package_sha256sums_sha256') == digest(package / 'SHA256SUMS.txt'), 'acceptance not bound to exact package')
    return report


def validate_shape(root):
    files = cli.verify_package(root)
    allowed = {EXE, 'README-preview.txt', 'provenance.json',
               'examples/source/ישראל כהן - שיר חדש.wav', 'examples/source/אברהם פריד - בדיקה.wav',
               *(f'models/original/{n}' for n in cli.ASSETS)}
    require(set(files) - {n for n in files if n.startswith('licenses/')} == allowed, 'unexpected desktop package files')
    require(set(p.name for p in (root / 'models/original').iterdir()) == set(cli.ASSETS), 'unexpected model inventory')
    actual_assets = cli.check_assets(root / 'models/original')
    require((root / 'licenses/inventory.json').is_file() and (root / 'licenses/SOURCE-AVAILABILITY.md').is_file(), 'complete license inventory required')
    inventory = json.loads((root / 'licenses/inventory.json').read_text(encoding='utf-8'))
    require(inventory.get('profile') == 'desktop' and inventory.get('features') == ['desktop', 'original-ai', 'tauri/custom-protocol'], 'desktop-specific notice closure required')
    inventory_files = inventory.get('files', [])
    names = ['licenses/' + row['path'] for row in inventory_files]
    require(len(names) == len(set(names)) and set(names) == {n for n in files if n.startswith('licenses/') and n != 'licenses/inventory.json'}, 'notice inventory does not cover exact files')
    for row in inventory_files:
        name = 'licenses/' + row['path']
        require(cli.safe_name(name) and files[name] == row['sha256'] and (root / name).stat().st_size == row['bytes'], 'notice inventory hash/size mismatch')
    provenance = json.loads((root / 'provenance.json').read_text(encoding='utf-8'))
    require(provenance['assets_sha256'] == actual_assets, 'provenance model mismatch')
    require(provenance['executable_sha256'] == digest(root / EXE), 'provenance executable mismatch')
    imports = cli.pe_imports(root / EXE)
    require(imports == provenance['pe_imports'], 'provenance PE mismatch')
    cli.crosscheck_dumpbin(imports, provenance['msvc_dumpbin_dependents'])
    return files


def read_windows_acl_evidence(paths):
    """Read-only evidence for generated staging objects; never changes access."""
    # pwsh -> Python -> powershell.exe otherwise inherits incompatible PS7 modules.
    # Microsoft documents clearing this child-only variable so Windows PowerShell
    # reconstructs its standard module paths. Parent environment/policy is unchanged.
    env = {key: value for key, value in os.environ.items() if key.upper() != 'PSMODULEPATH'}
    env['SINGLES_STAGING_ACL_PATHS'] = json.dumps([str(p) for p in paths])
    script = (
        "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=New-Object System.Text.UTF8Encoding($false); "
        "$rows=@(foreach($p in ($env:SINGLES_STAGING_ACL_PATHS|ConvertFrom-Json)){"
        "$a=Get-Acl -LiteralPath $p; "
        "[pscustomobject]@{path=$p;owner=$a.Owner;sddl=$a.Sddl;protected=$a.AreAccessRulesProtected}}); "
        "ConvertTo-Json -InputObject $rows -Compress"
    )
    result = subprocess.run(['powershell.exe', '-NoProfile', '-NonInteractive', '-Command', script],
                            env=env, check=False, capture_output=True, timeout=30)
    print('STAGING_ACCESS_EVIDENCE ' + result.stdout.decode('utf-8-sig', errors='replace').strip(), flush=True)
    if result.returncode:
        print('STAGING_ACCESS_ERROR ' + result.stderr.decode('utf-8-sig', errors='replace')[:16000],
              file=sys.stderr, flush=True)
    result.check_returncode()


def publish_inherited_copy(stage, output):
    """Publish public preview bytes with ordinary parent inheritance, never ACL edits.

    CPython 3.12 Windows TemporaryDirectory uses a protected mode-0700 DACL.
    Moving that tree preserves its ACL, including nested license staging ACLs.
    Recreate every directory/file under the owned output parent instead. copyfile
    deliberately copies bytes only; no source security descriptor is propagated.
    """
    require(not output.exists(), 'refusing output overwrite')
    expected = validate_shape(stage)
    pending = output.parent / ('desktop-preview-publish-' + uuid.uuid4().hex)
    pending.mkdir()  # default mode: normal Windows parent inheritance
    try:
        for source in sorted(stage.rglob('*')):
            dest = pending / source.relative_to(stage)
            if source.is_dir():
                dest.mkdir()
            else:
                shutil.copyfile(source, dest)
        require(validate_shape(pending) == expected, 'published package bytes changed')
        pending.rename(output)
    finally:
        if pending.exists():
            shutil.rmtree(pending)


def build(args):
    require(sys.platform == 'win32' and platform.machine().upper() in ('AMD64', 'X86_64'), 'actual Windows x64 build required')
    out = args.output.resolve()
    require(not out.exists(), 'refusing output overwrite')
    off = report_gate(args.featureoff_report, args.featureoff_binary, 'off')
    assets = cli.check_assets(args.bundle)
    imports = cli.pe_imports(args.binary)
    dumpbin = args.dumpbin_report.read_text(encoding='utf-8-sig')
    cli.crosscheck_dumpbin(imports, dumpbin)
    require(not cli.run_text(['git', 'status', '--porcelain', '--untracked-files=no']), 'tracked source dirty')
    require(re.fullmatch(r'https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/actions/runs/[0-9]+', args.ci_url), 'invalid CI URL')
    receipt = json.loads(args.pcre2_receipt.read_text(encoding='utf-8'))
    lock = json.loads((ROOT / 'native/probes/released-ner/pcre2-lock.json').read_text())
    require(all(receipt.get(k) == v for k,v in lock.items()), 'PCRE2 source mismatch')
    require(re.fullmatch('[0-9a-f]{64}', receipt.get('library_sha256', '')) and receipt.get('library_bytes',0)>0, 'PCRE2 archive identity missing')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='desktop-preview-build-', dir=out.parent) as temp:
        stage = Path(temp) / 'package'; stage.mkdir()
        shutil.copyfile(args.binary, stage / EXE)
        shutil.copyfile(HERE / 'README-preview.txt', stage / 'README-preview.txt')
        models = stage / 'models/original'; models.mkdir(parents=True)
        for name in cli.ASSETS: shutil.copyfile(args.bundle / name, models / name)
        examples = stage / 'examples/source'; examples.mkdir(parents=True)
        (stage / 'examples/target').mkdir()
        for name in ('ישראל כהן - שיר חדש.wav', 'אברהם פריד - בדיקה.wav'):
            with wave.open(str(examples / name), 'wb') as audio:
                audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(8000); audio.writeframes(b'\0\0'*800)
        subprocess.run([sys.executable, str(HERE.parent / 'licenses/collect_licenses.py'), '--profile', 'desktop',
                        '--repo-root', str(ROOT), '--output-dir', str(stage / 'licenses')], check=True)
        provenance = {
            'schema_version':1, 'kind':'original-ai-windows-x64-read-only-desktop-preview',
            'source_commit':cli.run_text(['git','rev-parse','HEAD']), 'source_tree':cli.run_text(['git','rev-parse','HEAD^{tree}']),
            'ci_url':args.ci_url, 'target':'x86_64-pc-windows-msvc', 'features':['desktop','original-ai','tauri/custom-protocol'],
            'executable_sha256':digest(stage / EXE), 'pe_imports':imports, 'msvc_dumpbin_dependents':dumpbin,
            'rustc':cli.run_text(['rustc','-vV']), 'cargo':cli.run_text(['cargo','-V']),
            'native_build':args.build_info.read_text(encoding='utf-8-sig'), 'pcre2_build':receipt,
            'prerequisites':['Microsoft Edge WebView2 Evergreen Runtime', 'Microsoft Visual C++ x64 Redistributable'],
            'crt_linkage':'dynamic MSVC; Microsoft runtime DLLs not bundled',
            'webview2_sdk_loader':'Microsoft SDK static loader; exact SDK license/notices included in licenses',
            'clean_machine_without_prerequisites_tested':False, 'physical_apply':False,
            'model_execution':'local native; no Python or model network', 'webview_network_isolation_tested':False,
            'baseline_commit':cli.BASELINE, 'bundle_id':'original-v14-ner-3.7.5',
            'assets_sha256':assets, 'assets_bytes':{n:(models/n).stat().st_size for n in cli.ASSETS},
            'featureoff_proof':{'report_sha256':digest(args.featureoff_report),'binary_sha256':off['binary_sha256']},
            'size_summary':{'executable_bytes':(stage/EXE).stat().st_size,
                            'model_assets_bytes':sum((models/n).stat().st_size for n in cli.ASSETS),
                            'notices_and_source_bytes':sum(p.stat().st_size for p in (stage/'licenses').rglob('*') if p.is_file())},
            'samples':'Two authored silent WAV files; no private input or oracle corpus.'}
        (stage/'provenance.json').write_text(json.dumps(provenance,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        files = sorted(p for p in stage.rglob('*') if p.is_file())
        (stage/'SHA256SUMS.txt').write_text(''.join(f'{digest(p)}  {p.relative_to(stage).as_posix()}\n' for p in files),encoding='utf-8')
        validate_shape(stage)
        read_windows_acl_evidence([out.parent, stage, stage / EXE, stage / "licenses"])
        publish_inherited_copy(stage, out)
        read_windows_acl_evidence([out.parent, out, out / EXE, out / "licenses"])
    print(f'Desktop package staged; real Windows UI acceptance still required: {out}')


def archive(args):
    root=args.package.resolve(); files=validate_shape(root)
    report_gate(args.report,root/EXE,'on',root)
    require(not args.output.exists() and not args.output.resolve().is_relative_to(root),'unsafe ZIP output')
    with zipfile.ZipFile(args.output,'x',compression=zipfile.ZIP_DEFLATED) as z:
        z.writestr('examples/target/',b'')
        for name in sorted([*files,'SHA256SUMS.txt']): z.write(root/name,name)
    with zipfile.ZipFile(args.output) as z:
        require(z.testzip() is None and set(z.namelist())=={*files,'SHA256SUMS.txt','examples/target/'},'ZIP inventory mismatch')
        for name,expected in files.items(): require(hashlib.sha256(z.read(name)).hexdigest()==expected,'ZIP bytes mismatch')
    args.output.with_suffix('.zip.sha256').write_text(f'{digest(args.output)}  {args.output.name}\n',encoding='ascii')


def main():
    p=argparse.ArgumentParser(description=__doc__); subs=p.add_subparsers(dest='command',required=True)
    b=subs.add_parser('build')
    for name in ('binary','bundle','featureoff-report','featureoff-binary','pcre2-receipt','build-info','dumpbin-report','output'): b.add_argument('--'+name,type=Path,required=True)
    b.add_argument('--ci-url',required=True)
    a=subs.add_parser('archive')
    for name in ('package','report','output'): a.add_argument('--'+name,type=Path,required=True)
    v=subs.add_parser('verify'); v.add_argument('--package',type=Path,required=True); v.add_argument('--report',type=Path,required=True)
    args=p.parse_args()
    if args.command=='build': build(args)
    elif args.command=='archive': archive(args)
    else:
        validate_shape(args.package); report_gate(args.report,args.package/EXE,'on',args.package)

if __name__=='__main__': main()
