"""Central visual tokens for the Flet application."""

from __future__ import annotations

from dataclasses import dataclass

import flet as ft


@dataclass(frozen=True, slots=True)
class BrandColors:
    NAVY = "#17345F"
    NAVY_DARK = "#0C1D36"
    STEEL = "#446B9E"
    GOLD = "#F5C518"
    GOLD_LIGHT = "#FFD95A"
    CANVAS = "#F6F8FC"
    SURFACE = "#FFFFFF"
    SURFACE_DARK = "#13243E"
    TEXT = "#172033"
    MUTED = "#657289"


def build_theme() -> ft.Theme:
    return ft.Theme(
        color_scheme=ft.ColorScheme(
            primary=BrandColors.NAVY,
            secondary=BrandColors.GOLD,
            surface=BrandColors.SURFACE,
            on_surface=BrandColors.TEXT,
        ),
        font_family="Segoe UI",
        visual_density=ft.VisualDensity.COMFORTABLE,
    )


def build_dark_theme() -> ft.Theme:
    return ft.Theme(
        color_scheme=ft.ColorScheme(
            primary=BrandColors.STEEL,
            secondary=BrandColors.GOLD_LIGHT,
            surface=BrandColors.SURFACE_DARK,
            on_surface="#F5F7FB",
        ),
        font_family="Segoe UI",
        visual_density=ft.VisualDensity.COMFORTABLE,
    )


CARD_RADIUS = 20
TOUCH_HEIGHT = 48
