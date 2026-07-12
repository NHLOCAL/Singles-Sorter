"""Dialog builders for review, progress, and completion states."""

from __future__ import annotations

import flet as ft

from ..state import SortJob, SortResult
from ..theme import BrandColors


def review_dialog(job: SortJob, on_confirm, on_cancel) -> ft.AlertDialog:
    operation = "העתקה בטוחה" if job.settings.copy_mode else "העברה"
    return ft.AlertDialog(
        modal=True,
        title="רגע לפני שמתחילים",
        icon=ft.Icon(ft.Icons.FACT_CHECK_OUTLINED, color=BrandColors.STEEL),
        content=ft.Column(
            [
                _summary_row("מקור", str(job.source)),
                _summary_row("יעד", str(job.target)),
                _summary_row("אופן פעולה", operation),
                ft.Text(
                    "הקבצים המקוריים יישארו במקומם."
                    if job.settings.copy_mode
                    else "הקבצים יועברו מהתיקייה המקורית.",
                    color=BrandColors.MUTED,
                    size=13,
                ),
            ],
            tight=True,
            spacing=12,
            rtl=True,
        ),
        actions=[
            ft.TextButton("חזרה", on_click=on_cancel),
            ft.FilledButton("התחלה", icon=ft.Icons.PLAY_ARROW_ROUNDED, on_click=on_confirm),
        ],
        actions_alignment=ft.MainAxisAlignment.END,
    )


def result_dialog(result: SortResult, on_close) -> ft.AlertDialog:
    if result.cancelled:
        title = "הפעולה הופסקה"
        icon = ft.Icons.CANCEL_OUTLINED
    elif result.error:
        title = "לא הצלחנו להשלים את המיון"
        icon = ft.Icons.ERROR_OUTLINE_ROUNDED
    else:
        title = "הפעולה הושלמה" if result.message else "המוזיקה מסודרת"
        icon = ft.Icons.CHECK_CIRCLE_OUTLINE_ROUNDED
    content = result.error or result.message or (
        f"הועתקו {result.songs_sorted} שירים, נוצרו "
        f"{result.artist_folders_created} תיקיות אמנים וטופלו {result.albums_processed} אלבומים."
    )
    return ft.AlertDialog(
        title=title,
        icon=ft.Icon(icon, color=BrandColors.STEEL, size=42),
        content=ft.Text(content, text_align=ft.TextAlign.CENTER, rtl=True),
        actions=[ft.FilledButton("סיום", on_click=on_close)],
        actions_alignment=ft.MainAxisAlignment.CENTER,
    )


def _summary_row(label: str, value: str) -> ft.Column:
    return ft.Column(
        [
            ft.Text(label, size=12, color=BrandColors.MUTED),
            ft.Text(value, size=14, weight=ft.FontWeight.W_500, selectable=True),
        ],
        spacing=2,
    )
