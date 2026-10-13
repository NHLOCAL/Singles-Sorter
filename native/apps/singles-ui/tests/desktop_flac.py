"""Owned FLAC five-field Fix acceptance through the actual desktop and TagLib.

Fixture construction is separate from the Rust transformer. The bounded reader
below independently checks exact comments, metadata blocks and compressed audio.
"""
import importlib.util
import json
from pathlib import Path

from desktop_modes import select_sort_setup
from desktop_albums import snapshot
from desktop_polish import set_developer_mode
from desktop_repair import assert_observed_filename, digest

NATIVE = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('owned_flac_fixture', NATIVE / 'tests/staged_flac/verify_fixture_contract.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
FIELDS = ('artist', 'albumartist', 'title', 'album', 'genre')
UNKNOWN = ['x-Custom=one', 'X-custom=two', 'TRACKNUMBER=03']
CHECKS = {'actual-fix-flac-five-fields', 'actual-fix-flac-shifted-audio',
          'flac-original-backup-retained', 'flac-picture-vendor-unknowns-preserved',
          'flac-opaque-offset-partial', 'flac-unchanged-hidden', 'flac-stale-plan-no-effects'}


def generated_flac(*, duplicate=False, opaque=False, unchanged=False):
    values = UNKNOWN.copy()
    for key in FIELDS:
        values.append(f"{key.upper()}={'שלום' if unchanged else 'à'}")
        if duplicate:
            values.append(f'{key}=á')
    blocks = [(0, fixture.BASE[8:42]), (4, fixture.comments(values)), *fixture.EXTRAS]
    if opaque:
        blocks.append((2, b'testprivate offsets unverified'))
    return b'fLaC' + b''.join(fixture.block(kind, body, i + 1 == len(blocks))
                              for i, (kind, body) in enumerate(blocks)) + fixture.AUDIO


def inspect_flac(data):
    assert data[:4] == b'fLaC'
    at = 4; blocks = []
    while True:
        assert at + 4 <= len(data)
        end = at + 4 + int.from_bytes(data[at + 1:at + 4], 'big')
        assert end <= len(data)
        blocks.append(data[at:end]); at = end
        if blocks[-1][0] & 128:
            break
    comments = [block[4:] for block in blocks if block[0] & 127 == 4]
    assert len(comments) == 1 and at < len(data)
    body = comments[0]; cursor = 0

    def word():
        nonlocal cursor
        assert cursor + 4 <= len(body)
        length = int.from_bytes(body[cursor:cursor + 4], 'little'); cursor += 4
        return length

    def vector():
        nonlocal cursor
        length = word()
        assert cursor + length <= len(body)
        value = body[cursor:cursor + length]; cursor += length
        return value

    vendor = vector(); count = word()
    assert count <= len(body) // 4
    values = [vector().decode('utf-8') for _ in range(count)]
    assert cursor == len(body)
    return vendor, values, blocks, at, data[at:]


def exercise_flac(script, click, fill, wait, capture, root, output):
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
    root = Path(root); root.mkdir()
    receipts = []

    def setup(source):
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        select_sort_setup(script, click, wait)
        fill('#source', str(source)); fill('#target', '')
        click('#advanced'); click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog')
        set_developer_mode(script, click, wait, False)
        click('#tab-mode-fix')
        click('#fix')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled")

    for duplicate, opaque, label in [(False, False, 'equal-size'), (True, False, 'shifted-audio'), (True, True, 'opaque-offset')]:
        source = root / label; source.mkdir()
        path = source / 'repair_input.flac'; fixed = source / 'repair input.flac'
        before = generated_flac(duplicate=duplicate, opaque=opaque); path.write_bytes(before)
        unchanged = source / 'unchanged.flac'; unchanged_bytes = generated_flac(unchanged=True)
        unchanged.write_bytes(unchanged_bytes)
        source_before = snapshot(source)
        vendor, _, blocks, offset, audio = inspect_flac(before)
        setup(source)
        assert snapshot(source) == source_before
        click('#review-apply')
        assert script("return document.querySelector('#apply-target-label').hidden")
        capture('flac-review-' + label); click('#apply-back')
        assert snapshot(source) == source_before
        click('#review-apply'); click('#confirm-apply')
        expected_title = 'הביצוע הושלם חלקית' if opaque else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == expected_title, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert not path.exists() and fixed.is_file()
        assert unchanged.read_bytes() == unchanged_bytes
        after = fixed.read_bytes(); backups = list(source.glob('*.backup'))
        if opaque:
            assert after == before and not backups
            assert set(source.iterdir()) == {fixed, unchanged}
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
            assert 'תגיות שלא תוקנו' in script("return document.querySelector('#execution-summary').textContent")
        else:
            assert len(backups) == 1 and backups[0].read_bytes() == before
            assert set(source.iterdir()) == {fixed, unchanged, backups[0]}
            new_vendor, values, new_blocks, new_offset, new_audio = inspect_flac(after)
            replacement = 'א, ב' if duplicate else 'א'
            assert values == UNKNOWN + [f'{key}={replacement}' for key in FIELDS]
            assert new_vendor == vendor == fixture.VENDOR
            assert [b for b in blocks if b[0] & 127 != 4] == [b for b in new_blocks if b[0] & 127 != 4]
            assert new_audio == audio == fixture.AUDIO
            assert (new_offset < offset) if duplicate else (new_offset == offset)
        assert not script("return document.querySelector('#show-inactive').checked")
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert_observed_filename(script, fixed)
        assert 'unchanged.flac' not in script("return document.querySelector('#rows').textContent")
        assert '\u2014' not in script("return document.querySelector('#rows').textContent")
        capture('flac-result-' + label)
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        capture('flac-result-including-unchanged-' + label)
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert script("return document.querySelector('#review-apply').hidden")
        receipts.append(dict(case=label, original_sha256=digest(before), current_sha256=digest(after),
                             displayed_filename=fixed.name, original_bytes=len(before), current_bytes=len(after), media_sha256=digest(audio),
                             backup_sha256=[digest(p.read_bytes()) for p in backups],
                             unchanged_sha256=digest(unchanged_bytes), renamed=True,
                             tags_repaired=not opaque, inactive_hidden_by_default=True, ipc_mocked=False))
    source = root / 'stale'; source.mkdir(); path = source / 'stale_input.flac'
    path.write_bytes(generated_flac(duplicate=True)); setup(source)
    path.write_bytes(generated_flac()); changed = snapshot(source)
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert snapshot(source) == changed
    capture('flac-stale-plan-rejected')
    receipts.append(dict(case='stale', source_unchanged=True, ipc_mocked=False))
    (Path(output) / 'flac-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
