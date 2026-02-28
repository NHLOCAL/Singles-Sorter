import pytest

from singlesorter import __VERSION__, sorter


def test_version_is_semver_like():
    parts = __VERSION__.split('.')
    assert len(parts) == 3
    assert all(part.isdigit() for part in parts)


def test_cli_help_exits_success(monkeypatch):
    monkeypatch.setattr('sys.argv', ['singlesorter', '--help'])
    with pytest.raises(SystemExit) as excinfo:
        sorter.main()
    assert excinfo.value.code == 0
