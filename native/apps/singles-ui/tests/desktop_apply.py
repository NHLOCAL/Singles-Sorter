"""Real Windows IPC execution acceptance, exclusively disposable generated audio.

Only UI interactions are used. No invoke replacement, backend test hook, arbitrary
user folder, or rollback assertion. Independent Python hashes verify each effect.
"""
import hashlib
import json
from pathlib import Path
from desktop_ai import write_wave


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def exercise_apply(script, click, fill, wait, capture, root, output):
    root = Path(root)
    root.mkdir()  # Refuse reused trees.
    receipts = []

    def setup(source, target, copy):
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        fill('#source', str(source)); fill('#target', str(target))
        desired = dict(copy_mode=copy, main_folder_only=False, singles_folder=True,
                       abc_sort=False, exist_only=False)
        for key, value in desired.items():
            if script(f"return document.querySelector('#{key}').checked") != value:
                click('#' + key)
        click('#advanced')
        click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        # The selected-root lane uses a feature-off binary. No model substitution.
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog')
        click('#sort')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results'"))
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), 'Native backend did not authorize the real owned plan'

    for copy in (True, False):
        case = root / ('copy' if copy else 'move')
        source, target = case / 'מקור', case / 'יעד'
        source.mkdir(parents=True); target.mkdir()
        paths = [source / f'אברהם פריד - בדיקת ביצוע {index}.wav' for index in range(2)]
        for path in paths:
            write_wave(path)
        before = {path.name: digest(path) for path in paths}
        setup(source, target, copy)
        assert not list(target.iterdir()), 'Preview changed the selected target'
        assert {p.name: digest(p) for p in paths} == before
        click('#review-apply')
        wait(lambda: script("return document.querySelector('#apply-dialog').open"))
        assert script("return document.querySelector('#apply-source').value") == str(source)
        assert script("return document.querySelector('#apply-target').value") == str(target)
        capture('apply-review-' + ('copy' if copy else 'move'))
        click('#apply-back')
        assert not list(target.iterdir()), 'Dismissed review must have no effects'
        click('#review-apply'); click('#confirm-apply')
        wait(lambda: script("return document.querySelector('#execution-title').textContent==='הביצוע הושלם'"), timeout=120)
        assert script("return document.querySelector('#review-apply').hidden"), 'One-shot plan offered a second apply'
        destinations = list(target.rglob('*.wav'))
        assert len(destinations) == 2
        assert {p.name: digest(p) for p in destinations} == before
        assert all(p.parent.name == 'סינגלים' and p.parent.parent.name == 'אברהם פריד' for p in destinations)
        if copy:
            assert {p.name: digest(p) for p in paths} == before
        else:
            assert not list(source.rglob('*.wav'))
        journal_path = script("return document.querySelector('#journal-path').value")
        assert journal_path and Path(journal_path).exists()
        capture('apply-complete-' + ('copy' if copy else 'move'))
        receipts.append(dict(copy_mode=copy, source=str(source), target=str(target),
                             source_sha256=before, destination_sha256={p.name:digest(p) for p in destinations},
                             journal_path=journal_path, source_retained=copy, ipc_mocked=False))
    # A fresh plan must detect a content change after inspection, even if the
    # filename is unchanged. No destination can be written after this rejection.
    case = root / 'stale'; source, target = case / 'source', case / 'target'
    source.mkdir(parents=True); target.mkdir()
    path = source / 'אברהם פריד - שונה.wav'; write_wave(path)
    setup(source, target, True)
    with path.open('ab') as stream:
        stream.write(b'changed-after-inspection')
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"))
    assert not list(target.iterdir()), 'Stale source caused a physical transfer'
    capture('apply-stale-plan-rejected')
    receipts.append(dict(case='stale_source_rejected', target_unchanged=True, ipc_mocked=False))
    (Path(output) / 'selected-execution.json').write_text(json.dumps(dict(status='passed', cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return ['actual-multi-file-copy', 'actual-same-volume-move', 'artist-and-singles-folders', 'review-dismiss-no-effects', 'stale-plan-no-effects', 'one-shot-no-repeat']
