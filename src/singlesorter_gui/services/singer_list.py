"""Persistent personal singer aliases shared by the GUI and sorting engine."""

from __future__ import annotations

import csv
import io
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True, slots=True)
class SingerEntry:
    source_name: str
    target_folder: str


def default_personal_list_path() -> Path:
    override = os.getenv("SINGLESORTER_PERSONAL_LIST")
    if override:
        return Path(override).expanduser()
    local_data = os.getenv("LOCALAPPDATA")
    root = Path(local_data) / "SinglesSorter" if local_data else Path.home() / ".singlesorter"
    return root / "personal-singer-list.csv"


class SingerListStore:
    def __init__(self, path: Path | str | None = None) -> None:
        self.path = Path(path) if path is not None else default_personal_list_path()

    def load(self) -> list[SingerEntry]:
        if not self.path.exists():
            return []
        return self._read(self.path)

    def save(self, entries: Iterable[SingerEntry]) -> int:
        cleaned = self._clean(entries)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("w", encoding="utf-8-sig", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerows((entry.source_name, entry.target_folder) for entry in cleaned)
        return len(cleaned)

    def import_csv(self, source: Path | str) -> int:
        existing = self.load()
        imported = self._read(Path(source))
        return self._merge_import(existing, imported)

    def import_bytes(self, raw: bytes) -> int:
        existing = self.load()
        imported = self._entries_from_bytes(raw)
        return self._merge_import(existing, imported)

    def _merge_import(self, existing: list[SingerEntry], imported: list[SingerEntry]) -> int:
        merged = self._clean([*existing, *imported])
        added = len(merged) - len(self._clean(existing))
        self.save(merged)
        return added

    def export_csv(self, destination: Path | str) -> Path:
        destination = Path(destination)
        if destination.suffix.lower() != ".csv":
            destination = destination.with_suffix(".csv")
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("w", encoding="utf-8-sig", newline="") as stream:
            writer = csv.writer(stream)
            writer.writerows((entry.source_name, entry.target_folder) for entry in self.load())
        return destination

    def to_csv_bytes(self) -> bytes:
        stream = io.StringIO(newline="")
        writer = csv.writer(stream)
        writer.writerows((entry.source_name, entry.target_folder) for entry in self.load())
        return ("\ufeff" + stream.getvalue()).encode("utf-8")

    @classmethod
    def _read(cls, path: Path) -> list[SingerEntry]:
        return cls._entries_from_bytes(path.read_bytes())

    @classmethod
    def _entries_from_bytes(cls, raw: bytes) -> list[SingerEntry]:
        decoded = None
        for encoding in ("utf-8-sig", "cp1255"):
            try:
                decoded = raw.decode(encoding)
                break
            except UnicodeDecodeError:
                continue
        if decoded is None:
            decoded = raw.decode("utf-8", errors="replace")
        rows = csv.reader(decoded.splitlines())
        return cls._clean(
            SingerEntry(row[0], row[1]) for row in rows if len(row) >= 2
        )

    @staticmethod
    def _clean(entries: Iterable[SingerEntry]) -> list[SingerEntry]:
        result: list[SingerEntry] = []
        seen: set[tuple[str, str]] = set()
        for entry in entries:
            source = entry.source_name.strip()
            target = entry.target_folder.strip()
            if not source or not target:
                continue
            key = (source.casefold(), target.casefold())
            if key in seen:
                continue
            seen.add(key)
            result.append(SingerEntry(source, target))
        return result
