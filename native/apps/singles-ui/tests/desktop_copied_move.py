"""Actual UI copied-Move proof on owned NTFS files, including named streams."""
import hashlib
import json
import os
import re
from pathlib import Path
import tempfile
import wave

from desktop_modes import select_sort_setup
from desktop_ai import write_wave
from desktop_albums import snapshot
from desktop_polish import set_developer_mode
from desktop_recovery import journal_snapshot

DUET = 'ישראל כהן וחיים לוי - שיר חדש.wav'
SINGLE = 'אברהם פריד - שיר בודד.wav'
ARTISTS = ('ישראל כהן', 'חיים לוי')
AUXILIARY = b'owned auxiliary'
CHECKS = {'actual-one-eligible-duet-move', 'actual-zero-eligible-duet-source-retained', 'actual-cross-volume-move', 'actual-original-ai-duet-move', 'actual-mixed-duet-and-rename',
          'copied-move-main-and-named-streams', 'copied-move-final-source-absence',
          'copied-move-preview-and-review-no-effects', 'copied-move-stale-stream-no-effects',
          'copied-move-stale-target-no-effects', 'copied-move-cancel-before-effects'}


def digest(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def auxiliary(path):
    return Path(str(path) + ':notes')


def large_wave(path):
    # Real ordinary allocated bytes. The fresh Apply hashes give the UI time to
    # deliver cancellation without introducing a backend hook or a blocked writer.
    with wave.open(str(path), 'wb') as stream:
        stream.setnchannels(1); stream.setsampwidth(2); stream.setframerate(8000)
        chunk = bytes(1024 * 1024)
        for _ in range(512):
            stream.writeframesraw(chunk)


def exercise_copied_move(script, click, fill, wait, capture, root, output, second_volume_parent):
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
    root = Path(root); root.mkdir(); receipts = []
    selected = json.loads((Path(output) / 'selected-execution.json').read_text(encoding='utf-8'))
    journals = {case['journal_path'] for case in selected['cases'] if case.get('journal_path')}
    assert selected['status'] == 'passed' and len(journals) == 1
    journal_root = Path(journals.pop()); assert journal_root.is_dir()
    # The second volume is the existing CI checkout volume; never mount a drive
    # or change filesystem/security settings to manufacture the capability.
    with tempfile.TemporaryDirectory(prefix='singles-ui-copied-target-', dir=second_volume_parent) as other:
        other = Path(other)
        assert root.stat().st_dev != other.stat().st_dev, 'CI must supply two actual volumes'
        for label in ('cross-volume', 'duet', 'mixed', 'one-eligible', 'zero-eligible', 'stale-stream', 'stale-target', 'cancel'):
            case = root / label; source = case / 'source'; source.mkdir(parents=True)
            cross_volume = label in ('cross-volume', 'stale-stream', 'cancel')
            target = (other / label) if cross_volume else case / 'target'; target.mkdir()
            configured = label in ('duet', 'mixed', 'one-eligible', 'zero-eligible', 'stale-target')
            if label == 'one-eligible': (target / ARTISTS[0]).mkdir()
            paths = [source / DUET] if configured else [source / SINGLE]
            if label == 'mixed': paths.append(source / SINGLE)
            for path in paths:
                (large_wave if label == 'cancel' else write_wave)(path)
                auxiliary(path).write_bytes(AUXILIARY)
                os.utime(path, ns=(1_700_000_000_000_000_000, 1_700_000_000_000_000_000))
            before = {path.name: digest(path) for path in paths}
            source_before, target_before = snapshot(source), snapshot(target)
            identities = {path.name: path.stat().st_ino for path in paths}
            if script("return document.querySelector('#app').dataset.view==='results'"):
                click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
            select_sort_setup(script, click, wait)
            fill('#source', str(source)); fill('#target', str(target))
            for key, value in dict(copy_mode=False, main_folder_only=True, singles_folder=False, abc_sort=False, exist_only=label in ('one-eligible', 'zero-eligible')).items():
                if script(f"return document.querySelector('#{key}').checked") != value: click('#'+key)
            click('#advanced'); click('#duet-all' if configured else '#duet-first')
            wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
            profile = 'Configured' if configured else 'Disabled'
            script(f"document.querySelector('#ai-profile').value='{profile}';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{{bubbles:true}}));")
            click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
            journal_before = journal_snapshot(journal_root)
            click('#tab-mode-sort')
            click('#sort')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
            assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), label
            assert snapshot(source) == source_before and snapshot(target) == target_before
            assert all(auxiliary(path).read_bytes() == AUXILIARY for path in paths)
            assert journal_snapshot(journal_root) == journal_before
            click('#review-apply')
            if label == 'zero-eligible':
                assert 'קובצי המקור יישארו במקומם' in script("return document.querySelector('#apply-description').textContent")
            else:
                assert 'המקור יוסר רק לאחר אימות כל היעדים' in script("return document.querySelector('#apply-description').textContent")
            capture('copied-move-review-'+label); click('#apply-back')
            assert snapshot(source) == source_before and snapshot(target) == target_before
            if label == 'stale-stream':
                auxiliary(paths[0]).write_bytes(b'changed stream!')
                os.utime(paths[0], ns=(1_700_000_000_000_000_000, 1_700_000_000_000_000_000))
                source_before = snapshot(source)
            elif label == 'stale-target':
                (target / ARTISTS[0]).mkdir()
                (target / ARTISTS[0] / DUET).write_bytes(b'owned collision after preview')
                target_before = snapshot(target)
            click('#review-apply'); click('#confirm-apply')
            if label == 'cancel':
                click('#cancel-apply')
                wait(lambda: script("return document.querySelector('#execution-title').textContent==='הביצוע נעצר'"), timeout=120)
            elif label.startswith('stale-'):
                wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
            else:
                wait(lambda: script("return document.querySelector('#execution-title').textContent==='הביצוע הושלם'"), timeout=120)
            if label.startswith('stale-') or label == 'cancel':
                assert snapshot(source) == source_before and snapshot(target) == target_before
                expected_auxiliary = b'changed stream!' if label == 'stale-stream' else AUXILIARY
                assert all(auxiliary(path).read_bytes() == expected_auxiliary for path in paths)
                capture('copied-move-refused-'+label)
                summary = script("return document.querySelector('#execution-summary').textContent")
                assert 'קבצים הועברו' not in summary and 'יעדים נכתבו' not in summary
                receipts.append(dict(case=label, source_and_target_unchanged=True, auxiliary_unchanged=True,
                                     observed_file_effects=dict(new_destinations=0, retired_sources=0),
                                     native_result_counters=None, displayed_summary=summary, ipc_mocked=False))
                continue
            if label == 'zero-eligible':
                assert snapshot(source) == source_before and snapshot(target) == target_before
                assert journal_snapshot(journal_root) == journal_before
                assert all(auxiliary(path).read_bytes() == AUXILIARY and path.stat().st_ino == identities[path.name] for path in paths)
                assert script("return document.querySelector('#execution-summary').textContent") == 'לא בוצעו שינויים. קובצי המקור נשמרו.'
                assert script("return document.querySelector('#execution-recovery').hidden")
                assert script("return document.querySelectorAll('#rows tr').length") == 0
                click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
                assert script("return document.querySelectorAll('#rows tr').length") == 1
                assert 'ללא פעולה' in script("return document.querySelector('#rows').textContent")
                capture('copied-move-zero-eligible-retained')
                click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
                set_developer_mode(script, click, wait, True)
                details = script("return document.querySelector('#execution-summary').textContent")
                assert details == '0 יעדים נכתבו · 0 קבצי מקור הועברו · 0 נכשלו · 0 לא בוצעו'
                set_developer_mode(script, click, wait, False)
                receipts.append(dict(case=label, source=str(source), target=str(target), source_and_target_unchanged=True,
                                     journal_unchanged=True, auxiliary_unchanged=True, source_identity_retained=True,
                                     observed_displayed_counts=dict(committed_destinations=0, moved_sources=0, failed_operations=0, blocked_operations=0),
                                     displayed_details=details, original_sha256=before, ipc_mocked=False))
                continue
            expected = {(artist, DUET) for artist in ARTISTS} if configured else {('אברהם פריד', SINGLE)}
            if label == 'one-eligible': expected = {(ARTISTS[0], DUET)}
            if label == 'mixed': expected.add(('אברהם פריד', SINGLE))
            found = {(path.parent.name, path.name) for path in target.rglob('*.wav')}
            assert found == expected, (found, expected)
            observed = []
            for artist, name in sorted(expected):
                destination = target / artist / name
                assert digest(destination) == before[name]
                assert auxiliary(destination).read_bytes() == AUXILIARY
                assert destination.stat().st_mtime_ns == 1_700_000_000_000_000_000
                if label == 'mixed' and name == SINGLE:
                    assert destination.stat().st_ino == identities[name], 'Ordinary same-volume file was copied instead of renamed'
                observed.append(dict(artist=artist, name=name, sha256=digest(destination), auxiliary_sha256=digest(auxiliary(destination))))
            assert not list(source.iterdir()), 'Original source still exists after all destinations completed'
            assert script("return document.querySelectorAll('#rows tr').length") == len(paths)
            assert script("return document.querySelector('#execution-recovery').hidden")
            summary = script("return document.querySelector('#execution-summary').textContent")
            assert 'מקובצי המקור נשמרו' not in summary
            capture('copied-move-complete-'+label)
            set_developer_mode(script, click, wait, True)
            details = script("return document.querySelector('#execution-summary').textContent")
            match = re.fullmatch(r'(\d+) יעדים נכתבו · (\d+) קבצי מקור הועברו · (\d+) נכשלו · (\d+) לא בוצעו', details)
            assert match, details
            committed, moved, failed, blocked = map(int, match.groups())
            assert (committed, moved, failed, blocked) == (len(expected), len(paths), 0, 0)
            set_developer_mode(script, click, wait, False)
            receipts.append(dict(case=label, ai_profile=profile, cross_volume=cross_volume,
                                 source=str(source), target=str(target),
                                 journal_path=script("return document.querySelector('#journal-path').value"),
                                 expected_counts=dict(source_items=len(paths), committed_destinations=len(expected), moved_sources=len(paths)),
                                 observed_displayed_counts=dict(committed_destinations=committed, moved_sources=moved, failed_operations=failed, blocked_operations=blocked),
                                 displayed_summary=summary, displayed_details=details,
                                 original_sha256=before, destinations=observed, ipc_mocked=False))
    (Path(output) / 'copied-move-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
