# -*- coding: utf-8 -*-
import flet as ft
from singles_sorter_v3 import MusicSorter
import json

# גרסת התוכנה
global VERSION
VERSION = MusicSorter.VERSION

# הגדרות משתמש שמורות
global USER_CONFIG
with open('app/config.json', 'r') as f:
    USER_CONFIG = json.load(f)

# שמירת הגדרות משתמש לקובץ
def save_recent_folders(e=None):
    user_config = USER_CONFIG
    with open('app/config.json', 'w') as file:
        json.dump(user_config, file, indent=4)

def update_path(e: ft.FilePickerResultEvent, target_input: ft.TextField, folder_type: str):
    try:
        target_input.value = e.path if e.path else None
        target_input.update()

        # עדכון רשימת תיקיות אחרונות שנבחרו
        recent_folders = USER_CONFIG['folders'].get(folder_type, [])
        if e.path and e.path not in recent_folders:
            recent_folders.insert(0, e.path)
            if len(recent_folders) > 5:  # הגבלת הרשימה ל-5 תיקיות אחרונות
                recent_folders = recent_folders[:5]
            USER_CONFIG['folders'][folder_type] = recent_folders
            save_recent_folders()
    except Exception as error:
        print(f"שגיאה בעדכון הנתיב: {error}")

def main(page: ft.Page):
    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.SPACE_BETWEEN
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True

    if page.platform == ft.PagePlatform.ANDROID:
        page.padding = ft.padding.all(20)
        page.scroll = ft.ScrollMode.AUTO
        auto_focus=False
    else:
        page.padding = ft.padding.only(45, 30, 45, 30)
        page.window_height = 760
        page.window_width = 850
        auto_focus=True

    round_button = ft.ButtonStyle(shape=ft.RoundedRectangleBorder(radius=15))

    height_button = '50'
    width_button = '150'
    round_text_field = ft.border_radius.only(15, 10, 15, 10)

    source_dir_input = ft.TextField(label="תיקית הסינגלים שלך", autofocus=auto_focus, rtl=True, expand=True, border_radius=round_text_field, height='50', hint_text=r"C:\Music\סינגלים")

    target_dir_input = ft.TextField(label="תיקית יעד", rtl=True, expand=True,  border_radius=round_text_field, height='50', hint_text=r"C:\Music\המוזיקה שלך",)
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input, "source"))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input, "target"))
    
    page.overlay.extend([source_picker, target_picker])
    
    source_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקיה המכילה את המוזיקה שברצונך לסדר', style=round_button,)
    target_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקית יעד אליה יוכנסו תיקיות המוזיקה שיווצרו', style=round_button)

    recent_source_folders = USER_CONFIG['folders'].get('source', [])
    recent_target_folders = USER_CONFIG['folders'].get('target', [])
    
    def on_recent_folder_selected(e, target_input: ft.TextField, current_dropdown):
        target_input.value = current_dropdown
        target_input.update()


    recent_source_dropdown = ft.Dropdown(
        hint_text="תיקיות אחרונות",
        options=[ft.dropdown.Option(text=folder, data=folder) for folder in recent_source_folders],
        on_change=lambda e: on_recent_folder_selected(e, source_dir_input, recent_source_dropdown.value)
    )

    recent_target_dropdown = ft.Dropdown(
        hint_text="תיקיות אחרונות",
        width=50,
        icon=ft.icons.DOWNLOAD,
        options=[ft.dropdown.Option(text=folder, data=folder) for folder in recent_target_folders],
        on_change=lambda e: on_recent_folder_selected(e, target_dir_input, recent_target_dropdown.value)
    )

    page.add(
        ft.Container(
            content=ft.Column(
                [
                    ft.Row([source_dir_button, source_dir_input, recent_source_dropdown], alignment=ft.MainAxisAlignment.CENTER),
                    ft.Row([target_dir_button, target_dir_input, recent_target_dropdown], alignment=ft.MainAxisAlignment.CENTER),
                ],
                spacing='20',
                alignment=ft.MainAxisAlignment.CENTER,
                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
            ),
            margin=ft.margin.only(0, 0, 0, 20)
        )
    )

ft.app(target=main)
