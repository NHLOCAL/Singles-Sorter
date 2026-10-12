"""Actual existing-ID3 container acceptance on tiny owned media fixtures.

Independent frame/container parsing verifies five fields, exact media offsets,
unknown bytes, original backups and a real FFmpeg decode. No arbitrary media is
loaded by this fixture helper. Native writer execution occurs only in the caller.
"""
import json
from pathlib import Path
import subprocess
import tempfile

from desktop_modes import select_sort_setup
from desktop_ai import tree_snapshot, write_wave
from desktop_polish import set_developer_mode
from desktop_repair import FIELDS, PRIVATE, assert_observed_filename, digest, inspect_mp3, sync_size, assert_repair_terminal_notice
from desktop_recovery import journal_snapshot

ARTWORK = b'\x00image/png\x00\x03\x00owned-retained-artwork'
CHECKS = {'actual-fix-wave-id3-five-fields', 'actual-fix-aiff-id3-five-fields',
          'actual-fix-dsf-id3-five-fields', 'container-id3v23-and-id3v24',
          'container-id3-exact-media-offsets-and-decoded-audio',
          'container-id3-opaque-artwork-info-preserved', 'container-id3-original-backup-retained',
          'container-id3-insufficient-padding-partial', 'container-id3-unchanged-hidden',
          'container-id3-stale-plan-no-effects'}
CASES = [('wav', 3, False), ('wav', 4, False), ('aiff', 4, False),
         ('dsf', 3, False), ('aiff', 3, True)]


def assertions_required():
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')


def generated_container(kind, version=4, unsupported=False, repaired=False):
    assertions_required()
    assert kind in ('wav', 'aiff', 'dsf') and version in (3, 4)
    frames = []
    for name, values in FIELDS.items():
        text = values[1 if repaired else 0]
        payload = b'\x00' + text.encode('latin1') if unsupported else (
            b'\x01' + text.encode('utf-16') if version == 3 else b'\x03' + text.encode('utf-8'))
        frames.append((name, payload))
    frames += [(b'PRIV', PRIVATE), (b'APIC', ARTWORK)]
    body = b''.join(name + (sync_size(len(payload)) if version == 4 else len(payload).to_bytes(4, 'big'))
                    + b'\0\0' + payload for name, payload in frames)
    if not unsupported:
        body += bytes(512 - len(body))
    tag = b'ID3' + bytes((version, 0, 0)) + sync_size(len(body)) + body
    base = Path(__file__).resolve().parents[3] / 'tests/fixtures/media'
    data = bytearray((base / ('blank.' + kind)).read_bytes())
    if kind == 'dsf':
        data[20:28] = len(data).to_bytes(8, 'little')
        data += tag
        data[12:20] = len(data).to_bytes(8, 'little')
    else:
        order = 'little' if kind == 'wav' else 'big'
        for name, value in [(b'JUNK', b'owned opaque odd chunk\0\xff'),
                            *([(b'LIST', b'INFOINAM\x04\0\0\0old\0')] if kind == 'wav' else []), (b'ID3 ', tag)]:
            data += name + len(value).to_bytes(4, order) + value
            if len(value) % 2:
                data += b'Z'
        data[4:8] = (len(data) - 8).to_bytes(4, order)
    return bytes(data)


def inspect_container(data, kind):
    assertions_required()
    if kind == 'dsf':
        assert data[:4] == b'DSD ' and int.from_bytes(data[4:12], 'little') == 28
        assert int.from_bytes(data[12:20], 'little') == len(data)
        assert data[28:32] == b'fmt ' and int.from_bytes(data[32:40], 'little') == 52
        assert data[80:84] == b'data'
        start = int.from_bytes(data[20:28], 'little')
        assert start == 80 + int.from_bytes(data[84:92], 'little') and 92 < start < len(data)
        end = len(data); layout = [('DSD ', 0, 28), ('fmt ', 28, 80), ('data', 80, start), ('ID3 ', start, end)]
        audio = data[92:start]
    else:
        assert kind in ('wav', 'aiff')
        order = 'little' if kind == 'wav' else 'big'
        assert data[:4] == (b'RIFF' if kind == 'wav' else b'FORM')
        assert data[8:12] == (b'WAVE' if kind == 'wav' else b'AIFF')
        assert int.from_bytes(data[4:8], order) == len(data) - 8
        at = 12; layout = []; spans = []; audio_parts = []
        while at < len(data):
            assert at + 8 <= len(data)
            name = data[at:at + 4]; size = int.from_bytes(data[at + 4:at + 8], order)
            limit = at + 8 + size; assert limit + (size & 1) <= len(data)
            layout.append((name.decode('ascii'), at, limit + (size & 1)))
            if name == b'ID3 ': spans.append((at + 8, limit))
            if name == (b'data' if kind == 'wav' else b'SSND'): audio_parts.append(data[at + 8:limit])
            at = limit + (size & 1)
        assert at == len(data) and len(spans) == 1 and len(audio_parts) == 1
        start, end = spans[0]; audio = audio_parts[0]
    fields, raw, tag_end, tail = inspect_mp3(data[start:end])
    assert tag_end == end - start and not tail and audio
    return fields, raw, start, end, layout, audio


