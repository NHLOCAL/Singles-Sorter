"""Compatibility launcher for historical Flet build commands.

The maintained application lives in :mod:`singlesorter_gui`. Keeping this
small adapter lets existing shortcuts continue to work without maintaining a
second UI implementation.
"""

from singlesorter_gui.main import main, run

__all__ = ["main", "run"]


if __name__ == "__main__":
    run()
