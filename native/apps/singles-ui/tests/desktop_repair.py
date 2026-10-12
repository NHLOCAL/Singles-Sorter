"""Actual source-only FixNames UI acceptance on owned generated MP3 fixtures.

The independent parser checks physical ID3 fields, media offsets, unknown frames,
and retained original bytes. It is deliberately not a general-purpose tag reader.
"""
import hashlib
import json
from pathlib import Path
from desktop_modes import select_sort_setup
from desktop_ai import write_wave
from desktop_polish import set_developer_mode
from desktop_stability import exercise_shell

FIELDS = {b'TPE1': ('àáâ', 'אבג'), b'TPE2': ('äåæ', 'הוז'),
          b'TIT2': ('çèé', 'חטי'), b'TALB': ('ëìî', 'כלמ'), b'TCON': ('ðñò', 'נסע')}
PRIVATE = b'owned-fixture-owner\x00opaque\x00\xff\x10payload'
CHECKS = {'actual-fix-id3v23', 'actual-fix-id3v24', 'fix-original-backup-retained',
          'fix-media-and-unknown-frame-preserved', 'fix-source-only-confirmation',
          'fix-unsupported-tags-partial', 'fix-stale-plan-no-effects',
          'fix-terminal-inactive-filter', 'fix-inactive-independent-of-developer',
          'fix-terminal-footer-stability'}


def sync_size(number):
    assert 0 <= number < 1 << 28
    return bytes((number >> 21, (number >> 14) & 127, (number >> 7) & 127, number & 127))


def read_size(data):
    assert len(data) == 4 and all(byte < 128 for byte in data)
    return sum(byte << shift for byte, shift in zip(data, (21, 14, 7, 0)))


def generated_mp3(audio, version, unsupported=False):
    assert version in (3, 4) and audio[:2] == b'\xff\xfb'
    body = bytearray()
    frames = []
    for name, (original, _) in FIELDS.items():
        payload = b'\x00' + original.encode('latin1') if unsupported else (
            b'\x01' + original.encode('utf-16') if version == 3 else b'\x03' + original.encode('utf-8'))
        frames.append((name, payload))
    frames.append((b'PRIV', PRIVATE))
    for name, payload in frames:
        body.extend(name + (sync_size(len(payload)) if version == 4 else len(payload).to_bytes(4, 'big')) + b'\x00\x00' + payload)
    if not unsupported:
        body.extend(bytes(256))
    return b'ID3' + bytes((version, 0, 0)) + sync_size(len(body)) + body + audio


def inspect_mp3(data):
    assert data[:3] == b'ID3' and data[3] in (3, 4) and data[4:6] == b'\x00\x00'
    end = 10 + read_size(data[6:10]); at = 10; fields = {}; raw = {}
    assert end <= len(data)
    while at < end and data[at]:
        name = data[at:at + 4]
        size = read_size(data[at + 4:at + 8]) if data[3] == 4 else int.from_bytes(data[at + 4:at + 8], 'big')
        assert at + 10 + size <= end and data[at + 8:at + 10] == b'\x00\x00' and name not in raw
        payload = data[at + 10:at + 10 + size]; raw[name] = data[at:at + 10 + size]
        if name in FIELDS:
            encoding = {0: 'latin1', 1: 'utf-16', 2: 'utf-16-be', 3: 'utf-8'}[payload[0]]
            fields[name] = payload[1:].decode(encoding).rstrip('\x00')
        at += 10 + size
    assert not any(data[at:end]) and set(fields) == set(FIELDS)
    return fields, raw, end, data[end:]


def digest(data):
    return hashlib.sha256(data).hexdigest()


REPAIR_PARTIAL_CAUTION = 'בדוק את הקבצים והגיבויים בתיקיית המקור לפני ניסיון נוסף.'


def assert_repair_terminal_notice(script, partial):
    """Expected unsupported tags require the ordinary caution, not an all-clear."""
    actual = script("""return (() => {
      const notice=document.querySelector('#execution-recovery');
      let visible=!notice.hidden&&notice.getClientRects().length>0;
      for(let node=notice;node;node=node.parentElement){
        const style=getComputedStyle(node);
        if(style.visibility!=='visible'||Number(style.opacity)===0)visible=false;
      }
      return {hidden:notice.hidden,visible,text:notice.textContent,
        title:document.querySelector('#execution-title').textContent};
    })();""")
    expected_title = 'הביצוע הושלם חלקית' if partial else 'הביצוע הושלם'
    if (not isinstance(actual, dict) or type(actual.get('hidden')) is not bool
            or type(actual.get('visible')) is not bool or not isinstance(actual.get('text'), str)
            or actual.get('title') != expected_title or actual['hidden'] != (not partial)
            or actual['visible'] != partial
            or (partial and actual['text'] != REPAIR_PARTIAL_CAUTION)):
        raise ValueError(f'Repair terminal caution differs: partial={partial!r}, actual={actual!r}')
    return actual


