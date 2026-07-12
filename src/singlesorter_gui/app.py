"""Application controller for the modern Flet UI."""

from __future__ import annotations

import asyncio
from pathlib import Path

import flet as ft
import flet_permission_handler as fph

from .services.permissions import required_android_permissions
from .services.sorting import CancellationToken, SortService
from .state import SortJob, SortProgress, SortResult, SortSettings
from .theme import BrandColors, build_dark_theme, build_theme
from .validation import validate_job
from .views.dialogs import result_dialog, review_dialog
from .views.home import HomeView
from .views.settings import SettingsView, settings_sheet


class SinglesSorterApp:
    SETTINGS_PREFIX = "singlesorter.settings."

    def __init__(self, page: ft.Page, service: SortService | None = None) -> None:
        self.page = page
        self.service = service or SortService()
        self.settings = SortSettings()
        self.source: Path | None = None
        self.target: Path | None = None
        self.cancellation: CancellationToken | None = None
        self.file_picker = ft.FilePicker()
        self.preferences = ft.SharedPreferences()
        self.permission_handler = fph.PermissionHandler()
        self.home = HomeView(
            on_pick_source=self.pick_source,
            on_pick_target=self.pick_target,
            on_start=self.show_review,
            on_settings=self.show_settings,
            on_theme=self.toggle_theme,
        )
        self.progress_bar = ft.ProgressBar(value=0, color=BrandColors.GOLD)
        self.progress_text = ft.Text("מתחילים…", text_align=ft.TextAlign.CENTER)
        self.progress_dialog = ft.AlertDialog(
            modal=True,
            title="מסדרים את המוזיקה",
            content=ft.Column(
                [self.progress_text, self.progress_bar],
                tight=True,
                spacing=16,
                width=360,
            ),
            actions=[ft.TextButton("ביטול", on_click=self.cancel_sort)],
        )

    async def mount(self) -> None:
        self._configure_page()
        self.settings = await self._load_settings()
        self.page.add(
            ft.SafeArea(
                ft.ResponsiveRow(
                    [self.home.control],
                    alignment=ft.MainAxisAlignment.CENTER,
                    vertical_alignment=ft.CrossAxisAlignment.START,
                    expand=True,
                ),
                expand=True,
            )
        )

    def _configure_page(self) -> None:
        self.page.title = "מסדר הסינגלים"
        self.page.rtl = True
        self.page.adaptive = True
        self.page.theme = build_theme()
        self.page.dark_theme = build_dark_theme()
        self.page.theme_mode = ft.ThemeMode.LIGHT
        self.page.bgcolor = BrandColors.CANVAS
        self.page.padding = 0
        self.page.window.min_width = 360
        self.page.window.min_height = 640

    async def pick_source(self, _=None) -> None:
        if not await self._ensure_android_permissions():
            return
        selected = await self.file_picker.get_directory_path(dialog_title="בחירת תיקיית המוזיקה")
        if selected:
            self.source = Path(selected)
            self._refresh_paths()

    async def pick_target(self, _=None) -> None:
        if not await self._ensure_android_permissions():
            return
        selected = await self.file_picker.get_directory_path(dialog_title="בחירת תיקיית היעד")
        if selected:
            self.target = Path(selected)
            self._refresh_paths()

    async def _ensure_android_permissions(self) -> bool:
        if self.page.platform != ft.PagePlatform.ANDROID:
            return True

        statuses = []
        for permission in required_android_permissions():
            statuses.append(await self.permission_handler.request(permission))

        allowed = {fph.PermissionStatus.GRANTED, fph.PermissionStatus.LIMITED}
        if all(status in allowed for status in statuses):
            return True

        permanently_denied = fph.PermissionStatus.PERMANENTLY_DENIED in statuses
        actions = [ft.TextButton("סגירה", on_click=lambda _: self.page.pop_dialog())]
        if permanently_denied:
            actions.append(
                ft.FilledButton(
                    "פתיחת הגדרות",
                    on_click=lambda _: self.page.run_task(self._open_permission_settings),
                )
            )
        self.page.show_dialog(
            ft.AlertDialog(
                title="נדרשת הרשאת אחסון",
                icon=ft.Icon(ft.Icons.FOLDER_OFF_OUTLINED),
                content=ft.Text(
                    "כדי לבחור ולסדר תיקיות מוזיקה, יש לאפשר גישה לקובצי השמע ולאחסון.",
                    rtl=True,
                ),
                actions=actions,
            )
        )
        return False

    async def _open_permission_settings(self) -> None:
        await self.permission_handler.open_app_settings()
        self.page.pop_dialog()

    def _refresh_paths(self) -> None:
        self.home.set_paths(
            str(self.source) if self.source else None,
            str(self.target) if self.target else None,
        )
        self.page.update(self.home.source_path, self.home.target_path, self.home.start_button)

    async def show_settings(self, _=None) -> None:
        view = SettingsView(self.settings)
        sheet = settings_sheet(
            view,
            on_cancel=lambda _: self.page.pop_dialog(),
            on_save=lambda _: self.page.run_task(self._save_settings_and_close, view),
        )
        self.page.show_dialog(sheet)

    async def _save_settings_and_close(self, view: SettingsView) -> None:
        self.settings = view.to_settings()
        for key, value in self.settings.to_mapping().items():
            await self.preferences.set(f"{self.SETTINGS_PREFIX}{key}", value)
        self.page.pop_dialog()

    async def _load_settings(self) -> SortSettings:
        values: dict[str, object] = {}
        for key in SortSettings().to_mapping():
            value = await self.preferences.get(f"{self.SETTINGS_PREFIX}{key}")
            if value is not None:
                values[key] = value
        return SortSettings.from_mapping(values)

    def show_review(self, _=None) -> None:
        job = SortJob(self.source, self.target, self.settings)
        errors = validate_job(job)
        if errors:
            self.page.show_dialog(
                ft.AlertDialog(
                    title="צריך להשלים עוד פרט קטן",
                    icon=ft.Icon(ft.Icons.INFO_OUTLINE_ROUNDED),
                    content=ft.Text("\n".join(errors), rtl=True),
                    actions=[ft.FilledButton("הבנתי", on_click=lambda _: self.page.pop_dialog())],
                )
            )
            return
        dialog = review_dialog(
            job,
            on_confirm=lambda _: self.page.run_task(self.start_sort, job),
            on_cancel=lambda _: self.page.pop_dialog(),
        )
        self.page.show_dialog(dialog)

    async def start_sort(self, job: SortJob) -> None:
        self.page.pop_dialog()
        self.cancellation = CancellationToken()
        self.progress_bar.value = 0
        self.progress_text.value = "סורקים ומזהים את השירים…"
        self.page.show_dialog(self.progress_dialog)
        try:
            result = await asyncio.to_thread(
                self.service.run,
                job,
                self._on_progress,
                self.cancellation,
            )
        except Exception as error:
            result = SortResult(error=self._friendly_error(error))
        self.page.pop_dialog()
        self.page.show_dialog(result_dialog(result, lambda _: self.page.pop_dialog()))

    def _on_progress(self, progress: SortProgress) -> None:
        self.page.run_task(self._apply_progress, progress)

    async def _apply_progress(self, progress: SortProgress) -> None:
        self.progress_bar.value = progress.fraction
        self.progress_text.value = f"{round(progress.fraction * 100)}% — {progress.message}"
        self.page.update(self.progress_bar, self.progress_text)

    def cancel_sort(self, _=None) -> None:
        if self.cancellation:
            self.cancellation.cancel()
            self.progress_text.value = "עוצרים בבטחה אחרי הקובץ הנוכחי…"
            self.page.update(self.progress_text)

    def toggle_theme(self, _=None) -> None:
        self.page.theme_mode = (
            ft.ThemeMode.DARK if self.page.theme_mode != ft.ThemeMode.DARK else ft.ThemeMode.LIGHT
        )
        self.page.bgcolor = (
            None if self.page.theme_mode == ft.ThemeMode.DARK else BrandColors.CANVAS
        )
        self.page.update()

    @staticmethod
    def _friendly_error(error: Exception) -> str:
        if isinstance(error, PermissionError):
            return "אין הרשאה לקרוא או לכתוב באחת התיקיות. בדקו את הרשאות האחסון ונסו שוב."
        if isinstance(error, FileNotFoundError):
            return "אחת התיקיות כבר אינה זמינה. בחרו אותה מחדש ונסו שוב."
        if isinstance(error, ValueError):
            return str(error)
        return "אירעה שגיאה בזמן המיון. הקבצים המקוריים לא נמחקו; אפשר לנסות שוב."
