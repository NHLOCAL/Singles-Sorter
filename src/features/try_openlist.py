import flet as ft
import json

# הגדרות משתמש שמורות
global USER_CONFIG
try:
    with open('app/config.json', 'r') as f:
        USER_CONFIG = json.load(f)
except FileNotFoundError:
    USER_CONFIG = {
        "general": {
            "copy_mode": False,
            "main_folder_only": False,
            "singles_folder": True,
            "exist_only": False,
            "abc_sort": False
        },
        "folders": {
            "source": [],
            "target": []
        }
    }


# פונקציה לשמירת נתיב שהוזן לאחרונה
def save_recent_path(path, path_type):
    if path not in USER_CONFIG['folders'][path_type]:
        USER_CONFIG['folders'][path_type].insert(0, path)
        USER_CONFIG['folders'][path_type] = USER_CONFIG['folders'][path_type][:5]
        with open('app/config.json', 'w') as file:
            json.dump(USER_CONFIG, file, indent=4)


def main(page: ft.Page):
    page.title = "דוגמה לנתיבים אחרונים"

    # Input fields - SearchBar
    source_dir_input = ft.SearchBar(
        #label="תיקית מקור",  # השתמש ב- label במקום hint_text
        on_submit=lambda e: print(f"Selected: {e.control.value}"),
    )
    target_dir_input = ft.SearchBar(
        #label="תיקית יעד",  # השתמש ב- label במקום hint_text
        on_submit=lambda e: print(f"Selected: {e.control.value}"),
    )

    # פונקציה לעדכון שדה קלט עם נתיב שנבחר
    def update_input_with_path(path, input_field):
        input_field.value = path
        input_field.update()

    # פונקציה לטיפול בבחירת תיקיה
    def update_path(e: ft.FilePickerResultEvent, target_input: ft.SearchBar, path_type):
        if e.path:
            target_input.value = e.path
            save_recent_path(e.path, path_type)

            # עדכון suggestions לאחר שמירת הנתיב החדש
            target_input.suggestions = create_search_bar_suggestions(
                path_type, target_input
            )

            target_input.update()

    # FilePickers
    source_picker = ft.FilePicker(
        on_result=lambda e: update_path(e, source_dir_input, 'source')
    )
    target_picker = ft.FilePicker(
        on_result=lambda e: update_path(e, target_dir_input, 'target')
    )
    page.overlay.extend([source_picker, target_picker])

    # פונקציה ליצירת אפשרויות עבור SearchBar
    def create_search_bar_suggestions(path_type, input_field):
        return [
            ft.Control(
                ft.ListTile(
                    leading=ft.Icon(ft.Icons.FOLDER),
                    title=ft.Text(path),
                    on_click=lambda e, p=path: update_input_with_path(p, input_field),
                )
            )
            for path in USER_CONFIG['folders'][path_type]
        ]

    # הגדרת הצעות (suggestions) ל- SearchBar
    source_dir_input.suggestions = create_search_bar_suggestions(
        'source', source_dir_input
    )
    target_dir_input.suggestions = create_search_bar_suggestions(
        'target', target_dir_input
    )

    # UI
    page.add(
        ft.Column(
            [
                source_dir_input,
                ft.ElevatedButton(
                    "בחר תיקית מקור",
                    icon=ft.Icons.FOLDER_OPEN,
                    on_click=lambda _: source_picker.get_directory_path(),
                ),
            ]
        ),
        ft.Column(
            [
                target_dir_input,
                ft.ElevatedButton(
                    "בחר תיקית יעד",
                    icon=ft.Icons.FOLDER_OPEN,
                    on_click=lambda _: target_picker.get_directory_path(),
                ),
            ]
        ),
    )


ft.app(target=main)
