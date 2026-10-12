"""Actual app restart and read-only native-root reconciliation on owned files.

This is Windows WebDriver acceptance, not a portable runtime claim. Completed
Copy and Move come from the prior real selected-execution lane. Deterministic
partial-job interruption is separately covered by windows_selected_job.rs.
"""
import hashlib
import json
import os
import stat
from pathlib import Path
from desktop_ai import tree_snapshot


def file_hash(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def journal_snapshot(root, permissions=None):
    """Strict app/journal namespace, with one explicit live-WebView boundary.

    Tauri documents APPLOCALDATA/EBWebView as Windows browser data. Never walk
    that live subtree; retain its exact directory identity/ACL/attributes instead.
    Its contents and last-write time may change while the UI is running. Every
    other path, including profile-v1 and nested folders called EBWebView, stays
    fully protected. Missing EBWebView is valid; new/replaced entries still differ.
    See native/docs/restart-journal-snapshot.md for pinned-source limitations.
    """
    root = Path(root)
    options = {'permissions': permissions} if permissions is not None else {}
    protected = tree_snapshot(root, recursive=False, **options)
    assert protected[0]['directory'], 'Journal namespace root must be a directory'
    live = None
    for child in sorted(root.iterdir(), key=lambda path: path.name):
        if child.name == 'EBWebView':
            info = child.lstat()
            assert stat.S_ISDIR(info.st_mode) and not stat.S_ISLNK(info.st_mode) and not (
                getattr(info, 'st_file_attributes', 0) & 0x400), 'Live WebView boundary must be a real non-reparse directory'
            live = tree_snapshot(child, recursive=False, **options)[0]
            assert live['directory'], 'Live WebView boundary changed kind during observation'
            live['path'] = child.name
            live.pop('modified_ns')
            live.pop('win32_last_write_ticks', None)
        else:
            for row in tree_snapshot(child, **options):
                row['path'] = child.name if row['path'] == '.' else child.name + '/' + row['path']
                protected.append(row)
    return {'protected_namespace': protected, 'live_webview_subtree': live}


def selected_cases(receipt):
    return [next(row for row in receipt['cases'] if row.get('copy_mode') is copy)
            for copy in (True, False)]


def copied_cases(receipt):
    # Same-volume duet/mixed roots survive the prior helper. Its separate-volume
    # temporary target is deliberately cleaned up and cannot supply restart proof.
    selected = [next(row for row in receipt['cases'] if row['case'] == name)
                for name in ('duet', 'mixed')]
    for row in selected:
        if row.get('cross_volume') is not False or not all(row.get(key) for key in ('source', 'target', 'journal_path', 'destinations')):
            raise ValueError('Copied restart requires retained actual roots, journals and destination receipts')
    return selected


def copied_snapshot(root):
    root = Path(root)
    return {'tree': tree_snapshot(root), 'named_streams': {
        str(path.relative_to(root)): file_hash(Path(str(path) + ':notes'))
        for path in sorted(root.rglob('*.wav'))}}


def repair_cases(receipt):
    selected = [next(row for row in receipt['cases'] if row['case'] == name)
                for name in ('id3v24', 'unsupported')]
    for row in selected:
        if not row.get('source') or not row.get('journal_path'):
            raise ValueError('Fix restart requires actual source and journal receipts')
    return selected


def self_scope(script):
    return script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryScope")


def exercise_recovery(script, click, wait, capture, pick, restart, binary, output):
    output = Path(output)
    receipt = json.loads((output / 'selected-execution.json').read_text(encoding='utf-8'))
    cases = selected_cases(receipt)
    restart(binary, 'restart-recovery-owned-app')
    assert script("return document.querySelector('#source').value==='' && document.querySelector('#target').value===''")
    inspected = []
    families = set()
    inspectable_whole_seen = False

    def idle():
        wait(lambda: script("return document.querySelector('#recovery-dialog').open && document.querySelector('#recovery-busy').hidden"))

    def choose(source):
        nonlocal inspectable_whole_seen
        click('#menu'); click('#recovery-menu'); idle()
        for _ in range(26):  # At most 256 retained records, ten per page.
            count = script("return document.querySelectorAll('#recovery-list button').length")
            for index in range(count):
                selector = f'#recovery-list button:nth-child({index + 1})'
                click(selector)
                family = script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryFamily")
                families.add(family)
                state = script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryState")
                if family == 'album' and state == 'inspection_needed' and self_scope(script) == 'recorded_main_hash_identity_membership':
                    inspectable_whole_seen = True
                if family == 'orphan' or state in ('unsupported', 'invalid'):
                    assert script("return document.querySelector('#recovery-selection').hidden && document.querySelector('#recovery-inspect').disabled")
                    assert not script("return document.querySelector('#recovery-error').hidden")
                if script("return document.querySelector('#recovery-recorded-source').value") == str(source):
                    return script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryJob")
            if script("return document.querySelector('#recovery-next').disabled"):
                break
            click('#recovery-next'); idle()
        raise AssertionError('Restarted native host did not discover the real persisted job journal')

    for case in cases:
        source, target, journals = (Path(case[key]) for key in ('source', 'target', 'journal_path'))
        before_source, before_target = tree_snapshot(source), tree_snapshot(target)
        before_journals = journal_snapshot(journals)
        changed_target = None
        job_id = choose(source)
        recorded = {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')}
        assert len(recorded) == 2
        assert script("return document.querySelector('#recovery-inspect').disabled"), 'Historical paths became current authority'
        capture('recovery-restarted-' + ('copy' if case['copy_mode'] else 'move'))
        pick('#recovery-source-picker', source); idle()
        assert script("return document.querySelector('#recovery-inspect').disabled"), 'Missing target authority was accepted'
        pick('#recovery-target-picker', source); idle()
        click('#recovery-inspect'); idle()
        assert not script("return document.querySelector('#recovery-error').hidden")
        assert script("return document.querySelector('#recovery-report').hidden"), 'Wrong role/root became authority'
        pick('#recovery-target-picker', target); idle()
        click('#recovery-inspect')
        wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
        assert script("return document.querySelector('#recovery-result-title').textContent") == 'הקבצים שנבדקו תואמים ליומן'
        assert script("return document.querySelectorAll('#recovery-operation-list article').length") == 2
        measured_progress = script("return document.querySelector('#recovery-busy').textContent")
        assert '2 מתוך 2' in measured_progress and 'MiB' in measured_progress and 'שניות' in measured_progress
        assert tree_snapshot(source) == before_source and tree_snapshot(target) == before_target
        capture('recovery-consistent-' + ('copy' if case['copy_mode'] else 'move'))
        if case['copy_mode']:
            # Closing revokes the renderer's selections. Reopening must obtain
            # new native choices, even after a previously complete inspection.
            click('#recovery-close')
            assert choose(source) == job_id
            assert script("return document.querySelector('#recovery-inspect').disabled")
            pick('#recovery-source-picker', source); idle()
            pick('#recovery-target-picker', target); idle()
            # External test mutation only, in a disposable generated destination.
            changed_file = next(target.rglob('*.wav'))
            with changed_file.open('ab') as stream:
                stream.write(b'owned-external-change-after-restart')
            changed_target = tree_snapshot(target)
            click('#recovery-inspect')
            wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
            assert script("return document.querySelector('#recovery-result-title').textContent") == 'נדרשת בדיקה נוספת'
            assert tree_snapshot(source) == before_source and tree_snapshot(target) == changed_target
            capture('recovery-external-change-detected')
        assert {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')} == recorded
        assert journal_snapshot(journals) == before_journals
        inspected.append({'job_id': job_id, 'copy_mode': case['copy_mode'], 'journal_sha256': recorded, 'measured_progress': measured_progress,
                          'source_before': before_source, 'target_before': before_target,
                          'journals_before': before_journals, 'owned_changed_target': changed_target})
        click('#recovery-close')
    copied_receipt = json.loads((output / 'copied-move-execution.json').read_text(encoding='utf-8'))
    for case in copied_cases(copied_receipt):
        source, target, journals = (Path(case[key]) for key in ('source', 'target', 'journal_path'))
        before_source, before_target = copied_snapshot(source), copied_snapshot(target)
        before_journals = journal_snapshot(journals)
        job_id = choose(source)
        assert script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryState") == 'inspection_needed'
        assert script("return document.querySelector('#recovery-inspect').disabled")
        recorded = {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')}
        assert len(recorded) == 2
        pick('#recovery-source-picker', source); idle()
        pick('#recovery-target-picker', target); idle()
        click('#recovery-inspect')
        wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
        assert script("return document.querySelector('#recovery-result-title').textContent") == 'הקבצים שנבדקו תואמים ליומן'
        assert script("return document.querySelectorAll('#recovery-operation-list article').length") == len(case['destinations'])
        assert copied_snapshot(source) == before_source and copied_snapshot(target) == before_target
        capture('recovery-copied-' + case['case'])
        changed_target = None
        if case['case'] == 'duet':
            destination = next(target.rglob('*.wav'))
            metadata = destination.stat()
            with Path(str(destination) + ':notes').open('ab') as stream:
                stream.write(b'owned-changed-ADS-after-restart')
            os.utime(destination, ns=(metadata.st_atime_ns, metadata.st_mtime_ns))
            changed_target = copied_snapshot(target)
            click('#recovery-inspect')
            wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
            assert script("return document.querySelector('#recovery-result-title').textContent") == 'נדרשת בדיקה נוספת'
            assert copied_snapshot(source) == before_source and copied_snapshot(target) == changed_target
            capture('recovery-copied-changed-named-stream')
        assert {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')} == recorded
        assert journal_snapshot(journals) == before_journals
        inspected.append({'job_id': job_id, 'family': 'single', 'case': case['case'], 'copied_move': True,
                          'source_before': before_source, 'target_before': before_target,
                          'journals_before': before_journals, 'owned_changed_target': changed_target})
        click('#recovery-close')
    repair_receipt = json.loads((output / 'repair-execution.json').read_text(encoding='utf-8'))
    for case in repair_cases(repair_receipt):
        source, journals = Path(case['source']), Path(case['journal_path'])
        before_source, before_journals = tree_snapshot(source), journal_snapshot(journals)
        job_id = choose(source)
        assert script("return document.querySelector('#recovery-target-picker').hidden && document.querySelector('#recovery-inspect').disabled")
        pick('#recovery-source-picker', source); idle()
        assert not script("return document.querySelector('#recovery-inspect').disabled")
        click('#recovery-inspect')
        wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
        expected = 'נדרשת בדיקה נוספת' if case['case'] == 'unsupported' else 'הקבצים שנבדקו תואמים ליומן'
        assert script("return document.querySelector('#recovery-result-title').textContent") == expected
        assert tree_snapshot(source) == before_source and journal_snapshot(journals) == before_journals
        capture('recovery-fix-' + case['case'])
        changed_source = None
        if case['case'] == 'id3v24':
            assert script("return [...document.querySelectorAll('[data-recovery-evidence-role=backup]')].some(row=>row.dataset.recoveryEvidenceStatus==='Verified')")
            backup = next(source.glob('*.backup'))
            with backup.open('ab') as stream:
                stream.write(b'owned-external-backup-change')
            changed_source = tree_snapshot(source)
            click('#recovery-inspect')
            wait(lambda: script("return !document.querySelector('#recovery-report').hidden && document.querySelector('#recovery-busy').hidden"), timeout=120)
            assert script("return document.querySelector('#recovery-result-title').textContent") == 'נדרשת בדיקה נוספת'
            assert script("return [...document.querySelectorAll('[data-recovery-evidence-role=backup]')].some(row=>row.dataset.recoveryEvidenceStatus==='Changed')")
            assert tree_snapshot(source) == changed_source and journal_snapshot(journals) == before_journals
            capture('recovery-fix-backup-change-detected')
        inspected.append({'job_id': job_id, 'family': 'repair', 'case': case['case'], 'source_before': before_source,
                          'journals_before': before_journals, 'owned_changed_source': changed_source})
        click('#recovery-close')
    # Whole-object Album records are inspectable; their complete evidence has a separate gate.
    assert {'album', 'repair'} <= families and inspectable_whole_seen
    checks = ['actual-process-restart', 'fresh-native-role-selections', 'wrong-roots-rejected',
              'historical-vs-current-state', 'per-file-reconciliation', 'recovery-no-file-or-journal-writes',
              'completed-copy-and-move-reconciled', 'copied-duet-and-mixed-restart-reconciled', 'copied-restart-named-stream-change-detected', 'whole-album-inspection-available',
              'completed-fix-reconciled', 'partial-fix-stays-unresolved', 'fix-source-only-fresh-grant', 'fix-retained-backup-change-detected',
              'measured-streaming-progress', 'close-reopen-requires-fresh-roots']
    (output / 'restart-recovery.json').write_text(json.dumps({'status': 'passed', 'ipc_mocked': False,
        'jobs': inspected, 'checks': checks}, ensure_ascii=False, indent=2), encoding='utf-8')
    return checks
