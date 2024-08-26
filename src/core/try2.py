import flet as ft
from add_singer_dialog import create_add_singer_dialog

def main(page: ft.Page):
    page.title = "טבלה ניתנת לעריכה"
    page.rtl = True

    # יצירת הדיאלוג באמצעות הפונקציה מיובאת
    dialog = create_add_singer_dialog(page)

    # פונקציה לפתיחת הדיאלוג
    def open_dlg_modal(e):
        page.overlay.append(dialog)
        dialog.open = True
        page.update()

    page.add(
        ft.ElevatedButton("פתח טבלה", on_click=open_dlg_modal),
    )

ft.app(target=main)