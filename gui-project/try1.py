import flet as ft
import flet_material as fm


fm.Theme.set_theme(theme="blue")


def main(page: ft.Page):
    page.bgcolor = fm.Theme.bgcolor

    page.horizontal_alignment = "center"
    page.vertical_alignment = "center"

    drop = fm.Admonitions(
        type_="note", expanded_height=300, expanded=False, controls_list=None
    )

    page.add(drop)

    page.update()


if __name__ == "__main__":
    ft.app(target=main)
