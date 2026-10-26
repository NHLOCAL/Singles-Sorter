"""Actual terminal APEv2 WavPack repair on tiny independently authored media.

The original NUL-separated scalar is retained, never replaced by a comma-space
join. Exact WavPack blocks, APE layout, artwork and unknown bytes are checked.
"""
import json
from pathlib import Path
import subprocess
import tempfile

from desktop_modes import select_sort_setup
from desktop_ai import tree_snapshot, write_wave
from desktop_polish import set_developer_mode
from desktop_repair import FIELDS, assert_observed_filename, digest, assert_repair_terminal_notice
from desktop_recovery import journal_snapshot

KEYS = ('Artist', 'Album Artist', 'Title', 'Album', 'Genre')
CASES = [(False, False, False, 'footer-only'), (True, False, False, 'with-header'),
         (True, True, False, 'nul-values'), (True, False, True, 'readonly-item')]
CHECKS = {'actual-fix-wavpack-five-fields', 'wavpack-header-and-footer-only',
          'wavpack-original-nul-scalars', 'wavpack-exact-blocks-and-decoded-audio',
          'wavpack-artwork-unknown-items-preserved', 'wavpack-original-backup-retained',
          'wavpack-readonly-item-partial', 'wavpack-unchanged-hidden', 'wavpack-stale-plan-no-effects'}


def assertions_required():
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')


def generated_wavpack(header=False, multiple=False, readonly=False, repaired=False):
    assertions_required()
    audio = (Path(__file__).resolve().parents[3] / 'tests/fixtures/media/blank.wv').read_bytes()
    def item(key, value, flags=0):
        return len(value).to_bytes(4, 'little') + flags.to_bytes(4, 'little') + key.encode('ascii') + b'\0' + value
    fields = []
    for key, values in zip(KEYS, FIELDS.values()):
        text = values[1 if repaired else 0]
        if multiple: text += '\0' + text[::-1]
        fields.append(item(key, text.encode('utf-8'), 1 if readonly and key == 'Artist' else 0))
    fields += [item('Cover Art (Front)', b'cover.png\0owned retained artwork\xff', 2),
               item('Unknown Binary', b'owned unknown item\0\xff', 2)]
    items = b''.join(fields)
    footer = b'APETAGEX' + (2000).to_bytes(4, 'little') + (len(items) + 32).to_bytes(4, 'little') + len(fields).to_bytes(4, 'little')
    footer += (0x80000000 if header else 0).to_bytes(4, 'little') + bytes(8)
    head = footer[:20] + (0xa0000000).to_bytes(4, 'little') + footer[24:] if header else b''
    return audio + head + items + footer


def inspect_wavpack(data):
    assertions_required()
    footer = data[-32:]
    assert footer[:8] == b'APETAGEX' and int.from_bytes(footer[8:12], 'little') == 2000
    size = int.from_bytes(footer[12:16], 'little'); count = int.from_bytes(footer[16:20], 'little')
    flags = int.from_bytes(footer[20:24], 'little')
    assert flags in (0, 0x80000000) and footer[24:] == bytes(8) and 32 < size <= len(data)
    items_start = len(data) - size; audio_end = items_start - (32 if flags else 0)
    if flags:
        assert data[audio_end:items_start] == footer[:20] + (0xa0000000).to_bytes(4, 'little') + footer[24:]
    at = 0; blocks = []
    while at < audio_end:
        assert data[at:at + 4] == b'wvpk' and at + 32 <= audio_end
        limit = at + 8 + int.from_bytes(data[at + 4:at + 8], 'little')
        assert at + 32 <= limit <= audio_end
        blocks.append((at, limit)); at = limit
    assert at == audio_end and blocks
    at = items_start; items = {}; keys = set()
    for _ in range(count):
        begin = at; assert at + 8 < len(data) - 32
        length = int.from_bytes(data[at:at + 4], 'little'); item_flags = int.from_bytes(data[at + 4:at + 8], 'little')
        boundary = data.index(b'\0', at + 8, len(data) - 32)
        key = data[at + 8:boundary].decode('ascii'); assert key and key.casefold() not in keys
        keys.add(key.casefold()); at = boundary + 1 + length; assert at <= len(data) - 32
        items[key] = dict(value=data[boundary + 1:at], flags=item_flags, raw=data[begin:at], offset=boundary + 1)
    assert at == len(data) - 32 and set(items) == {*KEYS, 'Cover Art (Front)', 'Unknown Binary'}
    return items, blocks, audio_end, footer


