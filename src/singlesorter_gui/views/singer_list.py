"""Adaptive personal singer-list editor."""

from __future__ import annotations

from dataclasses import dataclass

import flet as ft

from ..services.singer_list import SingerEntry


@dataclass(slots=True)
class SingerEntryRow:
    source: ft.TextField
    target: ft.TextField
    control: ft.Control


class SingerListEditor:
    def __init__(self, entries: list[SingerEntry], on_change=None) -> None:
        self.on_change = on_change
        self.rows: list[SingerEntryRow] = []
        self.rows_column = ft.Column(spacing=10)
        self.control = ft.Column(
            [self.rows_column],
            expand=True,
            scroll=ft.ScrollMode.AUTO,
        )
        for entry in entries:
            self.add_entry(entry, notify=False)
        if not entries:
            self.add_entry(notify=False)

    def add_entry(self, entry: SingerEntry | None = None, *, notify: bool = True) -> None:
        entry = entry or SingerEntry("", "")
        source = ft.TextField(
            label="שם שמופיע בקובץ",
            value=entry.source_name,
            dense=True,
            expand=True,
            text_align=ft.TextAlign.RIGHT,
        )
        target = ft.TextField(
            label="שם תיקיית האמן",
            value=entry.target_folder,
            dense=True,
            expand=True,
            text_align=ft.TextAlign.RIGHT,
        )
        row = SingerEntryRow(source=source, target=target, control=ft.Container())
        row.control = ft.Container(
            content=ft.Column(
                [
                    source,
                    target,
                    ft.TextButton(
                        "הסרת שורה",
                        icon=ft.Icons.DELETE_OUTLINE,
                        on_click=lambda _: self.remove_entry(row),
                    ),
                ],
                spacing=8,
            ),
            border=ft.Border.all(1, ft.Colors.OUTLINE_VARIANT),
            border_radius=12,
            padding=12,
        )
        self.rows.append(row)
        self.rows_column.controls.append(row.control)
        if notify and self.on_change:
            self.on_change()

    def remove_entry(self, row: SingerEntryRow) -> None:
        if row not in self.rows:
            return
        self.rows.remove(row)
        self.rows_column.controls.remove(row.control)
        if not self.rows:
            self.add_entry(notify=False)
        if self.on_change:
            self.on_change()

    def replace_entries(self, entries: list[SingerEntry]) -> None:
        self.rows.clear()
        self.rows_column.controls.clear()
        for entry in entries:
            self.add_entry(entry, notify=False)
        if not entries:
            self.add_entry(notify=False)
        if self.on_change:
            self.on_change()

    def to_entries(self) -> list[SingerEntry]:
        return [SingerEntry(row.source.value or "", row.target.value or "") for row in self.rows]


def singer_list_sheet(
    editor: SingerListEditor,
    *,
    on_close,
    on_save,
    on_import,
    on_export,
) -> ft.BottomSheet:
    return ft.BottomSheet(
        content=ft.Container(
            content=ft.Column(
                [
                    ft.Text("רשימת זמרים אישית", size=24, weight=ft.FontWeight.BOLD),
                    ft.Text(
                        "הוסיפו שם שמופיע בקובץ ואת שם התיקייה הרצוי. שורות ריקות לא יישמרו.",
                        size=13,
                    ),
                    ft.Divider(height=1),
                    editor.control,
                    ft.Row(
                        [
                            ft.TextButton("הוספת שורה", icon=ft.Icons.ADD, on_click=lambda _: editor.add_entry()),
                            ft.TextButton("ייבוא CSV", icon=ft.Icons.UPLOAD_FILE, on_click=on_import),
                            ft.TextButton("ייצוא CSV", icon=ft.Icons.DOWNLOAD, on_click=on_export),
                        ],
                        wrap=True,
                    ),
                    ft.Divider(height=1),
                    ft.Row(
                        [
                            ft.TextButton("ביטול", on_click=on_close),
                            ft.FilledButton("שמירה", icon=ft.Icons.SAVE_OUTLINED, on_click=on_save),
                        ],
                        alignment=ft.MainAxisAlignment.END,
                    ),
                ],
                expand=True,
                spacing=10,
            ),
            expand=True,
            padding=ft.Padding.only(left=20, top=10, right=20, bottom=10),
        ),
        fullscreen=True,
        dismissible=True,
        show_drag_handle=True,
        use_safe_area=True,
    )
