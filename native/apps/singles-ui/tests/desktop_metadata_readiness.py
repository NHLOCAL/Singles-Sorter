"""Real retained-plan readiness and ordinary refusal UI on owned ID3 fixtures."""
import json
from pathlib import Path

from desktop_ai import tree_snapshot
from desktop_albums import ARTIST
from desktop_modes import select_sort_setup
from desktop_polish import set_developer_mode
from desktop_recovery import journal_snapshot
from desktop_repair import sync_size

CHECKS = {'mixed-103-file-normal-apply-visible', 'mixed-metadata-review-no-effects',
          'unsupported-metadata-normal-reason-and-relative-file', 'metadata-refusal-no-effects'}


def audio_fixture(audio, version, flags):
    if audio.startswith(b'ID3'):
        size = 0
        for value in audio[6:10]:
            size = (size << 7) | value
        audio = audio[10 + size:]
    body = bytearray()
    for short, long, value in [(b'TP1', b'TPE1', ARTIST), (b'TP2', b'TPE2', ARTIST),
                              (b'TT2', b'TIT2', 'שיר לבדיקה'), (b'TAL', b'TALB', 'אלבום לדוגמה'),
                              (b'TCO', b'TCON', 'Test'), (b'TRK', b'TRCK', '2/9')]:
        encoded = b'\x01\xff\xfe' + value.encode('utf-16le') + b'\0\0'
        body.extend((short + len(encoded).to_bytes(3, 'big')) if version == 2
                    else (long + len(encoded).to_bytes(4, 'big') + b'\0\0'))
        body.extend(encoded)
    if flags & 0x40:
        body = bytearray(b'\0\0\0\x06\0\0\0\0\0\0') + body
    if flags & 0x80:
        escaped = bytearray()
        for index, value in enumerate(body):
            escaped.append(value)
            if value == 255 and (index + 1 == len(body) or body[index + 1] == 0 or body[index + 1] >= 0xe0):
                escaped.append(0)
        body = escaped
    return b'ID3' + bytes([version, 0, flags]) + sync_size(len(body)) + body + audio


def exercise_metadata_readiness(script, click, fill, wait, capture, root, output, audio_path):
    if not __debug__:
        raise RuntimeError('Readiness acceptance requires Python assertions enabled')
    root = Path(root); root.mkdir(); output = Path(output)
    audio = Path(audio_path).read_bytes()
    prior = json.loads((output / 'selected-execution.json').read_text(encoding='utf-8'))
    journals = {case['journal_path'] for case in prior['cases'] if case.get('journal_path')}
    assert prior['status'] == 'passed' and len(journals) == 1
    journal = Path(journals.pop())
    receipts = []
    for name in ['unsupported', 'mixed']:
        case = root / name; source = case / 'source'; target = case / 'target'
        source.mkdir(parents=True); target.mkdir()
        if name == 'mixed':
            for directory in ['disc-a', 'disc-b']:
                album = source / directory; album.mkdir()
                for index in range(40):
                    version, flags = [(2, 0), (3, 0x80), (3, 0x40)][index % 3]
                    (album / f'{index:03}.mp3').write_bytes(audio_fixture(audio, version, flags))
            for index in range(23):
                (source / f'{ARTIST} - {index:03}.mp3').write_bytes(b'' if index == 0 else audio_fixture(audio, 3, 0))
            assert len(list(source.rglob('*.mp3'))) == 103
        else:
            album = source / 'אלבום לבדיקה'; album.mkdir()
            for index in range(4):
                (album / f'{index}.mp3').write_bytes(audio_fixture(audio, 2, 0))
            (album / 'לא נתמך.mp3').write_bytes(audio_fixture(audio, 3, 0xc0))
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        select_sort_setup(script, click, wait)
        fill('#source', str(source)); fill('#target', str(target))
        for key, value in dict(copy_mode=True, main_folder_only=False, singles_folder=False, abc_sort=False, exist_only=False).items():
            if script(f"return document.querySelector('#{key}').checked") != value:
                click('#' + key)
        click('#advanced'); click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog'); set_developer_mode(script, click, wait, False)
        before, before_journal = tree_snapshot(case), journal_snapshot(journal)
        click('#sort')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
        assert script("return document.querySelector('#app').dataset.developerMode==='false'")
        summary = script("return document.querySelector('#execution-summary').textContent")
        if name == 'mixed':
            assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled")
            assert script("return document.querySelector('#execution-refusal').hidden")
            capture('metadata-mixed-103-ready')
            click('#review-apply'); wait(lambda: script("return document.querySelector('#apply-dialog').open"))
            assert tree_snapshot(case) == before and journal_snapshot(journal) == before_journal
            click('#apply-back')
            detail = None
        else:
            assert script("return document.querySelector('#review-apply').hidden")
            assert 'קריאת פרטי הקבצים' in summary and 'לא בוצעו שינויים' in summary
            assert script("return !document.querySelector('#execution-refusal').hidden && !document.querySelector('#execution-refusal').open")
            capture('metadata-refusal-ordinary')
            click('#execution-refusal > summary')
            detail = script("return document.querySelector('#execution-refusal-details').textContent")
            assert 'אלבום לבדיקה\\לא נתמך.mp3' in detail and 'UnsupportedClass' in detail
            assert str(source) not in detail and str(target) not in detail
            capture('metadata-refusal-relative-file')
        assert tree_snapshot(case) == before and journal_snapshot(journal) == before_journal
        receipts.append({'case': name, 'summary': summary, 'details': detail,
                         'input_files': len(list(source.rglob('*.mp3'))), 'source_target_unchanged': True,
                         'journal_unchanged': True, 'snapshot': before})
    (output / 'metadata-readiness.json').write_text(json.dumps({'status': 'passed', 'ipc_mocked': False,
                    'cases': receipts, 'checks': sorted(CHECKS)}, ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
