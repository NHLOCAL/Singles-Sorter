"""Real restarted file-Album UI inspection while every owned root still exists."""
import json
import os
from pathlib import Path

from desktop_ai import tree_snapshot
from desktop_recovery import file_hash, journal_snapshot

CHECKS = {'file-album-copy-restarted-with-fresh-source',
          'cross-album-root-restarted-with-fresh-parent',
          'cross-album-mixed-restarted-with-fresh-source',
          'file-album-source-location-explicit', 'file-album-parent-refusal',
          'file-album-current-scope-evidence', 'file-album-restart-no-effects',
          'file-album-stream-change-detected', 'file-album-recreated-root-refused',
          'file-album-restart-media-progress', 'file-album-close-revokes-selections'}


def selected_case(case):
    required = ('case', 'source', 'target', 'journal_path', 'file_count')
    if not all(case.get(key) is not None for key in required) or type(case['file_count']) is not int or case['file_count'] < 1:
        raise ValueError('File-Album restart requires actual retained roots and file count')
    if case['case'] not in ('copy', 'root-move', 'mixed-move'):
        raise ValueError('Unsupported owned file-Album restart case')
    if case['case'] != 'copy' and case.get('cross_volume') is not True:
        raise ValueError('Whole-object same-volume journals are a separate recovery family')
    return case


def named_streams(target, enabled):
    if not enabled:
        return {}
    return {path.relative_to(target).as_posix(): file_hash(Path(str(path) + ':notes'))
            for path in sorted(target.rglob('*')) if path.is_file()}


