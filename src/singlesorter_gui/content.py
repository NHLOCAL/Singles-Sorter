"""Access packaged, user-facing Markdown without relying on the working directory."""

from importlib.resources import files

CONTENT_NAMES = frozenset({"help", "whats-new", "about"})


def load_content(name: str) -> str:
    if name not in CONTENT_NAMES:
        raise ValueError(f"Unknown information page: {name}")
    return files("singlesorter_gui").joinpath("content", f"{name}.md").read_text(encoding="utf-8")
