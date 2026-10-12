"""Actual UI copied-Move proof on owned NTFS files, including named streams."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import wave

from desktop_ai import write_wave
from desktop_albums import snapshot
from desktop_polish import set_developer_mode

DUET = 'ישראל כהן וחיים לוי - שיר חדש.wav'
SINGLE = 'אברהם פריד - שיר בודד.wav'
ARTISTS = ('ישראל כהן', 'חיים לוי')
AUXILIARY = b'owned auxiliary'
CHECKS = {'actual-cross-volume-move', 'actual-original-ai-duet-move', 'actual-mixed-duet-and-rename',
          'copied-move-main-and-named-streams', 'copied-move-source-retired-once',
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
    # The second volume is the existing CI checkout volume; never mount a drive
    # or change filesystem/security settings to manufacture the capability.
    with tempfile.TemporaryDirectory(prefix='singles-ui-copied-target-', dir=second_volume_parent) as other:
        other = Path(other)
        assert root.stat().st_dev != other.stat().st_dev, 'CI must supply two actual volumes'
        for label in ('cross-volume', 'duet', 'mixed', 'stale-stream', 'stale-target', 'cancel'):
            case = root / label; source = case / 'source'; source.mkdir(parents=True)
            cross_volume = label in ('cross-volume', 'stale-stream', 'cancel')
            target = (other / label) if cross_volume else case / 'target'; target.mkdir()
            configured = label in ('duet', 'mixed', 'stale-target')
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
            fill('#source', str(source)); fill('#target', str(target))
            for key, value in dict(copy_mode=False, main_folder_only=True, singles_folder=False, abc_sort=False, exist_only=False).items():
                if script(f"return document.querySelector('#{key}').checked") != value: click('#'+key)
            click('#advanced'); click('#duet-all' if configured else '#duet-first')
            wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
            profile = 'Configured' if configured else 'Disabled'
            script(f"document.querySelector('#ai-profile').value='{profile}';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{{bubbles:true}}));")
            click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
            click('#sort')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
            assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), label
            assert snapshot(source) == source_before and snapshot(target) == target_before
            assert all(auxiliary(path).read_bytes() == AUXILIARY for path in paths)
            click('#review-apply')
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
                receipts.append(dict(case=label, source_and_target_unchanged=True, auxiliary_unchanged=True, ipc_mocked=False))
                continue
            expected = {(artist, DUET) for artist in ARTISTS} if configured else {('אברהם פריד', SINGLE)}
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
            assert not list(source.iterdir()), 'Original source must retire once only after all destinations'
            assert script("return document.querySelectorAll('#rows tr').length") == len(paths)
            assert script("return document.querySelector('#execution-recovery').hidden")
            capture('copied-move-complete-'+label)
            receipts.append(dict(case=label, ai_profile=profile, cross_volume=cross_volume, source_items=len(paths),
                                 committed_destinations=len(expected), moved_sources=len(paths),
                                 original_sha256=before, destinations=observed, ipc_mocked=False))
    (Path(output) / 'copied-move-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