def verify_repaired(before, after):
    assertions_required()
    old, new = inspect_wavpack(before), inspect_wavpack(after)
    assert len(before) == len(after) and old[1:] == new[1:]
    assert before[:old[2]] == after[:new[2]]
    multiple = b'\0' in old[0]['Artist']['value']; header = int.from_bytes(old[3][20:24], 'little') != 0
    assert after == generated_wavpack(header, multiple, repaired=True)
    expected = {}
    for key, values in zip(KEYS, FIELDS.values()):
        original, repaired = (value + ('\0' + value[::-1] if multiple else '') for value in values)
        assert old[0][key]['value'].decode('utf-8') == original
        assert new[0][key]['value'].decode('utf-8') == repaired
        assert old[0][key]['flags'] == new[0][key]['flags'] == 0
        assert old[0][key]['offset'] == new[0][key]['offset']; expected[key] = repaired
    for key in ('Cover Art (Front)', 'Unknown Binary'): assert old[0][key]['raw'] == new[0][key]['raw']
    with tempfile.TemporaryDirectory(prefix='owned-wavpack-decoder-') as directory:
        a, b = Path(directory) / 'before.wv', Path(directory) / 'after.wv'
        a.write_bytes(before); b.write_bytes(after)
        def decode(path):
            return subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:a:0', '-f', 's16le', '-'],
                                  check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30).stdout
        pcm = decode(a); assert pcm and pcm == decode(b)
    return dict(original_bytes=len(before), current_bytes=len(after), pcm_sha256=digest(pcm),
                block_sha256=digest(before[:old[2]]), fields=expected, exact_original_extent=True)


def exercise_wavpack(script, click, fill, wait, capture, root, output):
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
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
        click('#open-settings'); click('#duet-first')
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

    for header, multiple, readonly, label in CASES:
        source = root / label; source.mkdir(); extension = 'wv'
        path = source / f'repair_input.{extension}'; fixed = source / f'repair input.{extension}'
        before = generated_wavpack(header, multiple, readonly)
        path.write_bytes(before); unchanged = source / 'unchanged.wav'; write_wave(unchanged)
        unchanged_bytes = unchanged.read_bytes(); before_tree = tree_snapshot(source)
        before_journal = setup(source); assert tree_snapshot(source) == before_tree
        click('#review-apply'); assert script("return document.querySelector('#apply-target-label').hidden")
        capture('wavpack-review-' + label); click('#apply-back'); assert tree_snapshot(source) == before_tree
        assert journal_snapshot(journal_root) == before_journal
        click('#review-apply'); click('#confirm-apply')
        title = 'הביצוע הושלם חלקית' if readonly else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == title, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert fixed.is_file() and not path.exists() and unchanged.read_bytes() == unchanged_bytes
        after = fixed.read_bytes(); backups = list(source.glob('*.backup')); media = None
        if readonly:
            assert after == before and not backups
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
            assert 'תגיות שלא תוקנו' in script("return document.querySelector('#execution-summary').textContent")
        else:
            assert len(backups) == 1 and backups[0].read_bytes() == before
            media = verify_repaired(before, after)
        assert set(source.iterdir()) == {fixed, unchanged, *backups}
        terminal_notice = assert_repair_terminal_notice(script, readonly)
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert_observed_filename(script, fixed)
        assert 'unchanged.wav' not in script("return document.querySelector('#rows').textContent")
        capture('wavpack-result-' + label)
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert script("return document.querySelector('#review-apply').hidden")
        receipts.append(dict(case=label, original_sha256=digest(before), current_sha256=digest(after),
                             backup_sha256=[digest(path.read_bytes()) for path in backups], media=media,
                             unchanged_sha256=digest(unchanged_bytes), displayed_filename=fixed.name,
                             tags_repaired=not readonly, terminal_notice=terminal_notice, inactive_hidden_by_default=True, ipc_mocked=False))
    source = root / 'stale'; source.mkdir(); path = source / 'stale_input.wv'
    path.write_bytes(generated_wavpack()); before_journal = setup(source)
    changed_bytes = path.read_bytes().replace(b'owned unknown item', b'OWNED unknown item')
    assert changed_bytes != path.read_bytes(); path.write_bytes(changed_bytes); changed = tree_snapshot(source)
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert tree_snapshot(source) == changed
    assert journal_snapshot(journal_root) == before_journal
    capture('wavpack-stale-plan-rejected'); receipts.append(dict(case='stale', source_unchanged=True, journal_unchanged=True, ipc_mocked=False))
    (Path(output) / 'wavpack-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
