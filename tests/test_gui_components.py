import flet as ft

from singlesorter_gui.state import SortSettings
from singlesorter_gui.theme import BrandColors, build_dark_theme, build_theme
from singlesorter_gui.views.home import HomeView
from singlesorter_gui.views.settings import SettingsView, settings_sheet


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


def test_settings_view_starts_with_copy_enabled():
    view = SettingsView(SortSettings())

    assert view.copy_mode.value is True
    assert view.to_settings().copy_mode is True


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
