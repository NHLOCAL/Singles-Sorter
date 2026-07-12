"""Advanced settings kept outside the primary flow."""

from __future__ import annotations

import flet as ft

from ..state import SortSettings
from ..theme import BrandColors


class SettingsView:
    def __init__(self, settings: SortSettings) -> None:
        self.copy_mode = self._switch("העתקת הקבצים", settings.copy_mode)
        self.main_folder_only = self._switch("סריקת התיקייה הראשית בלבד", settings.main_folder_only)
        self.singles_folder = self._switch(
            "יצירת תיקיית ״סינגלים״ לכל אמן", settings.singles_folder
        )
        self.exist_only = self._switch("שימוש בתיקיות אמנים קיימות בלבד", settings.exist_only)
        self.abc_sort = self._switch("חלוקה לתיקיות לפי א׳–ב׳", settings.abc_sort)
        self.duet_mode = self._switch("העתקת דואטים לכל האמנים", settings.duet_mode)
        self.control = ft.Column(
            controls=[
                ft.Text("הגדרות מתקדמות", size=22, weight=ft.FontWeight.BOLD),
                ft.Text(
                    "ברירות המחדל מתאימות לרוב המשתמשים. אין צורך לשנות דבר כדי להתחיל.",
                    color=BrandColors.MUTED,
                    size=13,
                ),
                ft.Divider(),
                self.copy_mode,
                ft.Text(
                    "מומלץ להשאיר פעיל: הקבצים המקוריים לא יימחקו.",
                    size=12,
                    color=BrandColors.MUTED,
                ),
                self.main_folder_only,
                self.singles_folder,
                self.exist_only,
                self.abc_sort,
                self.duet_mode,
            ],
            spacing=10,
            rtl=True,
            scroll=ft.ScrollMode.AUTO,
        )

    @staticmethod
    def _switch(label: str, value: bool) -> ft.Switch:
        return ft.Switch(label=label, value=value, adaptive=True)

    def to_settings(self) -> SortSettings:
        return SortSettings(
            copy_mode=bool(self.copy_mode.value),
            main_folder_only=bool(self.main_folder_only.value),
            singles_folder=bool(self.singles_folder.value),
            exist_only=bool(self.exist_only.value),
            abc_sort=bool(self.abc_sort.value),
            duet_mode=bool(self.duet_mode.value),
        )


def settings_sheet(
    view: SettingsView,
    *,
    on_cancel,
    on_save,
) -> ft.BottomSheet:
    """Build a compact sheet with independently scrolling settings.

    The action row deliberately lives outside the expanded scroll area so it
    remains reachable on small Android displays and with larger text sizes.
    """

    view.control.expand = True
    actions = ft.Row(
        [
            ft.TextButton("ביטול", on_click=on_cancel),
            ft.FilledButton("שמירה", icon=ft.Icons.SAVE_OUTLINED, on_click=on_save),
        ],
        alignment=ft.MainAxisAlignment.END,
    )
    return ft.BottomSheet(
        content=ft.Container(
            content=ft.Column(
                [
                    view.control,
                    ft.Divider(height=1),
                    actions,
                ],
                spacing=12,
                expand=True,
            ),
            padding=ft.Padding.only(left=24, top=12, right=24, bottom=12),
            expand=True,
        ),
        fullscreen=True,
        dismissible=True,
        show_drag_handle=True,
        use_safe_area=True,
        scrollable=False,
    )
