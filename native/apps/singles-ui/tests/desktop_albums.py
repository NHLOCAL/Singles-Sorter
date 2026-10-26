"""Actual native album Copy/Move acceptance on disposable authored media only."""
import json
import os
from pathlib import Path
import stat

from desktop_modes import select_sort_setup
from desktop_ai import digest
from desktop_repair import sync_size
from desktop_polish import set_developer_mode

ARTIST = 'אברהם פריד'
ALBUM = 'אלבום בדיקה'
CHECKS = {'actual-album-copy', 'actual-root-album-move', 'actual-mixed-album-move',
          'album-nested-files-and-sidecars', 'album-copy-metadata',
          'album-review-no-effects', 'album-stale-membership-no-effects',
          'album-terminal-simple-results'}


def album_audio(audio, track):
    body = bytearray()
    for name, value in [(b'TPE1', ARTIST), (b'TPE2', ARTIST), (b'TALB', ALBUM), (b'TRCK', str(track))]:
        payload = b'\x03' + value.encode('utf-8')
        body.extend(name + sync_size(len(payload)) + b'\x00\x00' + payload)
    return b'ID3\x04\x00\x00' + sync_size(len(body)) + body + audio


def files(root):
    return {path.relative_to(root).as_posix(): digest(path) for path in root.rglob('*') if path.is_file()}


def snapshot(root):
    """No-effect receipt includes roots/directories; atime may change on reads."""
    result = {}
    for path in [root, *sorted(root.rglob('*'))]:
        info = path.lstat()
        result[path.relative_to(root).as_posix()] = dict(
            kind=stat.S_IFMT(info.st_mode), mode=info.st_mode,
            attributes=getattr(info, 'st_file_attributes', None),
            size=info.st_size, modified_ns=info.st_mtime_ns,
            sha256=digest(path) if stat.S_ISREG(info.st_mode) else None)
    return result


def metadata(root):
    # Copytree descendants receive copystat; the album container is freshly created.
    # Windows timestamps are compared before content verification reads.
    return {path.relative_to(root).as_posix(): (path.stat().st_mtime_ns, bool(path.stat().st_file_attributes & stat.FILE_ATTRIBUTE_READONLY))
            for path in root.rglob('*')}


def fixture(root, audio, root_album):
    source = root / 'source'; target = root / 'target'
    source.mkdir(parents=True); target.mkdir()
    album = source if root_album else source / 'input-album'
    if not root_album:
        album.mkdir()
        (source / f'{ARTIST} - שיר בודד.mp3').write_bytes(audio)
    for track in range(1, 4):
        (album / f'{track}-שיר.mp3').write_bytes(album_audio(audio, track))
    (album / 'cover.jpg').write_bytes(b'owned sidecar, not a commercial image')
    (album / 'booklet/nested').mkdir(parents=True)
    (album / 'booklet/nested/readme.txt').write_text('owned nested album file', encoding='utf-8')
    # Fixed, exactly representable NTFS modification times; no user files.
    for path in [*album.rglob('*'), album]:
        os.utime(path, ns=(1_600_000_000_000_000_000, 1_600_000_000_000_000_000))
    return source, target, album


