import flet as ft
from singles_sorter_gui import scan_dir  # Assuming this is your external function


def main(page: ft.Page):
    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.CENTER
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True
    page.padding = 20  # Added padding for better spacing

    # Consistent button style definition
    round_button_style = ft.ButtonStyle(shape=ft.RoundedRectangleBorder(radius=15))



    # Input fields with placeholder text
    source_dir_input = ft.TextField(
        label="תיקית הסינגלים שלך",
        hint_text="C:/Music/Singles",  # Example placeholder
        expand=True,
        border_radius=10,
    )
    target_dir_input = ft.TextField(
        label="תיקית יעד",
        hint_text="C:/Organized Music",  # Example placeholder
        expand=True,
        border_radius=10,
    )

    # File pickers
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input))
    page.overlay.extend([source_picker, target_picker])

    # Buttons with icons
    source_dir_button = ft.ElevatedButton(
        "בחר תיקיה",
        icon=ft.icons.FOLDER_OPEN,
        on_click=lambda _: source_picker.get_directory_path(),
        style=round_button_style,
    )
    target_dir_button = ft.ElevatedButton(
        "בחר תיקיה",
        icon=ft.icons.FOLDER_OPEN,
        on_click=lambda _: target_picker.get_directory_path(),
        style=round_button_style,
    )

    # Checkboxes with grouping
    copy_mode_checkbox = ft.Checkbox(label="העתק קבצים (העברה היא ברירת המחדל)")
    tree_folders_checkbox = ft.Checkbox(label="סרוק תיקיה ראשית בלבד")

    singles_folder_checkbox = ft.Checkbox(label="צור תיקיות סינגלים פנימיות")
    exist_only_checkbox = ft.Checkbox(label="השתמש בתיקיות קיימות בלבד")
    abc_sort_checkbox = ft.Checkbox(label="צור תיקיות ראשיות לפי ה-א' ב'")

    # Progress bar
    progress_bar = ft.ProgressBar(width=page.width * 0.8, value=0)  # Responsive width
    output_text = ft.Text()

    # Organize button with styling
    organize_button = ft.ElevatedButton(
        "הפעל כעת",
        on_click=lambda e: organize_files(e, page),  # Pass page for updates
        style=round_button_style,
        bgcolor=ft.colors.SECONDARY,  # Example color for emphasis
        width=150,
        height=40,
    )

    # Improved layout with spacing and visual hierarchy
    page.add(
        ft.Row(
            [
                ft.Column(
                    [
                        ft.Text("הגדרות בסיסיות", weight=ft.FontWeight.BOLD),
                        copy_mode_checkbox,
                        tree_folders_checkbox,
                    ],
                    spacing=10,  # Added spacing between checkboxes
                ),
                ft.Column(
                    [
                        ft.Text("מתקדם", weight=ft.FontWeight.BOLD),
                        singles_folder_checkbox,
                        exist_only_checkbox,
                        abc_sort_checkbox,
                    ],
                    spacing=10,
                ),
            ],
            alignment=ft.MainAxisAlignment.SPACE_EVENLY,
        ),
        ft.Row([source_dir_button, source_dir_input], alignment=ft.MainAxisAlignment.CENTER),
        ft.Row([target_dir_button, target_dir_input], alignment=ft.MainAxisAlignment.CENTER),
        ft.Row(
            [progress_bar, output_text],  # Progress bar next to output text
            alignment=ft.MainAxisAlignment.CENTER,
        ),
        ft.Row([organize_button], alignment=ft.MainAxisAlignment.CENTER),
    )

    # Function to update input field values
    def update_path(e: ft.FilePickerResultEvent, target_input: ft.TextField):
        target_input.value = e.path if e.path else None
        target_input.update()

    # Function to handle file organization (with error handling example)
    def organize_files(e, page: ft.Page):
        source_dir = source_dir_input.value
        target_dir = target_dir_input.value

        # ... (Get checkbox values as before)

        try:
            scan_dir(  # Assuming your external function
                source_dir,
                target_dir,
                copy_mode,
                abc_sort,
                exist_only,
                singles_folder,
                tree_folders,
                lambda progress: update_progress(progress, page),  # Update progress bar
            )
            output_text.value = "מיון הקבצים הסתיים!"
        except Exception as ex:
            output_text.value = f"שגיאה: {ex}"  # Display error message
        finally:
            page.update()

    # Function to update progress bar and page
    def update_progress(progress, page: ft.Page):
        progress_num = progress / 100
        progress_bar.value = progress_num
        page.update()


ft.app(target=main)