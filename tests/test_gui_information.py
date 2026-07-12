import flet as ft

from singlesorter import __VERSION__
from singlesorter_gui.content import load_content
from singlesorter_gui.views.home import HomeView
from singlesorter_gui.views.information import information_sheet


def test_packaged_information_content_is_available():
    assert "מסדר הסינגלים" in load_content("about")
    assert "בחירת תיקיות" in load_content("help")
    assert "14.0.1" in load_content("whats-new")


def test_information_sheet_is_fullscreen_scrollable_and_closable():
    sheet = information_sheet(
        title="עזרה",
        markdown="# תוכן",
        icon=ft.Icons.HELP_OUTLINE,
        on_close=lambda _: None,
    )

    layout = sheet.content.content
    scrolling_content = layout.controls[2]
    assert sheet.fullscreen is True
    assert sheet.dismissible is True
    assert layout.expand is True
    assert layout.scroll is None
    assert isinstance(scrolling_content, ft.Column)
    assert scrolling_content.scroll == ft.ScrollMode.AUTO
    assert isinstance(scrolling_content.controls[0], ft.Markdown)
    assert isinstance(layout.controls[-1], ft.FilledButton)


def test_home_menu_restores_original_information_and_management_entries():
    view = HomeView(on_pick_source=lambda _: None, on_pick_target=lambda _: None)

    entries = {item.data: item.content for item in view.more_menu.items}

    assert entries["help"] == "עזרה"
    assert entries["whats-new"] == "מה חדש"
    assert entries["about"] == "אודות התוכנה"
    assert entries["singers"] == "רשימת זמרים אישית"
    assert entries["update"] == "בדיקת עדכונים"
    assert view.version_text.value == f"גרסה {__VERSION__}"
