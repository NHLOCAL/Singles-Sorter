"""Adapter between GUI state and the framework-independent sorting engine."""

from __future__ import annotations

from threading import Event
from typing import Callable

from singlesorter.sorter import MusicSorter

from ..state import SortJob, SortProgress, SortResult


class CancellationToken:
    def __init__(self) -> None:
        self._event = Event()

    @property
    def cancelled(self) -> bool:
        return self._event.is_set()

    def cancel(self) -> None:
        self._event.set()


class SortService:
    def __init__(self, sorter_factory: Callable[..., MusicSorter] = MusicSorter) -> None:
        self._sorter_factory = sorter_factory

    def run(
        self,
        job: SortJob,
        on_progress: Callable[[SortProgress], None] | None = None,
        cancellation: CancellationToken | None = None,
    ) -> SortResult:
        last_fraction = -1.0

        def report(percent: float) -> None:
            nonlocal last_fraction
            fraction = min(1.0, max(0.0, float(percent) / 100.0))
            if fraction == last_fraction:
                return
            if 0 <= last_fraction < fraction < 1.0 and fraction - last_fraction < 0.005:
                return
            last_fraction = fraction
            if on_progress:
                on_progress(SortProgress(fraction=fraction))

        settings = job.settings
        sorter_kwargs = dict(
            source_dir=job.source,
            target_dir=job.target,
            copy_mode=settings.copy_mode,
            abc_sort=settings.abc_sort,
            exist_only=settings.exist_only,
            singles_folder=settings.singles_folder,
            main_folder_only=settings.main_folder_only,
            duet_mode=settings.duet_mode,
            progress_callback=report,
        )
        if cancellation is not None:
            sorter_kwargs["cancel_check"] = lambda: cancellation.cancelled
        sorter = self._sorter_factory(**sorter_kwargs)
        if cancellation and cancellation.cancelled:
            return SortResult(cancelled=True)
        summary = sorter.scan_dir()
        return SortResult(
            songs_sorted=int(summary.get("songs_sorted", 0)),
            artist_folders_created=int(summary.get("artist_folders_created", 0)),
            albums_processed=int(summary.get("albums_processed", 0)),
            top_artists=tuple(summary.get("top_artists", ())),
            cancelled=bool(cancellation and cancellation.cancelled),
        )
