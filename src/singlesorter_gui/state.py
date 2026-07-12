"""Framework-independent state used by the Singles Sorter GUI."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Mapping


def _as_bool(value: Any, default: bool) -> bool:
    if value is None:
        return default
    if isinstance(value, str):
        return value.strip().lower() in {"1", "true", "yes", "on"}
    return bool(value)


@dataclass(frozen=True, slots=True)
class SortSettings:
    copy_mode: bool = False
    main_folder_only: bool = False
    singles_folder: bool = False
    exist_only: bool = False
    abc_sort: bool = False
    duet_mode: bool = False

    @classmethod
    def from_mapping(cls, values: Mapping[str, Any] | None) -> "SortSettings":
        values = values or {}
        defaults = cls()
        return cls(
            copy_mode=_as_bool(values.get("copy_mode"), defaults.copy_mode),
            main_folder_only=_as_bool(values.get("main_folder_only"), defaults.main_folder_only),
            singles_folder=_as_bool(values.get("singles_folder"), defaults.singles_folder),
            exist_only=_as_bool(values.get("exist_only"), defaults.exist_only),
            abc_sort=_as_bool(values.get("abc_sort"), defaults.abc_sort),
            duet_mode=_as_bool(values.get("duet_mode"), defaults.duet_mode),
        )

    def to_mapping(self) -> dict[str, bool]:
        return {
            "copy_mode": self.copy_mode,
            "main_folder_only": self.main_folder_only,
            "singles_folder": self.singles_folder,
            "exist_only": self.exist_only,
            "abc_sort": self.abc_sort,
            "duet_mode": self.duet_mode,
        }


@dataclass(frozen=True, slots=True)
class SortJob:
    source: Path | str | None
    target: Path | str | None
    settings: SortSettings = field(default_factory=SortSettings)

    def __post_init__(self) -> None:
        if isinstance(self.source, str):
            object.__setattr__(self, "source", Path(self.source))
        if isinstance(self.target, str):
            object.__setattr__(self, "target", Path(self.target))


@dataclass(frozen=True, slots=True)
class SortProgress:
    fraction: float
    message: str = "מסדרים את המוזיקה…"


@dataclass(frozen=True, slots=True)
class SortResult:
    songs_sorted: int = 0
    artist_folders_created: int = 0
    albums_processed: int = 0
    top_artists: tuple[tuple[str, int], ...] = ()
    error: str | None = None
    cancelled: bool = False
