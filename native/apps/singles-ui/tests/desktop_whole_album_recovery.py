"""Actual whole-object Album restart checks, without inventing child transfers."""
import json
import hashlib
from pathlib import Path

from desktop_ai import tree_snapshot
from desktop_albums import ARTIST, ALBUM
from desktop_album_recovery import AlbumRecovery
from desktop_recovery import file_hash, journal_snapshot

SCOPE = 'recorded_main_hash_identity_membership'
CHECKS = {'whole-album-root-restarted-with-fresh-parent', 'whole-album-mixed-restarted-with-fresh-source',
          'whole-album-main-identity-membership-scope-visible', 'whole-album-descendants-are-inspection-rows',
          'whole-album-single-atomic-receipt-retained', 'whole-album-deferred-single-covered',
          'whole-album-parent-refusal', 'whole-album-restart-no-effects',
          'whole-album-main-content-change-detected', 'whole-album-extra-member-detected'}


def selected_whole_case(case):
    if case.get('case') not in ('root-move', 'mixed-move') or case.get('source_retained') is not False:
        raise ValueError('Whole restart requires an actual completed same-volume Album Move')
    if case.get('cross_volume') is True:
        raise ValueError('Copied cross-volume Album Move is a separate journal domain')
    if not all(case.get(key) for key in ('source', 'target', 'journal_path')) or type(case.get('file_count')) is not int or case['file_count'] < 1:
        raise ValueError('Whole restart requires retained roots and actual file count')
    return case


def require_only_owned_append(before, after, relative, expected_sha256, added_bytes):
    """The parent includes target: allow only the one intentional file append."""
    left = {row['path']: row for row in before}; right = {row['path']: row for row in after}
    if left.keys() != right.keys() or relative not in left:
        raise ValueError('Owned append changed the parent namespace')
    variable = {'modified_ns', 'win32_last_write_ticks', 'size', 'sha256'}
    for name, original in left.items():
        current = right[name]
        if name != relative:
            if current != original: raise ValueError('Owned append changed an unrelated entry')
        elif (original['directory'] or current['size'] != original['size'] + added_bytes
              or current['sha256'] != expected_sha256
              or {k: v for k, v in current.items() if k not in variable} != {k: v for k, v in original.items() if k not in variable}):
            raise ValueError('Owned append changed more than the intended bytes and write time')


