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
    
    
    
    # strings
    page.client_storage.set("key", "value")

    # numbers, booleans
    page.client_storage.set("number.setting", 12345)
    page.client_storage.set("bool_setting", True)

    # lists
    page.client_storage.set("favorite_colors", ["red", "green", "blue"])


    # The value is automatically converted back to the original type
    value = page.client_storage.get("key")

    colors = page.client_storage.get("favorite_colors")
    # colors = ["red", "green", "blue"]
    print(colors)

    
    page.add(tabs)

ft.app(target=main)
