import flet as ft

def main(page: ft.Page):


    # after
    page.navigation_bar = ft.NavigationBar(
        destinations=[
            ft.NavigationBarDestination(
                  icon=ft.Icon(
                        ft.Icons.PHONE,
                        badge="10",
                    ),
                label="Calls",
            ),
        ]
    )

    page.update()

ft.app(target=main)