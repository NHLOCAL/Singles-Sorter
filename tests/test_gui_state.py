from pathlib import Path

from singlesorter_gui.state import SortJob, SortSettings
from singlesorter_gui.validation import validate_job


def test_settings_use_safe_copy_default():
    assert SortSettings().copy_mode is True


def test_settings_migrate_missing_and_string_values():
    settings = SortSettings.from_mapping({"abc_sort": "true", "copy_mode": None})

    assert settings.copy_mode is True
    assert settings.abc_sort is True


def test_validation_requires_both_folders(tmp_path):
    errors = validate_job(SortJob(source=None, target=None))

    assert errors == ("יש לבחור תיקיית מוזיקה.", "יש לבחור תיקיית יעד.")


def test_validation_rejects_identical_folders(tmp_path):
    errors = validate_job(SortJob(source=tmp_path, target=tmp_path))

    assert "תיקיית המוזיקה ותיקיית היעד חייבות להיות שונות." in errors


def test_validation_accepts_existing_distinct_folders(tmp_path):
    source = tmp_path / "source"
    target = tmp_path / "target"
    source.mkdir()
    target.mkdir()
    (source / "song.mp3").write_bytes(b"audio")

    assert validate_job(SortJob(source=source, target=target)) == ()


def test_validation_rejects_empty_source_folder(tmp_path):
    source = tmp_path / "source"
    target = tmp_path / "target"
    source.mkdir()
    target.mkdir()

    assert "תיקיית המוזיקה ריקה." in validate_job(SortJob(source=source, target=target))


def test_validation_rejects_overlapping_folders(tmp_path):
    source = tmp_path / "source"
    target = source / "sorted"
    source.mkdir()
    target.mkdir()
    (source / "song.mp3").write_bytes(b"audio")

    errors = validate_job(SortJob(source=source, target=target))

    assert "תיקיית היעד אינה יכולה להיות בתוך תיקיית המוזיקה." in errors


def test_job_normalizes_string_paths(tmp_path):
    job = SortJob(source=str(tmp_path / "source"), target=str(tmp_path / "target"))

    assert isinstance(job.source, Path)
