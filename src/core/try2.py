import flet as ft

def main(page: ft.Page):

    menu_items = [
        ft.PopupMenuItem(text="פריט 1"),
        ft.PopupMenuItem(text="פריט 2"),
    ]

    menu_button = ft.Badge(
        content=ft.PopupMenuButton(
            items=menu_items,  # משתמשים ברשימה המקורית
            icon=ft.icons.MORE_VERT,
            icon_color=ft.colors.ON_PRIMARY,
            icon_size=28,
            tooltip="אפשרויות נוספות",
        ),
        text='up',
        label_visible=False,
        offset=ft.transform.Offset(0, -2),
    )

    def add_item(e):
        menu_items.append(ft.PopupMenuItem(text="פריט חדש"))  # מוסיפים פריט לרשימה המקורית
        menu_button.content.items = menu_items  # מעדכנים את ה-items של התפריט
        menu_button.update() # מעדכנים את ה-Badge

    page.add(menu_button, ft.ElevatedButton("הוסף פריט", on_click=add_item))

ft.app(target=main)