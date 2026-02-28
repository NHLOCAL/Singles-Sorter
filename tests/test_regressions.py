import pytest

from singlesorter.jibrish_to_hebrew import fix_jibrish
from singlesorter.sorter import MusicSorter


def test_scan_dir_requires_target_dir(tmp_path):
    source_dir = tmp_path / 'source'
    source_dir.mkdir()

    sorter = MusicSorter(source_dir=source_dir, target_dir=None)

    with pytest.raises(ValueError, match='תיקיית היעד נדרשת במצב מיון'):
        sorter.scan_dir()


def test_duet_mode_collects_multiple_artists_from_filename(tmp_path):
    source_dir = tmp_path / 'source'
    target_dir = tmp_path / 'target'
    source_dir.mkdir()
    target_dir.mkdir()

    song_path = source_dir / 'אברהם פריד ומוטי שטיינמץ - דואט.mp3'
    song_path.write_bytes(b'not-a-real-mp3')

    sorter = MusicSorter(source_dir=source_dir, target_dir=target_dir, duet_mode=True)
    sorter.singer_list = [
        ('אברהם פריד', 'אברהם פריד'),
        ('מוטי שטיינמץ', 'מוטי שטיינמץ'),
    ]

    assert sorter.artists_from_song(song_path) == ['אברהם פריד', 'מוטי שטיינמץ']


def test_list_from_csv_loads_personal_file_from_cwd(tmp_path, monkeypatch):
    source_dir = tmp_path / 'source'
    target_dir = tmp_path / 'target'
    source_dir.mkdir()
    target_dir.mkdir()

    personal_list = tmp_path / 'personal-singer-list.csv'
    personal_list.write_text('זמר בדיקה,זמר בדיקה\n', encoding='utf-8')

    monkeypatch.chdir(tmp_path)
    sorter = MusicSorter(source_dir=source_dir, target_dir=target_dir)

    assert ('זמר בדיקה', 'זמר בדיקה') in sorter.singer_list


def test_fix_jibrish_auto_mode_matches_legacy_first_convertible_behavior():
    assert fix_jibrish('abc àא', mode='auto') == 'abc אא'
    assert fix_jibrish('abc אà', mode='auto') == 'abc àà'