class WholeAlbumRecovery(AlbumRecovery):
    expected_scope = SCOPE

    def run(self, case):
        case = selected_whole_case(case)
        source, target, journals = (Path(case[key]) for key in ('source', 'target', 'journal_path'))
        removed = case['case'] == 'root-move'
        source_scope = source.parent if removed else source
        assert source.exists() == (not removed)
        assert source_scope.stat().st_dev == target.stat().st_dev, 'Actual same-volume identity domain required'
        before_source, before_target = tree_snapshot(source_scope), tree_snapshot(target)
        self.restart(self.binary, 'whole-album-restart-' + case['case'])
        assert self.script("return document.querySelector('#source').value==='' && document.querySelector('#target').value===''")
        before_journals = journal_snapshot(journals)
        job_id = self.choose(source)
        assert self.script("return document.querySelector('#recovery-list button[aria-pressed=true]').dataset.recoveryScope") == SCOPE
        assert (journals / (job_id + '.album-move.jsonl')).is_file()
        journal_hashes = {path.name: file_hash(path) for path in journals.glob('*.jsonl')}
        assert self.script("return document.querySelector('#recovery-inspect').disabled")
        assert self.script("return document.querySelector('#recovery-source-location').value") == 'present'
        if removed:
            self.script("document.querySelector('#recovery-source-location').value='removed';document.querySelector('#recovery-source-location').dispatchEvent(new Event('change',{bubbles:true}));")
            self.pick('#recovery-source-picker', target); self.idle()
            assert not self.script("return document.querySelector('#recovery-error').hidden")
            assert self.script("return document.querySelector('#recovery-inspect').disabled")
        self.pick('#recovery-source-picker', source.parent if removed else source); self.idle()
        self.pick('#recovery-target-picker', target); self.idle()
        self.inspect()
        assert self.script("return document.querySelector('#recovery-error').hidden")
        assert self.script("return document.querySelector('#recovery-result-title').textContent") == 'הקבצים שנבדקו תואמים ליומן'
        rows = self.script("return [...document.querySelectorAll('#recovery-operation-list article')].map(row=>({status:row.dataset.recoveryOperationStatus,title:row.querySelector('h4').textContent}))")
        assert len(rows) == case['file_count']
        assert all(row['status'] == 'Verified' and row['title'].startswith('קובץ ') for row in rows)
        assert 'הפעולה רשומה ומאומתת' not in self.script("return document.querySelector('#recovery-operation-list').textContent")
        assert 'מידע נלווה שלא תועד ביומן לא נבדק' in self.script("return document.querySelector('#recovery-next-step').textContent")
        evidence = self.script("return [...document.querySelectorAll('#recovery-evidence-list [data-recovery-evidence-role]')].map(row=>({role:row.dataset.recoveryEvidenceRole,status:row.dataset.recoveryEvidenceStatus,path:row.querySelector('input').value}))")
        assert evidence and all(row['status'] in ('Verified', 'AbsentExpected') for row in evidence)
        assert len([row for row in evidence if row['role'] == 'whole_album']) == 1
        assert any(row['role'] == 'membership' and row['status'] == 'Verified' for row in evidence)
        progress = self.script("return document.querySelector('#recovery-busy').textContent")
        assert f"{case['file_count']} מתוך {case['file_count']}" in progress and 'MiB' in progress
        if not self.script("return document.querySelector('#recovery-evidence').open"):
            self.click('#recovery-evidence > summary')
        self.capture('recovery-whole-album-' + case['case'])
        assert tree_snapshot(source_scope) == before_source and tree_snapshot(target) == before_target
        assert journal_snapshot(journals) == before_journals
        if removed:
            changed_file = next(target.rglob('*.mp3'))
            added = b'owned-whole-album-restart-main-change'
            with changed_file.open('rb') as stream:
                expected = hashlib.file_digest(stream, 'sha256')
            expected.update(added)
            with changed_file.open('ab') as stream:
                stream.write(added)
            changed_source_scope = tree_snapshot(source_scope)
            require_only_owned_append(before_source, changed_source_scope,
                changed_file.relative_to(source_scope).as_posix(), expected.hexdigest(), len(added))
        else:
            album = target / ARTIST / ALBUM
            assert album.is_dir()
            (album / 'owned-extra-empty-directory').mkdir()
            changed_source_scope = before_source
        changed_target = tree_snapshot(target)
        self.inspect()
        assert self.script("return document.querySelector('#recovery-result-title').textContent") == 'נדרשת בדיקה נוספת'
        if not removed:
            assert self.script("return [...document.querySelectorAll('#recovery-evidence-list [data-recovery-evidence-role=membership]')].some(row=>row.dataset.recoveryEvidenceStatus==='Changed')")
        assert source.exists() == (not removed)
        assert tree_snapshot(source_scope) == changed_source_scope and tree_snapshot(target) == changed_target
        assert journal_snapshot(journals) == before_journals
        assert {path.name: file_hash(path) for path in journals.glob('*.jsonl')} == journal_hashes
        self.capture('recovery-whole-album-changed-' + case['case'])
        self.click('#recovery-close')
        self.cases.append(dict(case=case['case'], job_id=job_id, observed_file_rows=rows,
            scope=SCOPE, scope_evidence=evidence, measured_progress=progress,
            source_before=before_source, target_before=before_target, journals_before=before_journals,
            journal_sha256=journal_hashes, owned_changed_target=changed_target,
            owned_changed_source_scope=changed_source_scope, ipc_mocked=False))

    def finish(self):
        if sorted(row['case'] for row in self.cases) != ['mixed-move', 'root-move']:
            raise ValueError('Both actual whole-object Album restart scopes are mandatory')
        (self.output / 'whole-album-restart-recovery.json').write_text(json.dumps(
            dict(status='passed', checks=sorted(CHECKS), cases=self.cases, ipc_mocked=False),
            ensure_ascii=False, indent=2), encoding='utf-8')
        return sorted(CHECKS)
