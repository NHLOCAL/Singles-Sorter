"""Actual desktop Ogg Fix proof, using owned recordings and independent packets."""
import base64
import importlib.util
import json
from pathlib import Path
from desktop_albums import snapshot
from desktop_flac import fixture as flac_fixture
from desktop_polish import set_developer_mode
from desktop_repair import digest

NATIVE = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('owned_ogg_fixture', NATIVE / 'tests/staged_ogg/verify_fixture_contract.py')
fixture = importlib.util.module_from_spec(spec); spec.loader.exec_module(fixture)
FIELDS = ('artist', 'albumartist', 'title', 'album', 'genre')
UNKNOWN = ['x-Custom=one', 'METADATA_BLOCK_PICTURE=' + base64.b64encode(flac_fixture.picture()).decode('ascii'),
           'X-custom=two', 'TRACKNUMBER=03', 'ALBUM ARTIST=keep alias']
CHECKS = {'actual-fix-vorbis-five-fields', 'actual-fix-opus-five-fields',
          'ogg-encoded-audio-and-timing-preserved', 'ogg-artwork-vendor-unknowns-preserved',
          'ogg-original-backup-retained', 'ogg-chained-refused', 'ogg-unchanged-hidden'}


def generated_ogg(opus, *, duplicate=False, unchanged=False):
    base = (fixture.MEDIA / ('blank.opus' if opus else 'blank.ogg')).read_bytes()
    values = UNKNOWN.copy()
    for key in FIELDS:
        values.append(f"{key.upper()}={'שלום' if unchanged else 'à'}")
        if duplicate: values.append(f'{key}=á')
    suffix = b'\x01opaque\0\xff' if opus else b'\x81\0padding'
    return fixture.replace_headers(base, fixture.comment(opus, values, suffix))


def inspect_ogg(data):
    pages = fixture.pages(data); packets = fixture.packets(data)
    opus = packets[0].startswith(b'OpusHead')
    prefix = b'OpusTags' if opus else b'\x03vorbis'
    body = packets[1]; assert body.startswith(prefix)
    cursor = len(prefix)
    def word():
        nonlocal cursor
        assert cursor + 4 <= len(body)
        size = int.from_bytes(body[cursor:cursor + 4], 'little'); cursor += 4
        return size
    def vector():
        nonlocal cursor
        size = word(); assert cursor + size <= len(body)
        value = body[cursor:cursor + size]; cursor += size
        return value
    vendor = vector(); count = word(); assert count <= len(body) // 4
    values = [vector().decode('utf-8') for _ in range(count)]
    suffix = body[cursor:]
    required = 2 if opus else 3; complete = 0; timing = []
    for flags, granule, serial, _, laces, payload, _ in pages:
        if complete >= required:
            timing.append((flags, granule, serial, bytes(laces), payload))
        else:
            complete += sum(size < 255 for size in laces)
            assert complete <= required  # No mixed header/audio pages in these fixtures.
    assert timing
    return vendor, values, suffix, packets, timing


def verify_repaired(before, after, duplicate):
    vendor, _, suffix, packets, timing = inspect_ogg(before)
    new_vendor, values, new_suffix, new_packets, new_timing = inspect_ogg(after)
    replacement = 'א, ב' if duplicate else 'א'
    assert values == UNKNOWN + [f'{key}={replacement}' for key in FIELDS]
    assert (new_vendor, new_suffix) == (vendor, suffix)
    assert new_packets[0] == packets[0] and new_packets[2:] == packets[2:]
    assert new_timing == timing
    assert len(after) <= len(before) + 27
    if duplicate: assert len(after) < len(before)


def exercise_ogg(script, click, fill, wait, capture, root, output):
    assert __debug__, 'Physical acceptance requires enabled assertions'
    root = Path(root); root.mkdir(); receipts = []
    for opus, duplicate, chained in [(False, False, False), (True, True, False), (False, True, True), (True, True, True)]:
        extension = 'opus' if opus else 'ogg'; label = extension + ('-chained' if chained else '-repair')
        source = root / label; source.mkdir()
        path = source / f'repair_input.{extension}'; fixed = source / f'repair input.{extension}'
        before = generated_ogg(opus, duplicate=duplicate)
        if chained: before += before
        path.write_bytes(before)
        unchanged = source / f'unchanged.{extension}'; unchanged_bytes = generated_ogg(opus, unchanged=True)
        unchanged.write_bytes(unchanged_bytes); original = snapshot(source)
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        fill('#source', str(source)); fill('#target', '')
        click('#advanced'); click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
        click('#fix')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
        assert snapshot(source) == original
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled")
        click('#review-apply'); capture('ogg-review-' + label); click('#apply-back')
        assert snapshot(source) == original
        click('#review-apply'); click('#confirm-apply')
        expected = 'הביצוע הושלם חלקית' if chained else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == expected, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert fixed.is_file() and not path.exists()
        after = fixed.read_bytes(); backups = list(source.glob('*.backup'))
        if chained:
            assert after == before and not backups
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
        else:
            verify_repaired(before, after, duplicate)
            assert len(backups) == 1 and backups[0].read_bytes() == before
        assert set(source.iterdir()) == {fixed, unchanged, *backups}
        assert unchanged.read_bytes() == unchanged_bytes
        assert not script("return document.querySelector('#show-inactive').checked")
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert '\u2014' not in script("return document.querySelector('#rows').textContent")
        capture('ogg-result-' + label)
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelector('#review-apply').hidden")
        receipts.append(dict(case=label, original_sha256=digest(before), current_sha256=digest(after),
                             original_bytes=len(before), current_bytes=len(after),
                             backup_sha256=[digest(p.read_bytes()) for p in backups],
                             tags_repaired=not chained, unchanged_sha256=digest(unchanged_bytes), ipc_mocked=False))
    (Path(output) / 'ogg-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
