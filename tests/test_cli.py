import inspect

import pytest

from singlesorter import __VERSION__, sorter


def test_version_is_semver_like():
    parts = __VERSION__.split(".")
    assert len(parts) == 3
    assert all(part.isdigit() for part in parts)


def test_cli_help_exits_success(monkeypatch):
    monkeypatch.setattr("sys.argv", ["singlesorter", "--help"])
    with pytest.raises(SystemExit) as excinfo:
        sorter.main()
    assert excinfo.value.code == 0


def test_cli_defaults_to_move_without_inner_singles_folder(monkeypatch, capsys):
    parameters = inspect.signature(sorter.MusicSorter).parameters

    assert parameters["copy_mode"].default is False
    assert parameters["singles_folder"].default is False

    monkeypatch.setattr("sys.argv", ["singlesorter", "--help"])
    with pytest.raises(SystemExit):
        sorter.main()

    assert "--singles-dir" in capsys.readouterr().out
