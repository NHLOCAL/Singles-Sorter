"""Owned MP4 AAC/ALAC Fix acceptance through the real shared desktop.

The authored atom oracle is independent of the Rust writer. Full-byte equality,
original backups, unchanged encoded media and an actual FFmpeg decode are checked.
This helper handles tiny fixed fixtures, never an arbitrary user media library.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile

from desktop_modes import select_sort_setup
from desktop_ai import tree_snapshot, write_wave
from desktop_polish import set_developer_mode
from desktop_repair import assert_observed_filename, assert_repair_terminal_notice, digest
from desktop_recovery import journal_snapshot

CHECKS = {'actual-fix-m4a-five-fields', 'actual-fix-alac-five-fields',
          'mp4-fast-start-multiple-values', 'mp4-exact-media-offsets-and-decoded-audio',
          'mp4-artwork-opaque-atoms-preserved', 'mp4-original-backup-retained',
          'mp4-duplicate-outer-fields-consolidated', 'mp4-unsupported-structure-partial', 'mp4-unchanged-hidden', 'mp4-stale-plan-no-effects'}
_fixture = None


def fixture():
    global _fixture
    if not __debug__:
        raise RuntimeError('Physical acceptance requires Python assertions enabled')
    if _fixture is None:
        path = Path(__file__).resolve().parents[3] / 'tests/staged_mp4/verify_fixture_contract.py'
        spec = importlib.util.spec_from_file_location('owned_mp4_contract', path)
        _fixture = importlib.util.module_from_spec(spec); spec.loader.exec_module(_fixture)
    return _fixture


def generated_mp4(*, alac=False, multiple=False, fast_start=False, duplicate_outer=False, opaque_structure=False):
    oracle = fixture()
    data = oracle.fixture(alac, multiple)
    if duplicate_outer:
        unknown = b''.join(box for kind, box in oracle.list_items(data) if kind not in oracle.KEYS)
        # First values are already correct. Only the later artist value needs
        # repair, so an old first-atom-only native projection cannot pass.
        payload = b''.join(oracle.text(key, ['שלום']) for key in oracle.KEYS)
        data = oracle.update(data, payload + unknown + oracle.text(oracle.KEYS[0], ['á']))
    if opaque_structure:
        data = b''.join(oracle.atom(kind, box[8:] + oracle.atom(b'uuid', bytes(range(16)) + b'owned private structure'))
                        if kind == b'moov' else box for kind, box in oracle.children(data))
    return oracle.fast_start(data) if fast_start else data


def expected_repair(before):
    """Known fixture text tuples, following frozen ordered comma-space joins."""
    oracle = fixture()
    values = {key: [] for key in oracle.KEYS}
    unknown = []
    for key, box in oracle.list_items(before):
        if key not in values:
            unknown.append(box); continue
        for kind, data in oracle.children(box[8:]):
            assert kind == b'data' and data[8:16] == b'\0\0\0\1\0\0\0\0'
            values[key].append(data[16:].decode('utf-8'))
    conversion = str.maketrans(dict(zip(oracle.ORIGINAL, oracle.REPAIRED)))
    replacements = {key: ', '.join(items).translate(conversion) for key, items in values.items() if items}
    payload = b''.join(oracle.text(key, [text]) for key, text in replacements.items()) + b''.join(unknown)
    return oracle.update(before, payload, exact=True), replacements


def verify_repaired(before, after, multiple=False):
    oracle = fixture()
    expected_bytes, replacements = expected_repair(before)
    assert after == expected_bytes, 'MP4 differs from the independent exact-extent rewrite'
    assert len(after) == len(before)
    old, new = oracle.children(before), oracle.children(after)
    assert [(kind, len(box)) for kind, box in old] == [(kind, len(box)) for kind, box in new]
    for (kind, left), (_, right) in zip(old, new):
        if kind != b'moov':
            assert left == right
    for key in (b'zzzz', b'covr'):
        assert [box for kind, box in oracle.list_items(before) if kind == key] == [box for kind, box in oracle.list_items(after) if kind == key]
    with tempfile.TemporaryDirectory(prefix='owned-mp4-decoder-') as directory:
        original, current = Path(directory) / 'before.m4a', Path(directory) / 'after.m4a'
        original.write_bytes(before); current.write_bytes(after)
        def pcm(path):
            return subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:a:0', '-f', 's16le', '-'],
                                  check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30).stdout
        old_pcm, new_pcm = pcm(original), pcm(current)
        assert old_pcm and old_pcm == new_pcm
        output = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format_tags', '-of', 'json', str(current)],
                                check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        tags = json.loads(output.stdout)['format']['tags']
    names = dict(zip(oracle.KEYS, ('artist', 'album_artist', 'title', 'album', 'genre')))
    expected = {names[key]: value for key, value in replacements.items()}
    assert {name: tags[name] for name in expected} == expected
    return dict(original_bytes=len(before), current_bytes=len(after), pcm_sha256=digest(old_pcm),
                media_sha256=digest(b''.join(box for kind, box in old if kind == b'mdat')),
                fields=expected, exact_original_extent=True)


def exercise_mp4(script, click, fill, wait, capture, root, output):
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

    for alac, multiple, duplicate, opaque, label in [(False, False, False, False, 'm4a-five-fields'), (True, False, False, False, 'alac-five-fields'),
                                                     (False, True, False, False, 'fast-start-values'), (False, False, True, False, 'duplicate-outer'),
                                                     (False, False, False, True, 'opaque-structure')]:
        source = root / label; source.mkdir(); extension = 'alac' if alac else 'm4a'
        path = source / f'repair_input.{extension}'; fixed = source / f'repair input.{extension}'
        before = generated_mp4(alac=alac, multiple=multiple, fast_start=multiple, duplicate_outer=duplicate, opaque_structure=opaque)
        path.write_bytes(before); unchanged = source / 'unchanged.wav'; write_wave(unchanged)
        unchanged_bytes = unchanged.read_bytes(); before_tree = tree_snapshot(source)
        before_journal = setup(source); assert tree_snapshot(source) == before_tree
        click('#review-apply'); assert script("return document.querySelector('#apply-target-label').hidden")
        capture('mp4-review-' + label); click('#apply-back'); assert tree_snapshot(source) == before_tree
        assert journal_snapshot(journal_root) == before_journal
        click('#review-apply'); click('#confirm-apply')
        title = 'הביצוע הושלם חלקית' if opaque else 'הביצוע הושלם'
        wait(lambda: script("return document.querySelector('#execution-title').textContent") == title, timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        assert fixed.is_file() and not path.exists() and unchanged.read_bytes() == unchanged_bytes
        after = fixed.read_bytes(); backups = list(source.glob('*.backup')); media = None
        if opaque:
            assert after == before and not backups
            assert 'בוצע חלקית' in script("return document.querySelector('#rows').textContent")
            assert 'תגיות שלא תוקנו' in script("return document.querySelector('#execution-summary').textContent")
        else:
            assert len(backups) == 1 and backups[0].read_bytes() == before
            media = verify_repaired(before, after, multiple)
        assert set(source.iterdir()) == {fixed, unchanged, *backups}
        terminal_notice = assert_repair_terminal_notice(script, opaque)
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert_observed_filename(script, fixed)
        assert 'unchanged.wav' not in script("return document.querySelector('#rows').textContent")
        capture('mp4-result-' + label)
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 2
        assert 'ללא שינוי' in script("return document.querySelector('#rows').textContent")
        click('#show-inactive'); wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        assert script("return document.querySelector('#review-apply').hidden")
        receipts.append(dict(case=label, original_sha256=digest(before), current_sha256=digest(after),
                             backup_sha256=[digest(path.read_bytes()) for path in backups], media=media,
                             unchanged_sha256=digest(unchanged_bytes), displayed_filename=fixed.name,
                             tags_repaired=not opaque, terminal_notice=terminal_notice, inactive_hidden_by_default=True, ipc_mocked=False))
    source = root / 'stale'; source.mkdir(); path = source / 'stale_input.m4a'
    path.write_bytes(generated_mp4()); before_journal = setup(source)
    changed_bytes = path.read_bytes().replace(b'opaque unrecognized atom', b'opaque UNRECOGNIZED atom')
    assert changed_bytes != path.read_bytes(); path.write_bytes(changed_bytes); changed = tree_snapshot(source)
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert tree_snapshot(source) == changed
    assert journal_snapshot(journal_root) == before_journal
    capture('mp4-stale-plan-rejected'); receipts.append(dict(case='stale', source_unchanged=True, journal_unchanged=True, ipc_mocked=False))
    (Path(output) / 'mp4-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
