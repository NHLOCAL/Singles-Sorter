import flet as ft

def main(page: ft.Page):
    page.title = "Icon Explorer"

    icons = [
        ft.icons.INFO,
        ft.icons.QUESTION_MARK,
        ft.icons.LIGHTBULB,
        ft.icons.TOUCH_APP,
        ft.icons.GESTURE,
        ft.icons.MORE_HORIZ,
        ft.icons.MORE_VERT,
    ]

    icon_grid = ft.GridView(
        runs_count=3,
        spacing=10,
        run_spacing=10,
        padding=20,
        child_aspect_ratio=1.0,
    )


    for icon_name in icons:
        icon_grid.controls.append(
            ft.Container(
                content=ft.Column(
                    [
                        ft.Icon(icon_name, size=40),
                        ft.Text(str(icon_name)),  # Correct: just use str(icon_name)
                    ],
                    alignment=ft.MainAxisAlignment.CENTER,
                    horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                ),
            )
        )

    page.add(icon_grid)


ft.app(target=main)