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
import re
from pathlib import Path
import subprocess
import sys
import stat
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'native/apps/singles-ui/tests'))
from desktop_albums import fixture as album_fixture, metadata as album_metadata, files as album_files, ARTIST, ALBUM
from desktop_copied_move import DUET, SINGLE, ARTISTS, AUXILIARY, auxiliary
from desktop_ogg import generated_ogg, verify_repaired as verify_ogg_repaired
from check_cross_albums import exercise as exercise_cross_albums, CHECKS as CROSS_ALBUM_CHECKS
from desktop_mp4 import generated_mp4, verify_repaired as verify_mp4_repaired
from desktop_id3_container import CASES as CONTAINER_CASES, generated_container, verify_repaired as verify_container_repaired
from desktop_flac import generated_flac, inspect_flac, FIELDS as FLAC_FIELDS, UNKNOWN as FLAC_UNKNOWN
from desktop_repair import generated_mp3, inspect_mp3, FIELDS  # Independent fixture parser.

spec = importlib.util.spec_from_file_location('execution_package_contract', ROOT / 'native/packaging/windows-preview/package_preview.py')
package = importlib.util.module_from_spec(spec); spec.loader.exec_module(package)
CHECKS = {'positional-one-eligible-duet-move', 'positional-zero-eligible-duet-source-retained', 'positional-wave-id3-five-fields', 'positional-aiff-id3-five-fields', 'positional-dsf-id3-five-fields',
          'positional-container-id3-exact-extent-decoded-audio', 'positional-container-id3-insufficient-padding-partial', 'positional-m4a-five-fields', 'positional-alac-five-fields', 'positional-mp4-fast-start-values',
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

CHECKS |= CROSS_ALBUM_CHECKS

def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


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
    records = [json.loads(match.group(1)) for line in stderr.splitlines()
               if (match := re.fullmatch(r'\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} - MusicSorter - INFO - result: (.*)', line))]
    if len(records) != 1: raise AssertionError(f'Expected one native result: {stderr}')
    return records[0]


def log_inventory(root):
    logs = root/'logs'
    if not logs.exists(): return {}
    require(logs.is_dir() and not logs.is_symlink(), 'Unexpected diagnostic log parent')
    entries = {}
    for path in logs.rglob('*'):
        info = path.lstat()
        require(not path.is_symlink(), 'Diagnostic history contains an unexpected link')
        entries[path.relative_to(logs).as_posix()] = (info.st_mode, info.st_size, info.st_mtime_ns, digest(path) if path.is_file() else None)
    return entries


def verify_new_log(root, before, stderr):
    after = log_inventory(root)
    added = set(after)-set(before)
    require(len(added)==1 and all(after.get(name)==value for name,value in before.items()), 'Logging changed an existing file or did not create exactly one file')
    name = next(iter(added))
    require(re.fullmatch(r'music_sorter_\d{8}_\d{6}_[1-9]\d*\.log',name), 'Unexpected diagnostic log filename')
    path = root/'logs'/name
    require(path.read_bytes()==stderr.encode('utf-8'), 'UTF-8 file and stderr log records differ')
    return dict(relative_path='logs/'+name,sha256=digest(path),bytes=path.stat().st_size)


def no_effect_except_observed_log(before, after, log):
    old={row[0]:row[1:] for row in before}; new={row[0]:row[1:] for row in after}
    created_parent='logs' not in old
    expected={log['relative_path']}|({'logs'} if created_parent else set())
    require(set(new)-set(old)==expected and not (set(old)-set(new)), 'Rejected execution changed more than its one diagnostic log')
    for name,value in old.items():
        if name=='logs' or (name=='.' and created_parent):
            require(value[0]==new[name][0], 'Diagnostic logging changed directory kind or mode')
        else:
            require(new[name]==value, 'Rejected execution changed existing entry: '+name)
    require(new[log['relative_path']][-1]==log['sha256'], 'Observed diagnostic log content changed')


def require_album_recovery(report):
    require(report and report['manifest_finished'] and not report['incomplete_tail'], 'Album manifest lacks its terminal evidence')
    for key in ('pending_directories', 'pending_cleanup', 'pending_metadata'):
        require(not report[key], 'Album reconciliation contains pending effects: ' + key)
    require(all(status == 'verified' for _, status in report['directory_metadata_status']), 'Album directory metadata was not verified')
    require(report.get('recorded_status') in (None, 'Complete'), 'Album cleanup did not record successful completion')
    key = lambda reference, identity: json.dumps([reference, identity], sort_keys=True, separators=(',', ':'))
    removed = {key(reference, identity) for reference, identity in report.get('removed_directory_references', [])}
    observed_removed = set()
    for entry in report['directories']:
        role_key = key(entry['reference'], entry['expected_identity'])
        if role_key in removed:
            require(entry['status'] == 'Absent', 'Acknowledged removed album source directory reappeared or is unverified')
            observed_removed.add(role_key)
        else:
            require(entry['status'] == 'SameIdentity', 'Album destination directory identity differs or is unverified')
    require(observed_removed == removed, 'Removed source directory evidence is incomplete')
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


def require_copied_recovery(report):
    require(report and report['directories']['complete'] and not report['directories']['incomplete_tail'], 'Copied Move directory journal lacks its terminal record')
    require(not report['directories']['pending'] and not report['directories']['rejected'], 'Copied Move directory evidence is uncertain')
    require(all(len(entry) == 2 and entry[1] for entry in report['directories']['observed']), 'Copied Move observed directory identity is missing')
    transfers = report['transfers']
    require(transfers and not transfers['append_blocked'], 'Copied Move transfer journal is incomplete')
    require(transfers['operations'] and all(item['status'] == 'Committed' and item['copy_metadata'] in ('Verified', 'NotRequested') for item in transfers['operations']), 'Copied Move transfer or metadata reconciliation failed')


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

    def invoke(cwd, args, expected=0, expect_log=True):
        before_logs=log_inventory(cwd)
        completed=subprocess.run([str(binary),*args],cwd=cwd,env=env,stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE,stderr=subprocess.PIPE,encoding='utf-8',timeout=120)
        receipts.append(dict(args=args,exit=completed.returncode,stdout=completed.stdout,stderr=completed.stderr))
        if completed.returncode != expected: raise AssertionError(receipts[-1])
        if not expect_log or expected==2 or '--help' in args or '--dry-run' in args or '--decision-only' in args:
            require(log_inventory(cwd)==before_logs,'Read-only/help/invalid invocation changed diagnostic logs')
        else:
            receipts[-1]['log']=verify_new_log(cwd,before_logs,completed.stderr)
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

    # C: owns the temporary source; the existing CI checkout supplies D:.
    # No mounts, security changes, or synthetic volume identifiers are used.
    other = Path(tempfile.mkdtemp(prefix='singles-cli-copied-target-', dir=ROOT))
    require(scratch.stat().st_dev != other.stat().st_dev, 'Copied Move gate needs two actual Windows volumes')
    for label in ('cross-volume', 'duet-move', 'mixed-duet-move'):
        case = roots(label); source = case/'source'
        target = other/label if label == 'cross-volume' else case/'target'
        if label == 'cross-volume': target.mkdir()
        configured = label != 'cross-volume'
        names = [DUET] if configured else [SINGLE]
        if label == 'mixed-duet-move': names.append(SINGLE)
        before = {}; identities = {}
        for name in names:
            path = source/name; write_wave(path); auxiliary(path).write_bytes(AUXILIARY)
            os.utime(path, ns=(1_700_000_000_000_000_000, 1_700_000_000_000_000_000))
            before[name] = digest(path); identities[name] = path.stat().st_ino
        args = ['source', str(target), '-n', '--journal-directory', 'journal']
        if configured:
            args += ['-d']
            if not package_root: args += ['--model-directory', str(bundle)]
        else: args += ['--no-ai']
        completed = invoke(case, args); actual = result_record(completed.stderr)
        require(actual['status'] == 'complete' and actual.get('recovery_error') is None, 'Copied Move did not complete and reconcile')
        require_copied_recovery(actual['recovery'])
        expected = {(artist, DUET) for artist in ARTISTS} if configured else {('אברהם פריד', SINGLE)}
        if label == 'mixed-duet-move': expected.add(('אברהם פריד', SINGLE))
        require({(path.parent.name, path.name) for path in target.rglob('*.wav')} == expected, 'Actual copied Move destinations differ')
        for artist, name in expected:
            path = target/artist/name
            require(digest(path) == before[name] and auxiliary(path).read_bytes() == AUXILIARY, 'Copied Move main/named-stream bytes differ')
            require(path.stat().st_mtime_ns == 1_700_000_000_000_000_000, 'Copied Move timestamp differs')
            if label == 'mixed-duet-move' and name == SINGLE:
                require(path.stat().st_ino == identities[name], 'Ordinary same-volume file did not retain rename identity')
        require(not list(source.iterdir()), 'Copied Move source was not retired')
        require(actual['source_items'] == len(names) and actual['moved_sources'] == len(names) and actual['committed_destinations'] == len(expected), 'Copied Move physical counters differ')
        require(actual['legacy_summary']['songs_sorted'] == len(expected), 'Copied Move original destination credits differ')
        effects = actual['effect_receipts']
        require(sum(effect['kind'] == 'source_retired' for effect in effects) == 1, 'Copied source retirement was not acknowledged exactly once')
        require(sum(effect['kind'] == 'file_renamed' for effect in effects) == (1 if label == 'mixed-duet-move' else 0), 'Ordinary same-volume rename receipt is missing or fabricated')
        receipts[-1].update(case=label, source_sha256=before, source_items=len(names), destinations=len(expected), named_streams_verified=True)

    for eligible in (1, 0):
        label='one-eligible-duet' if eligible else 'zero-eligible-duet'
        case=roots(label); source=case/'source'; target=case/'target'; journal=case/'journal'
        original=source/DUET; write_wave(original); auxiliary(original).write_bytes(AUXILIARY)
        before=digest(original); identity=original.stat().st_ino
        if eligible: (target/ARTISTS[0]).mkdir()
        source_before,target_before,journal_before=snapshot(source),snapshot(target),snapshot(journal)
        args=['source','target','-nde','--journal-directory','journal']
        if not package_root: args += ['--model-directory',str(bundle)]
        actual=result_record(invoke(case,args).stderr)
        require(actual['status']=='complete' and actual.get('recovery_error') is None,'Eligible duet did not complete')
        require(actual['committed_destinations']==eligible and actual['moved_sources']==eligible,'Eligible duet physical counters differ')
        require(actual['legacy_summary']=={'songs_sorted':eligible,'artist_folders_created':0,'albums_processed':0,'top_artists':[[ARTISTS[0],1]] if eligible else []},'Eligible duet original summary differs')
        if eligible:
            require_copied_recovery(actual['recovery'])
            destination=target/ARTISTS[0]/DUET
            require(not original.exists() and digest(destination)==before and auxiliary(destination).read_bytes()==AUXILIARY,'Single eligible destination/source retirement differs')
            require({path.relative_to(target).as_posix() for path in target.rglob('*')}=={ARTISTS[0],ARTISTS[0]+'/'+DUET},'Skipped duet destination was created')
            require(sum(effect['kind']=='source_retired' for effect in actual['effect_receipts'])==1,'Eligible source retirement receipt differs')
        else:
            require(actual['no_op'] is True and actual['items']==[] and actual['effect_receipts']==[] and actual['journal_directory'] is None,'Zero-eligible duet fabricated execution evidence')
            require(snapshot(source)==source_before and snapshot(target)==target_before and snapshot(journal)==journal_before,'Zero-eligible duet changed media or journal namespace')
            require(original.stat().st_ino==identity and auxiliary(original).read_bytes()==AUXILIARY,'Zero-eligible duet source identity or named stream changed')
        receipts[-1].update(case=label,observed_result=actual,source_sha256=before,source_retained=not eligible)

    audio=(ROOT/'native/tests/fixtures/media/blank.mp3').read_bytes()
    exercise_cross_albums(invoke, result_record, require_album_recovery, scratch, other, audio, receipts)
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

    for explicit in (False,True):
        case=roots('primary-explicit' if explicit else 'primary-cwd')
        (case/'app/singer-list.csv').write_text('זמר בדיקה,ראשי מקומי\n',encoding='utf-8')
        (case/'primary.csv').write_text('זמר בדיקה,ראשי מפורש\n',encoding='utf-8')
        (case/'app/personal-singer-list.csv').write_text('זמר בדיקה,אישי שני\n',encoding='utf-8')
        name='זמר בדיקה - שיר.wav';write_wave(case/'source'/name);before=digest(case/'source'/name)
        args=['source','target','-cn','--no-ai','--journal-directory','journal']
        if explicit: args+=['--singer-list','primary.csv']
        completed=invoke(case,args);actual=result_record(completed.stderr)
        artist='ראשי מפורש' if explicit else 'ראשי מקומי'
        require(actual['status']=='complete' and actual.get('recovery_error') is None,'Primary alias resource execution failed')
        require(digest(case/'target'/artist/name)==before and digest(case/'source'/name)==before,'Selected primary aliases did not precede personal aliases')
        require(not (case/'target/אישי שני').exists(),'Personal alias unexpectedly replaced the earlier primary row')
        receipts[-1].update(case='primary-explicit' if explicit else 'primary-cwd',primary_first=True,source_sha256=before)
    for opus,duplicate,chained in [(False,False,False),(True,True,False),(False,True,True),(True,True,True)]:
        extension='opus' if opus else 'ogg';label=extension+('-chained' if chained else '-repair')
        case=roots(label);source=case/'source';original=source/f'repair_input.{extension}'
        before=generated_ogg(opus,duplicate=duplicate)
        if chained: before+=before
        original.write_bytes(before)
        completed=invoke(case,['source','-f','--no-ai','--journal-directory','journal'],1 if chained else 0)
        actual=result_record(completed.stderr)
        require(actual['status']==('partial' if chained else 'complete') and actual.get('recovery_error') is None,'Ogg execution or recovery failed')
        current=source/f'repair input.{extension}';require(current.is_file() and not original.exists(),'Ogg filename was not repaired')
        after=current.read_bytes();backups=list(source.glob('*.backup'))
        require(actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==(0 if chained else 1),'Ogg observed counts differ')
        if chained:
            require(after==before and not backups,'Chained Ogg was changed by tag repair')
        else:
            verify_ogg_repaired(before,after,duplicate)
            require(len(backups)==1 and backups[0].read_bytes()==before,'Ogg original backup differs')
        require(set(source.iterdir())=={current,*backups},'Unexpected Ogg staged or source files remain')
        receipts[-1].update(case=label,original_sha256=hashlib.sha256(before).hexdigest(),current_sha256=digest(current),backup_sha256=[digest(p) for p in backups])

    for alac,multiple,duplicate,opaque,label in [(False,False,False,False,'m4a-five-fields'), (True,False,False,False,'alac-five-fields'),
                                                 (False,True,False,False,'mp4-fast-start-values'), (False,False,True,False,'mp4-duplicate-outer'),
                                                 (False,False,False,True,'mp4-opaque-structure')]:
        case=roots(label); source=case/'source'; extension='alac' if alac else 'm4a'
        original=source/f'repair_input.{extension}'; current=source/f'repair input.{extension}'
        before=generated_mp4(alac=alac,multiple=multiple,fast_start=multiple,duplicate_outer=duplicate,opaque_structure=opaque); original.write_bytes(before)
        completed=invoke(case,['source','-f','--no-ai','--journal-directory','journal'],1 if opaque else 0)
        actual=result_record(completed.stderr)
        require(actual['status']==('partial' if opaque else 'complete') and actual.get('recovery_error') is None,'MP4 repair or reconciliation failed')
        require(current.is_file() and not original.exists(),'MP4 filename repair did not complete')
        after=current.read_bytes(); backups=list(source.glob('*.backup')); media=None
        require(actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==(0 if opaque else 1),'MP4 observed counts differ')
        if opaque:
            require(after==before and not backups,'Opaque MP4 structure was rewritten or backed up as repaired')
            require(actual['repair']['unsupported_files']==1,'Opaque MP4 refusal was not reported')
        else:
            require(len(backups)==1 and backups[0].read_bytes()==before,'Exact MP4 original backup is missing')
            media=verify_mp4_repaired(before,after,multiple)
        require(set(source.iterdir())=={current,*backups},'Unexpected MP4 source or staged files remain')
        receipts[-1].update(case=label,original_sha256=hashlib.sha256(before).hexdigest(),current_sha256=digest(current),
                            backup_sha256=[digest(path) for path in backups],observed_repair=actual['repair'],media=media)

    for kind,version,unsupported in CONTAINER_CASES:
        label='container-'+kind+'-v2'+str(version)+('-partial' if unsupported else '')
        case=roots(label); source=case/'source'; original=source/('repair_input.'+kind); current=source/('repair input.'+kind)
        before=generated_container(kind,version,unsupported); original.write_bytes(before)
        completed=invoke(case,['source','-f','--no-ai','--journal-directory','journal'],1 if unsupported else 0)
        actual=result_record(completed.stderr)
        require(actual['status']==('partial' if unsupported else 'complete') and actual.get('recovery_error') is None,'Container ID3 repair or reconciliation failed')
        require(current.is_file() and not original.exists(),'Container ID3 filename repair did not complete')
        after=current.read_bytes(); backups=list(source.glob('*.backup')); media=None
        require(actual['repair']['renamed_files']==1 and actual['repair']['repaired_files']==(0 if unsupported else 1),'Container ID3 observed counts differ')
        if unsupported:
            require(after==before and not backups,'Insufficient ID3 padding produced a changed or backed-up tag')
            require(actual['repair']['unsupported_files']==1,'Container ID3 refusal was not reported')
        else:
            require(len(backups)==1 and backups[0].read_bytes()==before,'Exact container original backup is missing')
            media=verify_container_repaired(before,after,kind)
        require(set(source.iterdir())=={current,*backups},'Unexpected container source or staging objects remain')
        receipts[-1].update(case=label,original_sha256=hashlib.sha256(before).hexdigest(),current_sha256=digest(current),
                            backup_sha256=[digest(path) for path in backups],observed_repair=actual['repair'],media=media)

    for level in ('DEBUG','INFO','WARNING','ERROR','CRITICAL'):
        case=roots('log-level-'+level.lower());name='אברהם פריד - בדיקה.wav'
        write_wave(case/'source'/name);before=digest(case/'source'/name)
        completed=invoke(case,['source','target','-cn','--no-ai','--journal-directory','journal','-l',level])
        require(digest(case/'target/אברהם פריד'/name)==before,'Logging trial did not execute its copy')
        if level in ('DEBUG','INFO'):
            require(result_record(completed.stderr)['status']=='complete','Logging trial result incomplete')
            require((' - MusicSorter - DEBUG - ' in completed.stderr)==(level=='DEBUG'),'Debug filtering differs from selected log level')
        else:
            require(completed.stderr=='','High log level emitted lower-level records')
        receipts[-1].update(case='log-level-'+level.lower(),level=level)

    for selected_cwd in ('source','source/nested','target/nested','journal/nested'):
        case=roots('log-overlap-'+selected_cwd.replace('/','-'))
        for directory in ('source/nested','target/nested','journal/nested'): (case/directory).mkdir()
        write_wave(case/'source/אברהם פריד - בדיקה.wav');before=snapshot(case)
        completed=invoke(case/selected_cwd,[str(case/'source'),str(case/'target'),'-cn','--no-ai','--journal-directory',str(case/'journal')],1,expect_log=False)
        require('LOG_CREATE_FAILED' in completed.stderr and snapshot(case)==before,'Diagnostic overlap was not refused before effects')
        receipts[-1].update(case='log-overlap-'+selected_cwd,no_effects=True)

    case=roots('dry-run');write_wave(case/'source/אברהם פריד - בדיקה.wav');before=snapshot(case)
    completed=invoke(case,['source','target','-c','--no-ai','--dry-run','-l','ERROR'])
    require('predictions only' in completed.stdout and completed.stderr=='' and snapshot(case)==before, 'Explicit dry-run changed files or output contract')
    completed=invoke(case,['source','target','-c','--no-ai','--journal-directory','source'],1)
    no_effect_except_observed_log(before,snapshot(case),receipts[-1]['log'])
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
