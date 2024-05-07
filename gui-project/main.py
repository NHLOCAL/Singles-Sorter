import flet as ft
from singles_sorter_func import scan_dir

def main(page: ft.Page):
    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.CENTER
    page.window_height = 600
    page.window_width = 800
    
    
    
    LABLEL_PROGRAM = ft.Container(
        content=ft.Row(
            [
                ft.Text(
                    "מסדר הסינגלים 13.0",
                    size=40,
                    text_align=ft.TextAlign.CENTER,
                    color="#FCD41C",
                    weight=ft.FontWeight.BOLD,
                    rtl=True,
                ),
                ft.Image(src="assets/icon.png", width=80, height=80),  # הוספת התמונה
            ],
            alignment=ft.MainAxisAlignment.CENTER,
        ),
        bgcolor="#27447D",
        expand=True,
    )
    page.add(LABLEL_PROGRAM)
    

    # Input fields
    source_dir_input = ft.TextField(label="תיקית הסינגלים שלך", autofocus=True, rtl=True, expand=True)
    source_dir_button = ft.ElevatedButton("בחר תיקיה", on_click=lambda _: pick_directory(source_dir_input))

    target_dir_input = ft.TextField(label="תיקית יעד", rtl=True, expand=True)
    target_dir_button = ft.ElevatedButton("בחר תיקיה", on_click=lambda _: pick_directory(target_dir_input))
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input))
    
    page.overlay.extend([source_picker, target_picker])  # הוספת ה-FilePickers ל-overlay
    
    source_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path())
    target_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path())


    # Checkboxes
    copy_mode_checkbox = ft.Checkbox(label="העתק קבצים (העברה היא ברירת המחדל)")
    tree_folders_checkbox = ft.Checkbox(label="אל תסרוק עץ תיקיות", )
    singles_folder_checkbox = ft.Checkbox(label='צור תיקית "סינגלים"')
    exist_only_checkbox = ft.Checkbox(label="השתמש בתיקיות קיימות בלבד")
    abc_sort_checkbox = ft.Checkbox(label="מיין בתיקיות לפי הא' ב'")

    # Progress bar
    progress_bar = ft.ProgressBar(width=400, value=0)

    # Output text
    output_text = ft.Text()
    
    
    def update_path(e: ft.FilePickerResultEvent, target_input: ft.TextField):
        target_input.value = e.path if e.path else None
        target_input.update()
        

    def organize_files(e):
        source_dir = source_dir_input.value
        target_dir = target_dir_input.value

        # Get checkbox values
        copy_mode = copy_mode_checkbox.value
        tree_folders = tree_folders_checkbox.value
        singles_folder = singles_folder_checkbox.value
        exist_only = exist_only_checkbox.value
        abc_sort = abc_sort_checkbox.value

        def progress_callback(progress):
            progress_bar.value = progress / 100
            page.update()

        # Call the scan_dir function with arguments and progress callback
        scan_dir(source_dir, target_dir, copy_mode, abc_sort, exist_only, singles_folder, tree_folders, progress_callback)
        output_text.value = "מיון הקבצים הסתיים!"
        page.update()

    # Organize button
    organize_button = ft.ElevatedButton("הפעל כעת", on_click=organize_files)

    page.add(

        ft.Column(
            [
                ft.Row([source_dir_button, source_dir_input], alignment=ft.MainAxisAlignment.CENTER),
                ft.Row([target_dir_button, target_dir_input], alignment=ft.MainAxisAlignment.CENTER),
                
                ft.Column(
                    [copy_mode_checkbox,
                    tree_folders_checkbox,
                    singles_folder_checkbox,
                    exist_only_checkbox,
                    abc_sort_checkbox],
                    
                    alignment=ft.MainAxisAlignment.CENTER,
                    rtl=True,
                ),
                    progress_bar,
                    output_text,
                    organize_button,
            ],
            alignment=ft.MainAxisAlignment.CENTER,
            horizontal_alignment=ft.CrossAxisAlignment.CENTER,
        )
    )
    

ft.app(target=main)