import flet as ft

def main(page: ft.Page):
    text = ft.Text("Hello", color="yellow")


    def change_color(e):
        text.color = "red"
        text.value = "World!"
        text.update()  # מעדכן רק את ה-Text

    button = ft.ElevatedButton("Change Text", on_click=change_color)

    page.add(text, button)

ft.app(target=main)