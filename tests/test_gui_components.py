import flet as ft

from singlesorter_gui.services.singer_list import SingerEntry
from singlesorter_gui.state import SortSettings
from singlesorter_gui.theme import BrandColors, build_dark_theme, build_theme
from singlesorter_gui.views.home import HomeView
from singlesorter_gui.views.settings import SettingsView, settings_sheet
from singlesorter_gui.views.singer_list import SingerListEditor, singer_list_sheet


def test_theme_uses_icon_inspired_blue_and_gold_palette():
    theme = build_theme()

    assert theme.color_scheme.primary == BrandColors.NAVY
    assert theme.color_scheme.secondary == BrandColors.GOLD
    assert build_dark_theme().color_scheme.secondary == BrandColors.GOLD_LIGHT


def test_home_view_has_one_primary_action_disabled_without_paths():
    view = HomeView(on_pick_source=lambda _: None, on_pick_target=lambda _: None)

    assert isinstance(view.start_button, ft.FilledButton)
    assert view.start_button.content == "סדר את המוזיקה"
    assert view.start_button.disabled is True
    assert view.control.col["xs"] == 12
    assert view.control.col["lg"] < 12


def test_home_view_enables_action_when_both_paths_are_selected():
    view = HomeView(on_pick_source=lambda _: None, on_pick_target=lambda _: None)

    view.set_paths("C:/music", "C:/sorted")

    assert view.start_button.disabled is False
    assert view.source_path.value == "C:/music"


def test_fix_names_action_requires_only_a_source_folder():
    view = HomeView(on_pick_source=lambda _: None, on_pick_target=lambda _: None)

    assert view.fix_names_button.disabled is True
    view.set_paths("C:/music", None)

    assert view.fix_names_button.content == "תיקון שמות ותגיות"
    assert view.fix_names_button.disabled is False
    assert view.start_button.disabled is True


def test_settings_view_starts_with_move_and_no_singles_folder():
    view = SettingsView(SortSettings())

    assert view.copy_mode.value is False
    assert view.singles_folder.value is False
    assert view.to_settings().copy_mode is False
    assert view.to_settings().singles_folder is False


def test_settings_sheet_keeps_actions_visible_outside_scroll_area():
    view = SettingsView(SortSettings())

    sheet = settings_sheet(
        view,
        on_cancel=lambda _: None,
        on_save=lambda _: None,
    )

    layout = sheet.content.content
    scrolling_settings, divider, actions = layout.controls

    assert sheet.scrollable is False
    assert sheet.fullscreen is True
    assert sheet.dismissible is True
    assert sheet.content.expand is True
    assert sheet.content.height is None
    assert scrolling_settings is view.control
    assert scrolling_settings.expand is True
    assert view.control.scroll == ft.ScrollMode.AUTO
    assert isinstance(divider, ft.Divider)
    assert isinstance(actions, ft.Row)
    assert [button.content for button in actions.controls] == ["ביטול", "שמירה"]


def test_personal_singer_editor_is_fullscreen_and_editable():
    editor = SingerListEditor([SingerEntry("שם בקובץ", "שם תיקייה")])
    sheet = singer_list_sheet(
        editor,
        on_close=lambda _: None,
        on_save=lambda _: None,
        on_import=lambda _: None,
        on_export=lambda _: None,
    )

    assert sheet.fullscreen is True
    assert len(editor.rows) == 1
    editor.add_entry()
    assert len(editor.rows) == 2
    editor.rows[1].source.value = "זמר חדש"
    editor.rows[1].target.value = "תיקייה חדשה"
    assert editor.to_entries()[-1] == SingerEntry("זמר חדש", "תיקייה חדשה")