def verify_repaired(before, after, kind):
    assertions_required()
    old, new = inspect_container(before, kind), inspect_container(after, kind)
    assert old[0] == {key: value[0] for key, value in FIELDS.items()}
    assert new[0] == {key: value[1] for key, value in FIELDS.items()}
    assert len(before) == len(after) and old[2:] == new[2:]
    start, end = old[2:4]
    assert before[:start] == after[:start] and before[end:] == after[end:]
    assert before[start:start + 10] == after[start:start + 10]
    for name in (b'PRIV', b'APIC'):
        assert old[1][name] == new[1][name]
    assert after == generated_container(kind, before[start + 3], repaired=True)
    with tempfile.TemporaryDirectory(prefix='owned-container-decoder-') as directory:
        a, b = Path(directory) / ('before.' + kind), Path(directory) / ('after.' + kind)
        a.write_bytes(before); b.write_bytes(after)
        def decode(path):
            return subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:a:0', '-f', 's16le', '-'],
                                  check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30).stdout
        pcm = decode(a); assert pcm and pcm == decode(b)
    return dict(kind=kind, original_bytes=len(before), current_bytes=len(after),
                tag_offset=start, tag_end=end, pcm_sha256=digest(pcm), media_sha256=digest(old[5]),
                fields={key.decode(): value for key, value in new[0].items()}, exact_original_extent=True)


def exercise_id3_containers(script, click, fill, wait, capture, root, output):
    assertions_required()
    root = Path(root); root.mkdir(); receipts = []
    previous = json.loads((Path(output) / 'selected-execution.json').read_text(encoding='utf-8'))
    roots = {case['journal_path'] for case in previous['cases'] if case.get('journal_path')}
    assert previous['status'] == 'passed' and len(roots) == 1
    journal_root = Path(roots.pop()); assert journal_root.is_dir()

    def setup(source):
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        select_sort_setup(script, click, wait)
        fill('#source', str(source)); fill('#target', '')
        click('#advanced'); click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
        before_journal = journal_snapshot(journal_root)
        click('#tab-mode-fix')
        click('#fix')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled")
        if script("return document.querySelector('#show-inactive').checked"):
            click('#show-inactive')
            wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert journal_snapshot(journal_root) == before_journal
        return before_journal

    for kind, version, unsupported in CASES:
        label = kind + '-id3v2' + str(version) + ('-insufficient-padding' if unsupported else '')
        source = root / label; source.mkdir(); path = source / ('repair_input.' + kind); fixed = source / ('repair input.' + kind)
        before = generated_container(kind, version, unsupported); path.write_bytes(before)
        unchanged = source / 'unchanged.wav'; write_wave(unchanged); unchanged_bytes = unchanged.read_bytes()
        before_tree = tree_snapshot(source); before_journal = setup(source)
        assert tree_snapshot(source) == before_tree
        click('#review-apply'); assert script("return document.querySelector('#apply-target-label').hidden")
        capture('container-review-' + label); click('#apply-back')
        assert tree_snapshot(source) == before_tree and journal_snapshot(journal_root) == before_journal
        click('#review-apply'); click('#confirm-apply')
        title = 'הביצוע הושלם חלקית' if unsupported else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == title, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert fixed.is_file() and not path.exists() and unchanged.read_bytes() == unchanged_bytes
        after = fixed.read_bytes(); backups = list(source.glob('*.backup')); media = None
        if unsupported:
            assert after == before and not backups
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
            assert 'תגיות שלא תוקנו' in script("return document.querySelector('#execution-summary').textContent")
        else:
            assert len(backups) == 1 and backups[0].read_bytes() == before
            media = verify_repaired(before, after, kind)
        assert set(source.iterdir()) == {fixed, unchanged, *backups}
        terminal_notice = assert_repair_terminal_notice(script, unsupported)
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert_observed_filename(script, fixed)
        capture('container-result-' + label)
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert script("return document.querySelector('#review-apply').hidden")
        receipts.append(dict(case=label, original_sha256=digest(before), current_sha256=digest(after),
                             backup_sha256=[digest(path.read_bytes()) for path in backups], media=media,
                             unchanged_sha256=digest(unchanged_bytes), displayed_filename=fixed.name,
                             tags_repaired=not unsupported, terminal_notice=terminal_notice, inactive_hidden_by_default=True, ipc_mocked=False))
    source = root / 'stale'; source.mkdir(); path = source / 'stale_input.dsf'
    path.write_bytes(generated_container('dsf')); before_journal = setup(source)
    changed_bytes = path.read_bytes().replace(b'owned-fixture-owner', b'OWNED-fixture-owner')
    assert changed_bytes != path.read_bytes(); path.write_bytes(changed_bytes); changed = tree_snapshot(source)
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert tree_snapshot(source) == changed and journal_snapshot(journal_root) == before_journal
    capture('container-stale-plan-rejected'); receipts.append(dict(case='stale', source_unchanged=True, journal_unchanged=True, ipc_mocked=False))
    (Path(output) / 'id3-container-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
