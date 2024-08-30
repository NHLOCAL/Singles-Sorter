import flet as ft
from plyer import notification

def main(page: ft.Page):
    def on_click(e):
        # דוגמה עם ticker
        notification.notify(
            title="פעולה הושלמה",
            message="הפעולה הסתיימה בהצלחה!",
            app_name="מסדר הסינגלים",
            timeout=5,
            app_icon="assets/icon.ico",
            ticker="הודעה חדשה!",
        )

    page.add(
        ft.ElevatedButton("הפעלת פעולה", on_click=on_click)
    )

ft.app(target=main)