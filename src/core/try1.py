import flet as ft

def main(page: ft.Page):
    def regular_action(e):
        # קוד להפעלה רגילה
        pass

    def clean_only_action(e):
        # קוד להפעלת ניקוי בלבד
        pass

    tabs = ft.Tabs(
        selected_index=0,
        tabs=[
            ft.Tab(text="הפעל", content=ft.Text("הפעל",)),
            ft.Tab(text="הפעל ניקוי בלבד", content=ft.Text("הפעל ניקוי בלבד",))
        ]
    )

    page.add(tabs)

ft.app(target=main)
