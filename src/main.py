"""Flet build entry point.

Flet 0.85 resolves the configured module as a file in the root of
``tool.flet.app.path``. The maintained implementation remains in the optional
``singlesorter_gui`` package.
"""

from singlesorter_gui.main import main, run

__all__ = ["main", "run"]


if __name__ == "__main__":
    run()
