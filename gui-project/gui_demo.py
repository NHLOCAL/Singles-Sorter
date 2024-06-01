import flet as ft
from singles_sorter_gui import scan_dir  # Assuming this is your external function


def main(page: ft.Page):
    def show_snackbar_with_action(message_text, color):
        def dismiss_snackbar(e):
            page.snack_bar.open = False
            page.update()

        page.snack_bar = ft.SnackBar(
            content=ft.Text(message_text),
            bgcolor=color,
            action=ft.TextButton("סגור", on_click=dismiss_snackbar),
        )
        
        page.snack_bar.open = True
        page.update()

    def show_alert_dialog(message_text):
        def close_dialog(e):
            page.dialog.open = False
            page.update()

        page.dialog = ft.AlertDialog(
            title=ft.Text("הודעה"),
            content=ft.Text(message_text),
            actions=[
                ft.TextButton("אישור", on_click=close_dialog),
            ],
        )
        page.dialog.open = True
        page.update()

    

    #show_snackbar_with_action('ddd', 'blue')
    show_alert_dialog('cccccccccccccc fdxbf')

ft.app(target=main)