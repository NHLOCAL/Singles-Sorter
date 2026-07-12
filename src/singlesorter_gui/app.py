"""Application controller for the modern Flet UI."""

from __future__ import annotations

import asyncio
import os
from pathlib import Path

import flet as ft
import flet_permission_handler as fph

from singlesorter import __VERSION__

from .content import load_content
from .services.permissions import required_android_permissions
from .services.singer_list import SingerListStore
from .services.sorting import CancellationToken, SortService
from .services.updates import ReleaseInfo, check_latest_release, should_show_whats_new
from .state import SortJob, SortProgress, SortResult, SortSettings
from .theme import BrandColors, build_dark_theme, build_theme
from .validation import validate_job
from .views.dialogs import result_dialog, review_dialog
from .views.home import HomeView
from .views.information import information_sheet
from .views.settings import SettingsView, settings_sheet
from .views.singer_list import SingerListEditor, singer_list_sheet


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
        self.singer_store = SingerListStore()
        os.environ["SINGLESORTER_PERSONAL_LIST"] = str(self.singer_store.path)
        self.singer_editor: SingerListEditor | None = None
        self.home = HomeView(
            on_pick_source=self.pick_source,
            on_pick_target=self.pick_target,
            on_start=self.show_review,
            on_settings=self.show_settings,
            on_theme=self.toggle_theme,
            on_menu=self.handle_menu,
            on_fix_names=self.show_fix_names_warning,
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
        last_seen = await self.preferences.get("singlesorter.last_whats_new")
        if should_show_whats_new(last_seen, __VERSION__):
            self.show_information("whats-new")
            await self.preferences.set("singlesorter.last_whats_new", __VERSION__)
        self.page.run_task(self._check_updates, False)

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

    def handle_menu(self, key: str) -> None:
        if key in {"help", "whats-new", "about"}:
            self.show_information(key)
        elif key == "singers":
            self.show_singer_list()
        elif key == "update":
            self.page.run_task(self._check_updates, True)

    def show_information(self, key: str) -> None:
        pages = {
            "help": ("עזרה", ft.Icons.HELP_OUTLINE),
            "whats-new": ("מה חדש", ft.Icons.NEW_RELEASES_OUTLINED),
            "about": ("אודות התוכנה", ft.Icons.INFO_OUTLINE),
        }
        title, icon = pages[key]
        self.page.show_dialog(
            information_sheet(
                title=title,
                markdown=load_content(key),
                icon=icon,
                on_close=lambda _: self.page.pop_dialog(),
            )
        )

    def show_singer_list(self) -> None:
        self.singer_editor = SingerListEditor(
            self.singer_store.load(),
            on_change=lambda: self.page.update(self.singer_editor.control),
        )
        self.page.show_dialog(
            singer_list_sheet(
                self.singer_editor,
                on_close=lambda _: self.page.pop_dialog(),
                on_save=lambda _: self._save_singer_list(),
                on_import=lambda _: self.page.run_task(self._import_singer_list),
                on_export=lambda _: self.page.run_task(self._export_singer_list),
            )
        )

    def _save_singer_list(self) -> None:
        if self.singer_editor is None:
            return
        self.singer_store.save(self.singer_editor.to_entries())
        self.page.pop_dialog()

    async def _import_singer_list(self) -> None:
        selected = await self.file_picker.pick_files(
            dialog_title="ייבוא רשימת זמרים אישית",
            file_type=ft.FilePickerFileType.CUSTOM,
            allowed_extensions=["csv"],
            allow_multiple=False,
            with_data=True,
        )
        if not selected:
            return
        file = selected[0]
        if file.path:
            added = self.singer_store.import_csv(file.path)
        elif file.bytes:
            added = self.singer_store.import_bytes(file.bytes)
        else:
            return
        if self.singer_editor:
            self.singer_editor.replace_entries(self.singer_store.load())
        self._show_notice(f"נוספו {added} רשומות חדשות.")

    async def _export_singer_list(self) -> None:
        destination = await self.file_picker.save_file(
            dialog_title="ייצוא רשימת זמרים אישית",
            file_name="personal-singer-list.csv",
            file_type=ft.FilePickerFileType.CUSTOM,
            allowed_extensions=["csv"],
            src_bytes=self.singer_store.to_csv_bytes(),
        )
        if destination:
            self.singer_store.export_csv(destination)
        self._show_notice("רשימת הזמרים יוצאה בהצלחה.")

    def _show_notice(self, message: str) -> None:
        self.page.show_dialog(
            ft.SnackBar(
                content=ft.Text(message),
                show_close_icon=True,
            )
        )

    async def _check_updates(self, show_result: bool) -> None:
        release = await asyncio.to_thread(check_latest_release, __VERSION__)
        if release:
            self.home.more_menu.badge = ft.Badge(small_size=9, bgcolor=ft.Colors.RED)
            self.page.update(self.home.more_menu)
            if show_result:
                self._show_update_dialog(release)
        elif show_result:
            self._show_notice("מותקנת הגרסה העדכנית ביותר.")

    def _show_update_dialog(self, release: ReleaseInfo) -> None:
        self.page.show_dialog(
            ft.AlertDialog(
                title=f"גרסה {release.version} זמינה",
                icon=ft.Icon(ft.Icons.SYSTEM_UPDATE_OUTLINED),
                content=ft.Column(
                    [ft.Text("מה חדש", weight=ft.FontWeight.BOLD), ft.Markdown(release.notes)],
                    tight=True,
                    scroll=ft.ScrollMode.AUTO,
                ),
                actions=[
                    ft.TextButton("סגירה", on_click=lambda _: self.page.pop_dialog()),
                    ft.FilledButton(
                        "פתיחת דף ההורדה",
                        on_click=lambda _: self.page.launch_url(release.url),
                    ),
                ],
            )
        )

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

    def show_fix_names_warning(self, _=None) -> None:
        if not self.source or not self.source.exists():
            self.page.show_dialog(
                ft.AlertDialog(
                    title="יש לבחור תיקיית מוזיקה",
                    content=ft.Text("בחרו תחילה את התיקייה שבה נמצאים הקבצים לתיקון."),
                    actions=[ft.FilledButton("הבנתי", on_click=lambda _: self.page.pop_dialog())],
                )
            )
            return
        self.page.show_dialog(
            ft.AlertDialog(
                modal=True,
                title="תיקון שמות ותגיות",
                icon=ft.Icon(ft.Icons.DRIVE_FILE_RENAME_OUTLINE),
                content=ft.Text(
                    "הפעולה עשויה לשנות שמות קבצים ותגיות מוזיקה. מומלץ לגבות את התיקייה לפני ההמשך."
                ),
                actions=[
                    ft.TextButton("ביטול", on_click=lambda _: self.page.pop_dialog()),
                    ft.FilledButton(
                        "המשך",
                        on_click=lambda _: self.page.run_task(self.start_fix_names),
                    ),
                ],
            )
        )

    async def start_fix_names(self) -> None:
        self.page.pop_dialog()
        self.cancellation = CancellationToken()
        self.progress_dialog.title = "מתקנים שמות ותגיות"
        self.progress_bar.value = 0
        self.progress_text.value = "סורקים את קובצי המוזיקה…"
        self.page.show_dialog(self.progress_dialog)
        try:
            result = await asyncio.to_thread(
                self.service.fix_names,
                self.source,
                self._on_progress,
                self.cancellation,
                main_folder_only=self.settings.main_folder_only,
            )
        except Exception as error:
            result = SortResult(error=self._friendly_error(error))
        self.page.pop_dialog()
        self.page.show_dialog(result_dialog(result, lambda _: self.page.pop_dialog()))

    async def start_sort(self, job: SortJob) -> None:
        self.page.pop_dialog()
        self.cancellation = CancellationToken()
        self.progress_dialog.title = "מסדרים את המוזיקה"
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
