"""Executable entry point for the optional Flet GUI."""

from __future__ import annotations

from pathlib import Path

import flet as ft

from .app import SinglesSorterApp


async def main(page: ft.Page) -> None:
    await SinglesSorterApp(page).mount()


def run() -> None:
    # Keep the web mount path empty. In Flet 0.85.3 a non-ASCII ``name`` is
    # used as a URL prefix and produces an incorrect WebSocket endpoint.
    # The visible product name is configured on Page and in pyproject.toml.
    assets_dir = Path(__file__).resolve().parent / "assets"
    ft.run(main, assets_dir=str(assets_dir))


if __name__ == "__main__":
    run()
