"""The simple primary sorting flow."""

from __future__ import annotations

from collections.abc import Callable

import flet as ft

from singlesorter import __VERSION__

from ..theme import CARD_RADIUS, TOUCH_HEIGHT, BrandColors
from ..tips import daily_tip


class HomeView:
    def __init__(
        self,
        on_pick_source: Callable[[object], None],
        on_pick_target: Callable[[object], None],
        on_start: Callable[[object], None] | None = None,
        on_settings: Callable[[object], None] | None = None,
        on_theme: Callable[[object], None] | None = None,
        on_menu: Callable[[str], None] | None = None,
        on_fix_names: Callable[[object], None] | None = None,
    ) -> None:
        self.source_path = ft.Text("לא נבחרה תיקייה", color=BrandColors.MUTED, size=13)
        self.target_path = ft.Text("לא נבחרה תיקייה", color=BrandColors.MUTED, size=13)
        self.start_button = ft.FilledButton(
            "סדר את המוזיקה",
            icon=ft.Icons.AUTO_AWESOME_ROUNDED,
            disabled=True,
            height=58,
            bgcolor=BrandColors.GOLD,
            color=BrandColors.NAVY_DARK,
            style=ft.ButtonStyle(
                shape=ft.RoundedRectangleBorder(radius=16),
                text_style=ft.TextStyle(size=18, weight=ft.FontWeight.BOLD),
            ),
            on_click=on_start,
        )
        self.fix_names_button = ft.OutlinedButton(
            "תיקון שמות ותגיות",
            icon=ft.Icons.DRIVE_FILE_RENAME_OUTLINE,
            disabled=True,
            height=50,
            on_click=on_fix_names,
        )
        self.control = ft.Container(
            content=ft.Column(
                controls=[
                    self._header(on_settings, on_theme, on_menu),
                    ft.Container(height=6),
                    ft.Text(
                        "מסדרים את המוזיקה. בפשטות.",
                        size=26,
                        weight=ft.FontWeight.BOLD,
                        color=BrandColors.NAVY,
                        text_align=ft.TextAlign.RIGHT,
                    ),
                    ft.Text(
                    "בחרו מאיפה לקחת את השירים ולאן להעביר אותם — אנחנו נטפל בשאר.",
                        size=15,
                        color=BrandColors.MUTED,
                        text_align=ft.TextAlign.RIGHT,
                    ),
                    ft.Container(height=8),
                    self._folder_card(
                        number="1",
                        title="מאיפה לקחת את המוזיקה?",
                        subtitle="התיקייה שבה נמצאים השירים",
                        path=self.source_path,
                        button_text="בחירת תיקייה",
                        on_click=on_pick_source,
                    ),
                    self._folder_card(
                        number="2",
            title="לאן להעביר אותה?",
                        subtitle="כאן ייווצרו תיקיות האמנים המסודרות",
                        path=self.target_path,
                        button_text="בחירת יעד",
                        on_click=on_pick_target,
                    ),
                    ft.Container(
                        content=ft.Row(
                            controls=[
                                ft.Icon(ft.Icons.SHIELD_OUTLINED, color=BrandColors.STEEL, size=20),
                                ft.Text(
                                    "ברירת המחדל היא העברה לתיקיות המסודרות. מומלץ לגבות מראש.",
                                    color=BrandColors.MUTED,
                                    size=13,
                                    expand=True,
                                    text_align=ft.TextAlign.RIGHT,
                                ),
                            ],
                            rtl=True,
                            spacing=10,
                        ),
                        padding=ft.Padding.symmetric(horizontal=4, vertical=6),
                    ),
                    ft.Container(
                        content=ft.Row(
                            [
                                ft.Icon(ft.Icons.LIGHTBULB_OUTLINE, color=BrandColors.GOLD, size=20),
                                ft.Text(daily_tip(), size=12, color=BrandColors.MUTED, expand=True),
                            ],
                            rtl=True,
                        ),
                        padding=ft.Padding.symmetric(horizontal=4, vertical=2),
                    ),
                    self.start_button,
                    self.fix_names_button,
                ],
                spacing=10,
                horizontal_alignment=ft.CrossAxisAlignment.STRETCH,
                scroll=ft.ScrollMode.AUTO,
            ),
            col={"xs": 12, "sm": 12, "md": 10, "lg": 7, "xl": 7},
            padding=ft.Padding.symmetric(horizontal=20, vertical=14),
            alignment=ft.Alignment.TOP_CENTER,
        )

    def _header(self, on_settings, on_theme, on_menu) -> ft.Row:
        self.version_text = ft.Text(f"גרסה {__VERSION__}", size=11, color=BrandColors.MUTED)
        menu_entries = [
            ("help", "עזרה", ft.Icons.HELP_OUTLINE),
            ("whats-new", "מה חדש", ft.Icons.NEW_RELEASES_OUTLINED),
            ("about", "אודות התוכנה", ft.Icons.INFO_OUTLINE),
            ("singers", "רשימת זמרים אישית", ft.Icons.GROUP_OUTLINED),
            ("update", "בדיקת עדכונים", ft.Icons.SYSTEM_UPDATE_OUTLINED),
        ]
        self.more_menu = ft.PopupMenuButton(
            icon=ft.Icons.MORE_VERT_ROUNDED,
            tooltip="אפשרויות נוספות",
            items=[
                ft.PopupMenuItem(
                    content=label,
                    icon=icon,
                    data=key,
                    on_click=(lambda _, key=key: on_menu(key)) if on_menu else None,
                )
                for key, label, icon in menu_entries
            ],
        )
        mark = ft.Container(
            content=ft.Image(src="icon.png", fit=ft.BoxFit.COVER),
            width=44,
            height=44,
            border_radius=14,
            clip_behavior=ft.ClipBehavior.ANTI_ALIAS,
            alignment=ft.Alignment.CENTER,
        )
        return ft.Row(
            controls=[
                mark,
                ft.Column(
                    [
                        ft.Text("מסדר הסינגלים", size=19, weight=ft.FontWeight.BOLD),
                        ft.Row(
                            [ft.Text("Singles Sorter", size=12, color=BrandColors.MUTED), self.version_text],
                            spacing=8,
                            rtl=True,
                        ),
                    ],
                    spacing=0,
                    expand=True,
                ),
                ft.IconButton(
                    icon=ft.Icons.DARK_MODE_OUTLINED,
                    tooltip="החלפת ערכת נושא",
                    on_click=on_theme,
                ),
                ft.IconButton(
                    icon=ft.Icons.TUNE_ROUNDED,
                    tooltip="הגדרות מתקדמות",
                    on_click=on_settings,
                ),
                self.more_menu,
            ],
            rtl=True,
            spacing=8,
            vertical_alignment=ft.CrossAxisAlignment.CENTER,
        )

    def _folder_card(
        self,
        *,
        number: str,
        title: str,
        subtitle: str,
        path: ft.Text,
        button_text: str,
        on_click,
    ) -> ft.Container:
        return ft.Container(
            content=ft.Column(
                controls=[
                    ft.Row(
                        controls=[
                            ft.Container(
                                content=ft.Text(
                                    number,
                                    weight=ft.FontWeight.BOLD,
                                    color=BrandColors.NAVY_DARK,
                                ),
                                width=34,
                                height=34,
                                bgcolor=BrandColors.GOLD,
                                border_radius=17,
                                alignment=ft.Alignment.CENTER,
                            ),
                            ft.Column(
                                [
                                    ft.Text(title, size=17, weight=ft.FontWeight.BOLD),
                                    ft.Text(subtitle, size=13, color=BrandColors.MUTED),
                                ],
                                spacing=1,
                                expand=True,
                            ),
                        ],
                        rtl=True,
                    ),
                    ft.Container(
                        content=path,
                        bgcolor=ft.Colors.with_opacity(0.05, BrandColors.NAVY),
                        border_radius=12,
                        padding=ft.Padding.symmetric(horizontal=14, vertical=8),
                    ),
                    ft.OutlinedButton(
                        button_text,
                        icon=ft.Icons.FOLDER_OPEN_ROUNDED,
                        height=TOUCH_HEIGHT,
                        on_click=on_click,
                    ),
                ],
                spacing=8,
                horizontal_alignment=ft.CrossAxisAlignment.STRETCH,
            ),
            bgcolor=BrandColors.SURFACE,
            border=ft.Border.all(1, ft.Colors.with_opacity(0.12, BrandColors.NAVY)),
            border_radius=CARD_RADIUS,
            padding=14,
        )

    def set_paths(self, source: str | None, target: str | None) -> None:
        self.source_path.value = source or "לא נבחרה תיקייה"
        self.target_path.value = target or "לא נבחרה תיקייה"
        self.start_button.disabled = not bool(source and target)
        self.fix_names_button.disabled = not bool(source)
