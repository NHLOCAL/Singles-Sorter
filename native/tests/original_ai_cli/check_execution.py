"""Actual Windows positional CLI execution, with closed stdin and no developer PATH.

Disposable authored audio only. This is CI code, never an application dependency.
A report is emitted only after independent byte/layout checks, not inferred from
logs. Package contents are hash-checked before and after the acceptance run.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import stat
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'native/apps/singles-ui/tests'))
from desktop_albums import fixture as album_fixture, metadata as album_metadata, files as album_files, ARTIST, ALBUM
from desktop_flac import generated_flac, inspect_flac, FIELDS as FLAC_FIELDS, UNKNOWN as FLAC_UNKNOWN
from desktop_repair import generated_mp3, inspect_mp3, FIELDS  # Independent fixture parser.

spec = importlib.util.spec_from_file_location('execution_package_contract', ROOT / 'native/packaging/windows-preview/package_preview.py')
package = importlib.util.module_from_spec(spec); spec.loader.exec_module(package)
CHECKS = {'positional-copy', 'positional-move', 'positional-original-ai-copy',
          'positional-fix-id3v23', 'positional-fix-id3v24', 'positional-fix-partial',
          'positional-album-copy', 'positional-root-album-move', 'positional-mixed-album-move',
          'positional-album-descendant-metadata', 'positional-album-dry-run-no-effects',
          'positional-flac-five-fields', 'positional-flac-shifted-audio', 'positional-flac-opaque-partial',
          'retained-original-and-media', 'closed-stdin', 'no-developer-path',
          'explicit-dry-run-no-effects', 'overlap-rejected-no-effects', 'legacy-argument-exits'}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_wave(path):
    with wave.open(str(path), 'wb') as stream:
        stream.setnchannels(1);stream.setsampwidth(2);stream.setframerate(8000)
        stream.writeframes(bytes(1600))


def snapshot(root):
    result = []
    for path in sorted([root, *root.rglob('*')]):
        info = path.lstat()
        result.append((path.relative_to(root).as_posix(), info.st_mode, info.st_size,
                       info.st_mtime_ns, digest(path) if path.is_file() else None))
    return result


def result_record(stderr):
    records = [json.loads(line.split('result: ', 1)[1]) for line in stderr.splitlines() if ' - MusicSorter - result: ' in line]
    if len(records) != 1: raise AssertionError(f'Expected one native result: {stderr}')
    return records[0]


def require_album_recovery(report):
    require(report and report['manifest_finished'] and not report['incomplete_tail'], 'Album manifest lacks its terminal evidence')
    for key in ('pending_directories', 'pending_cleanup', 'pending_metadata'):
        require(not report[key], 'Album reconciliation contains pending effects: ' + key)
    require(all(status == 'verified' for _, status in report['directory_metadata_status']), 'Album directory metadata was not verified')
    require(all(entry['status'] == 'SameIdentity' for entry in report['directories']), 'Album destination directory identity differs or is unverified')
    transfers = report.get('transfers')
    if transfers:
        require(not transfers['append_blocked'], 'Transfer journal cannot accept a safe terminal append')
        require(all(item['status'] == 'Committed' and item['copy_metadata'] not in ('Unverified', 'Mismatch') for item in transfers['operations']), 'Album transfer reconciliation failed')
    whole = report.get('whole')
    if whole:
        require(whole['recorded_finished'] and not whole['incomplete_tail'] and not whole['pending_albums'] and not whole['pending_directories'], 'Whole-album terminal evidence is incomplete')
        require(all(item['status'] == 'Absent' for item in whole['source_evidence']), 'Moved album source remains or is unverified')
        require(all(item['status'] == 'SameIdentity' for item in whole['destination_evidence']), 'Moved album destination identity differs')
        require(all(item['status'] == 'verified' for item in whole['files']), 'Moved album file content differs')
        if whole['singles']:
            require_album_recovery(whole['singles'])


def run(binary, bundle, report, package_root=None):
    # The reused independent media parser contains assertions. Never accept a
    # receipt from an interpreter that has disabled those structural checks.
    require(__debug__, 'Execution acceptance requires Python assertions enabled')
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
        raise RuntimeError('Execution acceptance requires the owned Windows CI runner')
    binary=binary.resolve(strict=True); bundle=bundle.resolve(strict=True)
    if package_root:
        package_root=package_root.resolve(strict=True)
        if binary != package_root/'singles-sorter.exe': raise ValueError('Wrong package executable')
        package.verify_package(package_root)
    executable_hash=digest(binary)
    manifest_hash=digest(package_root/'SHA256SUMS.txt') if package_root else None
    env=dict(os.environ, PATH=str(Path(os.environ['SystemRoot'])/'System32'))
    receipts=[]
    # Keep failed evidence until runner disposal; never remove an unrelated tree.
    scratch=Path(tempfile.mkdtemp(prefix='singles-cli-execution-'))

    def invoke(cwd, args, expected=0):
        completed=subprocess.run([str(binary),*args],cwd=cwd,env=env,stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE,stderr=subprocess.PIPE,encoding='utf-8',timeout=120)
        receipts.append(dict(args=args,exit=completed.returncode,stdout=completed.stdout,stderr=completed.stderr))
        if completed.returncode != expected: raise AssertionError(receipts[-1])
        return completed

    def roots(label):
        case=scratch/label;case.mkdir()
        for child in ('source','target','journal','app'): (case/child).mkdir()
        return case

    help_result=invoke(scratch,['--help'])
    for flag in ['--copy_mode','--abc_sort','--exist_only','--no_singles_dir','--main_dir_only','--duet_mode','--fix_names','--log_level']:
        require(flag in help_result.stdout, f'Missing original CLI option: {flag}')
    for args in [['source','--unknown'],['source','target','--log_level','debug']]: invoke(scratch,args,2)

    for label, copying, ai in [('copy',True,False),('move',False,False),('original-ai',True,True)]:
        case=roots(label);source=case/'source';target=case/'target'
        name='ישראל כהן - שיר חדש.wav' if ai else 'זמר בדיקה - שיר.wav'
        artist='ישראל כהן' if ai else 'אמן בדיקה'
        (case/'app/personal-singer-list.csv').write_text('זמר בדיקה,אמן בדיקה\n',encoding='utf-8')
        original=source/name;write_wave(original);before=digest(original)
        args=['source','target','-n','--journal-directory','journal']
        if copying: args+=['-c']
        if ai:
            if not package_root: args+=['--model-directory',str(bundle)]
        else: args+=['--no-ai']
        if label=='copy': args+=['-a']
        completed=invoke(case,args);actual=result_record(completed.stderr)
        require(actual['status']=='complete' and actual.get('recovery_error') is None, f'Incomplete transfer or reconciliation: {actual}')
        destination=target/artist[0]/artist/name if label=='copy' else target/artist/name
        require(destination.is_file() and digest(destination)==before, f'Destination bytes differ: {destination}')
        require(original.exists()==copying, 'Source retention differs from selected Copy/Move mode')
        if copying:
            require(digest(original)==before, 'Copy changed source bytes')
        require(list((case/'journal').glob('*.jsonl')), 'Missing retained native journals')
        receipts[-1].update(case=label,source_sha256=before,destination_sha256=digest(destination),source_retained=copying)

    audio=(ROOT/'native/tests/fixtures/media/blank.mp3').read_bytes()
    for version,unsupported in [(3,False),(4,False),(4,True)]:
        label='partial' if unsupported else f'fix-id3v2{version}'
        case=roots(label);source=case/'source';original=source/'repair_input.mp3'
        before=generated_mp3(audio,version,unsupported);original.write_bytes(before)
        fields,raw,offset,payload=inspect_mp3(before)
        completed=invoke(case,['source','-f','--no-ai','--journal-directory','journal'],1 if unsupported else 0)
        actual=result_record(completed.stderr)
        require(actual['status']==('partial' if unsupported else 'complete') and actual.get('recovery_error') is None, f'Unexpected repair or reconciliation: {actual}')
        current=source/'repair input.mp3'
        require(current.is_file() and not original.exists(), 'Original-policy filename rename did not complete')
        after=current.read_bytes();backups=list(source.glob('*.backup'))
        if unsupported:
            require(after==before and not backups and actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==0, 'Unsupported tag repair changed bytes or claimed a completed repair')
        else:
            require(len(backups)==1 and backups[0].read_bytes()==before, 'Original repair backup was not retained exactly')
            changed,new_raw,new_offset,new_payload=inspect_mp3(after)
            require(changed=={key:value[1] for key,value in FIELDS.items()}, 'Five ID3 fields differ from the frozen repair expectation')
            require(new_offset==offset and new_payload==payload and new_raw[b'PRIV']==raw[b'PRIV'], 'Repair changed audio offset, audio bytes or unknown frame')
            require(actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==1, 'Repair counts do not match observed effects')
        receipts[-1].update(case=label,original_sha256=hashlib.sha256(before).hexdigest(),current_sha256=digest(current),backup_sha256=[digest(p) for p in backups])

    for copying, root_album, label in [(True, False, 'album-copy'), (False, True, 'root-album-move'), (False, False, 'mixed-album-move')]:
        case=scratch/label; (case/'journal').mkdir(parents=True); (case/'app').mkdir()
        source,target,album=album_fixture(case,audio,root_album)
        if copying: (album/'cover.jpg').chmod(stat.S_IREAD)
        before=album_files(album); source_before=album_files(source); metadata_before=album_metadata(album)
        args=['source','target','-n','--no-ai','--journal-directory','journal']+(['-c'] if copying else [])
        no_effect=snapshot(case)
        invoke(case,[*args,'--dry-run'])
        require(snapshot(case)==no_effect,'Album dry-run changed files, directories or metadata')
        completed=invoke(case,args); actual=result_record(completed.stderr)
        require(actual['status']=='complete' and actual.get('recovery_error') is None,'Album execution or recovery failed')
        require_album_recovery(actual['recovery'])
        require(actual['albums_completed']==1 and actual['artist_files']==[[ARTIST,4]],'Album original-policy counters differ')
        expected_files=5 if root_album else 6
        require(actual['committed_destinations']==expected_files and actual['moved_sources']==(0 if copying else expected_files),'Physical album counters differ')
        destination=target/ARTIST/ALBUM
        if copying:
            require(album_metadata(destination)==metadata_before,'Album descendant timestamp/readonly metadata differs')
            require(destination.stat().st_mtime_ns!=album.stat().st_mtime_ns,'Copy album container should be newly created')
            require(album_files(source)==source_before,'Album Copy changed source content')
        else:
            require(not album.exists(),'Album Move retained its original container')
        require(album_files(destination)==before,'Album nested files or sidecar bytes differ')
        if not root_album:
            require((target/ARTIST/f'{ARTIST} - שיר בודד.mp3').read_bytes()==audio,'Mixed single destination differs')
        receipts[-1].update(case=label,source_sha256=before,destination_sha256=album_files(destination),source_retained=copying)

    for duplicate,opaque,label in [(False,False,'flac-five-fields'),(True,False,'flac-shifted-audio'),(True,True,'flac-opaque-partial')]:
        case=roots(label);source=case/'source';original=source/'repair_input.flac'
        before=generated_flac(duplicate=duplicate,opaque=opaque);original.write_bytes(before)
        vendor,_,blocks,offset,audio_bytes=inspect_flac(before)
        completed=invoke(case,['source','-f','--no-ai','--journal-directory','journal'],1 if opaque else 0)
        actual=result_record(completed.stderr)
        require(actual['status']==('partial' if opaque else 'complete') and actual.get('recovery_error') is None,'FLAC execution or recovery failed')
        current=source/'repair input.flac';require(current.is_file() and not original.exists(),'FLAC filename was not repaired')
        after=current.read_bytes();backups=list(source.glob('*.backup'))
        require(actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==(0 if opaque else 1),'FLAC observed counts differ')
        if opaque:
            require(after==before and not backups,'Unsupported opaque FLAC tags were changed')
        else:
            require(len(backups)==1 and backups[0].read_bytes()==before,'FLAC original backup differs')
            new_vendor,values,new_blocks,new_offset,new_audio=inspect_flac(after)
            replacement='א, ב' if duplicate else 'א'
            require(values==FLAC_UNKNOWN+[f'{key}={replacement}' for key in FLAC_FIELDS],'FLAC five fields or unknown pair order differ')
            require(new_vendor==vendor and [b for b in blocks if b[0]&127!=4]==[b for b in new_blocks if b[0]&127!=4],'FLAC vendor or non-comment blocks differ')
            require(new_audio==audio_bytes and ((new_offset<offset) if duplicate else (new_offset==offset)),'FLAC audio or expected metadata offset differs')
        require(set(source.iterdir())=={current,*backups},'Unexpected FLAC staged or source files remain')
        receipts[-1].update(case=label,original_sha256=hashlib.sha256(before).hexdigest(),current_sha256=digest(current),backup_sha256=[digest(p) for p in backups])

    case=roots('dry-run');write_wave(case/'source/אברהם פריד - בדיקה.wav');before=snapshot(case)
    completed=invoke(case,['source','target','-c','--no-ai','--dry-run','-l','ERROR'])
    require('predictions only' in completed.stdout and completed.stderr=='' and snapshot(case)==before, 'Explicit dry-run changed files or output contract')
    completed=invoke(case,['source','target','-c','--no-ai','--journal-directory','source'],1)
    require(snapshot(case)==before, 'Rejected overlapping journal selection changed files')
    if package_root:
        package.verify_package(package_root)
        require(digest(package_root/'SHA256SUMS.txt')==manifest_hash, 'Application changed the package manifest')
    require(digest(binary)==executable_hash, 'Executable changed during acceptance')
    report.parent.mkdir(parents=True,exist_ok=True)
    report.write_text(json.dumps(dict(schema_version=1,result='PASS',checks=sorted(CHECKS),
        executable_sha256=executable_hash,sha256sums_sha256=manifest_hash,
        closed_stdin=True,developer_path_removed=True,real_subprocess=True,
        fixture_root=str(scratch),cases=receipts),ensure_ascii=False,indent=2),encoding='utf-8')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--bundle',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True);parser.add_argument('--package',type=Path)
    args=parser.parse_args();run(args.binary,args.bundle,args.report,args.package)


if __name__=='__main__':main()
