import flet as ft
import flet_material as fm

fm.Theme.set_theme(theme="teal")

def main(page: ft.Page):
   page.bgcolor = fm.Theme.bgcolor

   page.horizontal_alignment = "center"
   page.vertical_alignment = "center"

   button = fm.Buttons(
       width=220,
       height=55,
       title="Give this repo a star!",
   )

   page.add(button)

   page.update()

ft.app(target=main)