def exercise_albums(script, click, fill, wait, capture, root, output, audio_path):
    root = Path(root); root.mkdir()
    audio = Path(audio_path).read_bytes()
    receipts = []

    def setup(source, target, copy):
        if script("return document.querySelector('#app').dataset.view==='results'"):
            click('#back-setup')
            wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
        select_sort_setup(script, click, wait)
        fill('#source', str(source)); fill('#target', str(target))
        for key, value in dict(copy_mode=copy, main_folder_only=False, singles_folder=False, abc_sort=False, exist_only=False).items():
            if script(f"return document.querySelector('#{key}').checked") != value:
                click('#' + key)
        click('#open-settings'); click('#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
        script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}));")
        click('#advanced-dialog .close-dialog')
        set_developer_mode(script, click, wait, False)
        click('#tab-mode-sort')
        click('#sort')
        wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#back-setup').disabled"), timeout=120)
        assert script("return !document.querySelector('#review-apply').hidden && !document.querySelector('#review-apply').disabled"), 'Native album plan was not authorized'

    for copying, root_album, label in [(True, False, 'copy'), (False, True, 'root-move'), (False, False, 'mixed-move')]:
        source, target, album = fixture(root / label, audio, root_album)
        if copying:
            (album / 'cover.jpg').chmod(stat.S_IREAD)
        before = files(album); source_before = files(source); before_metadata = metadata(album)
        source_snapshot = snapshot(source); target_snapshot = snapshot(target)
        setup(source, target, copying)
        assert snapshot(source) == source_snapshot and snapshot(target) == target_snapshot
        click('#review-apply')
        assert 'סגור תוכנות או סנכרון' in script("return document.querySelector('#apply-description').textContent")
        capture('album-review-' + label); click('#apply-back')
        assert snapshot(source) == source_snapshot and snapshot(target) == target_snapshot
        click('#review-apply'); click('#confirm-apply')
        wait(lambda: script("return document.querySelector('#execution-title').textContent==='הביצוע הושלם'"), timeout=120)
        wait(lambda: script("return !document.querySelector('#show-inactive').disabled"))
        destination = target / ARTIST / ALBUM
        if copying:
            assert metadata(destination) == before_metadata, 'Album Copy did not preserve descendant timestamps and readonly attributes'
            container = destination.stat()
            assert destination.is_dir() and container.st_mtime_ns != album.stat().st_mtime_ns
            assert not container.st_file_attributes & stat.FILE_ATTRIBUTE_READONLY, 'Fresh album container inherited readonly attributes'
            assert files(source) == source_before
        else:
            assert not album.exists(), 'Whole album Move left its original container path'
        assert files(destination) == before, 'Nested album files or sidecars differ'
        if not root_album:
            single = target / ARTIST / f'{ARTIST} - שיר בודד.mp3'
            assert single.read_bytes() == audio
        summary = script("return document.querySelector('#execution-summary').textContent")
        assert '1 אלבומים הושלמו' in summary
        assert script("return document.querySelectorAll('#rows tr').length") == (1 if root_album else 2)
        assert 'הקובץ לא נמצא' not in script("return document.querySelector('#rows').textContent")
        if not copying:
            click('#show-inactive')
            wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
            assert script("return document.querySelectorAll('#rows tr').length") == (3 if root_album else 4)
            assert script("return document.querySelector('#rows').textContent").count('כלול באלבום') == 2
            assert 'הקובץ לא נמצא' not in script("return document.querySelector('#rows').textContent")
            capture('album-included-descendants-' + label)
            click('#show-inactive')
            wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
            assert script("return document.querySelectorAll('#rows tr').length") == (1 if root_album else 2)
        assert '\u2014' not in script("return document.querySelector('#rows').textContent")
        assert script("return document.querySelector('#review-apply').hidden")
        capture('album-complete-' + label)
        receipts.append(dict(case=label, source=str(source), target=str(target),
                             journal_path=script("return document.querySelector('#journal-path').value"),
                             file_count=len(source_before), source_sha256=before, destination_sha256=files(destination),
                             source_retained=copying, descendant_metadata_preserved=copying, fresh_copy_container=copying, ipc_mocked=False))

    source, target, album = fixture(root / 'stale-membership', audio, True)
    setup(source, target, True)
    (album / 'late-sidecar.txt').write_bytes(b'owned addition after preview')
    changed = snapshot(source); target_before = snapshot(target)
    click('#review-apply'); click('#confirm-apply')
    wait(lambda: script("return !document.querySelector('#back-setup').disabled && !document.querySelector('#error').hidden"), timeout=120)
    assert snapshot(source) == changed and snapshot(target) == target_before, 'Changed album membership produced an effect'
    capture('album-stale-membership-rejected')
    receipts.append(dict(case='stale-membership', source_unchanged=True, target_unchanged=True, ipc_mocked=False))
    (Path(output) / 'album-execution.json').write_text(json.dumps(dict(status='passed', checks=sorted(CHECKS), cases=receipts), ensure_ascii=False, indent=2), encoding='utf-8')
    return sorted(CHECKS)
