import re
from pathlib import Path

import tomllib


def project_config():
    return tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))["project"]


def test_base_dependencies_do_not_include_flet():
    assert not any(item.lower().startswith("flet") for item in project_config()["dependencies"])


def test_runtime_version_matches_package_version():
    from singlesorter import __VERSION__

    assert __VERSION__ == project_config()["version"]


def test_gui_extra_pins_current_stable_flet():
    gui = project_config()["optional-dependencies"]["gui"]
    assert "flet[all]==0.85.3" in gui
    assert "flet-permission-handler==0.85.3" in gui


def test_gui_has_separate_entry_point():
    assert project_config()["scripts"]["singlesorter-gui"] == "singlesorter_gui.main:run"


def test_flet_build_uses_root_entry_module():
    config = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))

    assert config["tool"]["flet"]["app"]["path"] == "src"
    assert config["tool"]["flet"]["app"]["module"] == "main"
    assert Path("src/main.py").is_file()


def test_android_organization_is_a_valid_java_namespace():
    config = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))
    organization = config["tool"]["flet"]["org"]

    assert all(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", part) for part in organization.split("."))


def test_gui_icon_is_declared_as_package_data():
    config = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))

    assert "assets/*.png" in config["tool"]["setuptools"]["package-data"]["singlesorter_gui"]
    assert Path("src/singlesorter_gui/assets/icon.png").is_file()