class AlbumRecovery:
    def __init__(self, script, click, wait, capture, pick, restart, binary, output):
        if not __debug__:
            raise RuntimeError('Physical acceptance requires Python assertions enabled')
        self.script, self.click, self.wait, self.capture = script, click, wait, capture
        self.pick, self.restart, self.binary, self.output = pick, restart, binary, Path(output)
        self.cases = []

    def idle(self):
        self.wait(lambda: self.script("return document.querySelector('#recovery-dialog').open && document.querySelector('#recovery-busy').hidden"))

    def choose(self, source):
        self.click('#menu'); self.click('#recovery-menu'); self.idle()
        for _ in range(26):
            count = self.script("return document.querySelectorAll('#recovery-list button').length")
            for index in range(count):
                self.click(f'#recovery-list button:nth-child({index + 1})')
                if self.script("return document.querySelector('#recovery-recorded-source').value") == str(source):
                    expected_scope = getattr(self, 'expected_scope', None)
                    if expected_scope is not None and self.script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryScope") != expected_scope:
                        continue
                    assert self.script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryFamily") == 'album'
                    assert self.script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryState") == 'inspection_needed'
                    return self.script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryJob")
            if self.script("return document.querySelector('#recovery-next').disabled"):
                break
            self.click('#recovery-next'); self.idle()
        raise AssertionError('Fresh host did not discover the actual retained file-Album journal')

    def inspect(self):
        self.click('#recovery-inspect')
        self.wait(lambda: self.script("return document.querySelector('#recovery-busy').hidden && (!document.querySelector('#recovery-report').hidden || !document.querySelector('#recovery-error').hidden)"), timeout=120)

    def run(self, case):
        case = selected_case(case)
        source, target, journals = (Path(case[key]) for key in ('source', 'target', 'journal_path'))
        copying = case['case'] == 'copy'; removed = case['case'] == 'root-move'
        protected_source = source.parent if removed else source
        assert source.exists() == (not removed)
        before_source, before_target = tree_snapshot(protected_source), tree_snapshot(target)
        before_streams = named_streams(target, not copying)
        self.restart(self.binary, 'file-album-restart-' + case['case'])
        assert self.script("return document.querySelector('#source').value==='' && document.querySelector('#target').value===''")
        before_journals = journal_snapshot(journals)
        job_id = self.choose(source)
        hashes = {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')}
        assert set(hashes) == {job_id + '.album.jsonl', job_id + '.transfers.jsonl'}
        assert self.script("return document.querySelector('#recovery-inspect').disabled")
        assert self.script("return document.querySelector('#recovery-source-location').value") == 'present', 'Historical completion inferred present-day absence'
        if removed:
            assert not self.script("return document.querySelector('#recovery-source-mode').hidden")
            self.script("document.querySelector('#recovery-source-location').value='removed';document.querySelector('#recovery-source-location').dispatchEvent(new Event('change',{bubbles:true}));")
            self.pick('#recovery-source-picker', target); self.idle()
            assert not self.script("return document.querySelector('#recovery-error').hidden")
            assert self.script("return document.querySelector('#recovery-inspect').disabled")
            self.pick('#recovery-source-picker', source.parent); self.idle()
        else:
            self.pick('#recovery-source-picker', source); self.idle()
        assert self.script("return document.querySelector('#recovery-inspect').disabled")
        self.pick('#recovery-target-picker', target); self.idle()
        self.inspect()
        assert self.script("return document.querySelector('#recovery-error').hidden")
        assert self.script("return document.querySelector('#recovery-result-title').textContent") == 'הקבצים שנבדקו תואמים ליומן'
        observed_file_rows = self.script("return document.querySelectorAll('#recovery-operation-list article').length")
        assert observed_file_rows == case['file_count']
        assert self.script("return [...document.querySelectorAll('#recovery-operation-list article')].every(row=>row.dataset.recoveryOperationStatus==='Committed')")
        evidence = self.script("return [...document.querySelectorAll('#recovery-evidence-list [data-recovery-evidence-role]')].map(row=>({role:row.dataset.recoveryEvidenceRole,status:row.dataset.recoveryEvidenceStatus,path:row.querySelector('input').value}))")
        assert evidence and all(row['status'] in ('Verified', 'AbsentExpected') for row in evidence)
        assert any(row['role'] == 'source_root' and row['status'] == ('AbsentExpected' if removed else 'Verified') for row in evidence)
        assert any(row['role'] == 'target_root' and row['status'] == 'Verified' for row in evidence)
        assert any(row['role'] == 'journal_completion' and row['status'] == 'Verified' for row in evidence)
        progress = self.script("return document.querySelector('#recovery-busy').textContent")
        assert f"{case['file_count']} מתוך {case['file_count']}" in progress and 'MiB' in progress and 'שניות' in progress
        if not self.script("return document.querySelector('#recovery-evidence').open"):
            self.click('#recovery-evidence > summary')
        self.capture('recovery-file-album-' + case['case'])
        assert tree_snapshot(protected_source) == before_source and tree_snapshot(target) == before_target
        assert named_streams(target, not copying) == before_streams and journal_snapshot(journals) == before_journals
        self.click('#recovery-close')
        assert self.choose(source) == job_id
        assert self.script("return document.querySelector('#recovery-inspect').disabled")
        assert self.script("return document.querySelector('#recovery-source-location').value") == 'present'
        if removed:
            self.script("document.querySelector('#recovery-source-location').value='removed';document.querySelector('#recovery-source-location').dispatchEvent(new Event('change',{bubbles:true}));")
        self.pick('#recovery-source-picker', source.parent if removed else source); self.idle()
        self.pick('#recovery-target-picker', target); self.idle()
        changed = None
        if case['case'] == 'mixed-move':
            destination = next(target.rglob('*.mp3'))
            metadata = destination.stat()
            with Path(str(destination) + ':notes').open('ab') as stream:
                stream.write(b'owned-album-restart-stream-change')
            os.utime(destination, ns=(metadata.st_atime_ns, metadata.st_mtime_ns))
            changed = {'tree': tree_snapshot(target), 'streams': named_streams(target, True)}
            self.inspect()
            assert self.script("return document.querySelector('#recovery-result-title').textContent") == 'נדרשת בדיקה נוספת'
            assert tree_snapshot(target) == changed['tree'] and named_streams(target, True) == changed['streams']
            assert tree_snapshot(protected_source) == before_source
            self.capture('recovery-file-album-changed-stream')
        elif removed:
            source.mkdir()  # Disposable foreign object at the recorded old leaf.
            changed = tree_snapshot(protected_source)
            self.inspect()
            assert not self.script("return document.querySelector('#recovery-error').hidden")
            assert self.script("return document.querySelector('#recovery-report').hidden")
            assert tree_snapshot(protected_source) == changed and tree_snapshot(target) == before_target
            self.capture('recovery-file-album-foreign-source-refused')
        else:
            self.inspect()
            assert self.script("return document.querySelector('#recovery-result-title').textContent") == 'הקבצים שנבדקו תואמים ליומן'
            assert tree_snapshot(protected_source) == before_source and tree_snapshot(target) == before_target
        assert journal_snapshot(journals) == before_journals
        assert {path.name: file_hash(path) for path in journals.glob(job_id + '.*.jsonl')} == hashes
        self.click('#recovery-close')
        self.cases.append(dict(case=case['case'], job_id=job_id, source=str(source), target=str(target),
            journal_sha256=hashes, observed_file_rows=observed_file_rows, measured_progress=progress,
            scope_evidence=evidence, source_before=before_source, target_before=before_target,
            journals_before=before_journals, streams_before=before_streams, owned_external_change=changed,
            ipc_mocked=False))

    def finish(self):
        if sorted(row['case'] for row in self.cases) != ['copy', 'mixed-move', 'root-move']:
            raise ValueError('All three actual file-Album restart scopes are mandatory')
        (self.output / 'album-restart-recovery.json').write_text(json.dumps(
            dict(status='passed', checks=sorted(CHECKS), cases=self.cases, ipc_mocked=False),
            ensure_ascii=False, indent=2), encoding='utf-8')
        return sorted(CHECKS)