def assert_observed_filename(script, fixed):
    """Actual ordinary result must match the independently verified current file."""
    assert Path(fixed).is_file()
    name = Path(fixed).name
    assert script("return Array.from(document.querySelectorAll('#rows .source-name'),n=>n.textContent)") == [name]
    assert script("return document.querySelector('#rows .source-name').title") == name
    assert script("return document.querySelector('#rows .detail-button').getAttribute('aria-label')") == 'פרטים: ' + name


def exercise_repair(command, script, click, fill, wait, capture, root, output, audio_path):
    root = Path(root); root.mkdir()  # Never reuse another test's tree.
    audio = Path(audio_path).read_bytes()
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
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), 'Source-only repair was not authorized'
        assert script("return document.querySelector('#execution-summary').textContent") == 'מוכן לאימות הקבצים ולביצוע תיקונים לפי הצורך.'
        click('#review-apply')
        assert script("return document.querySelector('#apply-target-label').hidden")
        assert script("return document.querySelector('#apply-source').value") == str(source)
        description = script("return document.querySelector('#apply-description').textContent")
        assert 'תגיות' in description and 'תיקונים מתוכננים בלבד' in description
        assert 'קבצים שלא נדרש בהם תיקון יישארו ללא שינוי' in description

    for version, unsupported in ((3, False), (4, False), (4, True)):
        label = 'unsupported' if unsupported else f'id3v2{version}'
        source = root / label; source.mkdir()
        path = source / f'repair_input_{version}.mp3'
        expected_name = f'repair input {version}.mp3'
        before = generated_mp3(audio, version, unsupported); path.write_bytes(before)
        unchanged = source / 'unchanged.wav'; write_wave(unchanged)
        unchanged_bytes = unchanged.read_bytes()
        fields, raw, offset, payload = inspect_mp3(before)
        assert fields == {key: value[0] for key, value in FIELDS.items()}
        setup(source)
        capture('fix-review-' + label)
        click('#apply-back')
        assert set(source.iterdir()) == {path, unchanged} and path.read_bytes() == before
        assert unchanged.read_bytes() == unchanged_bytes
        click('#review-apply'); click('#confirm-apply')
        expected_title = 'הביצוע הושלם חלקית' if unsupported else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == expected_title, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelector('#review-apply').hidden")
        fixed = source / expected_name
        assert fixed.is_file() and not path.exists()
        backups = list(source.glob('*.backup'))
        after = fixed.read_bytes()
        summary = script("return document.querySelector('#execution-summary').textContent")
        assert 'שמות תוקנו' in summary and 'הועברו' not in summary and 'עותקים' not in summary
        if unsupported:
            assert after == before and not backups
            assert 'תגיות שלא תוקנו' in summary
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
        else:
            assert len(backups) == 1 and backups[0].read_bytes() == before
            changed, new_raw, new_offset, new_payload = inspect_mp3(after)
            assert changed == {key: value[1] for key, value in FIELDS.items()}
            assert new_offset == offset and new_payload == payload
            assert new_raw[b'PRIV'] == raw[b'PRIV']
        assert unchanged.read_bytes() == unchanged_bytes
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert 'unchanged.wav' not in script("return document.querySelector('#rows').textContent")
        assert_observed_filename(script, fixed)
        click('#rows .detail-button')
        assert script("return document.querySelector('#row-title').textContent") == fixed.name
        click('#row-close')
        wait(lambda: script("return !document.querySelector('#row-dialog').open"))
        set_developer_mode(script, click, wait, True)
        click('#rows .detail-button')
        assert script("return document.querySelector('#row-title').textContent") == fixed.name
        assert str(path) in script("return Array.from(document.querySelectorAll('#row-content input'),n=>n.value)")
        click('#row-close')
        wait(lambda: script("return !document.querySelector('#row-dialog').open"))
        assert not script("return document.querySelector('#show-inactive').checked")
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        set_developer_mode(script, click, wait, False)
        assert script("return document.querySelector('#show-inactive').checked")
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        capture('fix-result-including-unchanged-' + label)
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert '—' not in script("return document.querySelector('#rows').textContent")
        if version == 4:
            exercise_shell(command, script, capture, output, 'fix-terminal-' + label)
        capture('fix-result-' + label)
        receipts.append(dict(case=label, source=str(source), journal_path=script("return document.querySelector('#journal-path').value"), original_sha256=digest(before), current_sha256=digest(after),
                             backup_sha256=[digest(backup.read_bytes()) for backup in backups],
                             displayed_filename=fixed.name, renamed=True, tags_repaired=not unsupported, media_sha256=digest(payload),
                             unchanged_sha256=digest(unchanged_bytes), inactive_hidden_by_default=True,
                             developer_filter_independent=True, ipc_mocked=False))
    source = root / 'stale'; source.mkdir(); path = source / 'stale_input.mp3'
    path.write_bytes(generated_mp3(audio, 4)); setup(source)
    path.write_bytes(path.read_bytes() + b'owned-stale-change'); changed = path.read_bytes()
    click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert list(source.iterdir()) == [path] and path.read_bytes() == changed
    capture('fix-stale-plan-rejected')
    receipts.append(dict(case='stale', source_unchanged=True, ipc_mocked=False))
    (Path(output) / 'repair-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
