import flet as ft

def main(page: ft.Page):
    def radio_changed(e):
        print(f"Selected value: {e.control.value}")
        page.update()

    page.add(
        ft.Row(
            controls=[
                ft.Radio(value="Option 1", label=ft.Text("Option 1 with a very long label that should wrap", overflow=ft.TextOverflow.ELLIPSIS),),
                ft.Radio(value="Option 2", label=ft.Text("Option 2 with a very long label that should wrap", overflow=ft.TextOverflow.ELLIPSIS),),
            ],
            wrap=True,
        ),
    )

ft.app(target=main)