"""Unsafe legacy mechanics, recorded separately from desired routing semantics.

These are observations of the old engine on owned disposable fixtures, not
requirements that a new safe transfer layer must reproduce destructive behavior.
"""
import logging

import pytest

import singles_sorter_v5 as legacy


@pytest.fixture
def sorter(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    (tmp_path / "app").mkdir()
    (tmp_path / "app/singer-list.csv").write_text("אברהם פריד,אברהם פריד\n", encoding="utf-8")
    source, target = tmp_path / "source", tmp_path / "target"
    source.mkdir()
    target.mkdir()
    monkeypatch.setattr(legacy, "ai_invalid", True)
    return legacy.MusicSorter(source, target, singles_folder=False, logger=logging.getLogger("io-observation"))


def test_duet_move_deletes_source_when_every_target_is_skipped(sorter, monkeypatch):
    sorter.duet_mode = True
    sorter.exist_only = True
    file = sorter.source_dir / "fixture.mp3"
    file.write_bytes(b"owned disposable placeholder")
    monkeypatch.setattr(sorter, "artists_from_song", lambda _: ["אברהם פריד", "מוטי שטיינמץ"])
    summary = sorter.scan_dir()
    assert summary["songs_sorted"] == 0
    assert not file.exists()
    assert list(sorter.target_dir.iterdir()) == []


def test_failed_move_still_increments_success_count(sorter, monkeypatch):
    file = sorter.source_dir / "fixture.mp3"
    file.write_bytes(b"owned disposable placeholder")
    monkeypatch.setattr(sorter, "artists_from_song", lambda _: ["אברהם פריד"])
    monkeypatch.setattr(sorter, "move_file", lambda *_: False)
    assert sorter.scan_dir()["songs_sorted"] == 1
    assert file.exists()
    assert not (sorter.target_dir / "אברהם פריד/fixture.mp3").exists()
