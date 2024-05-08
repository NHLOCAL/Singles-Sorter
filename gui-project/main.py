import flet as ft
from singles_sorter_func import scan_dir

def main(page: ft.Page):
    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.SPACE_BETWEEN
    page.theme = ft.Theme(color_scheme=ft.ColorScheme(primary="#27447D",primary_container=ft.colors.YELLOW_50),)
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True
    page.padding = 0
    page.bgcolor = ft.colors.YELLOW_50
    #page.window_height = 680
    #page.window_width = 800
    
    
    
    LABLEL_PROGRAM = ft.Container(
        content=ft.Row(
            [
                ft.Image(src="assets/icon.png", width=60),

                ft.Text(
                    "מסדר הסינגלים 13.0",
                    size=30,
                    text_align=ft.TextAlign.CENTER,
                    color="#FCD41C",
                    weight=ft.FontWeight.BOLD,
                    rtl=True,
                ),
                
            ],

            alignment=ft.MainAxisAlignment.CENTER,
            
        ),
        bgcolor="#27447D",
        expand=False,
        height=80,
    )
    
    page.add(LABLEL_PROGRAM)
    

    # Input fields
    round_button = ft.ButtonStyle(shape=ft.RoundedRectangleBorder(radius=15))
    height_button = '50'
    round_text_field = ft.border_radius.only(15, 10, 15, 10)

    source_dir_input = ft.TextField(label="תיקית הסינגלים שלך", autofocus=True, rtl=True, expand=True, border_radius=round_text_field, height='60')

    target_dir_input = ft.TextField(label="תיקית יעד", rtl=True, expand=True,  border_radius=round_text_field, height='60')
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input))
    
    page.overlay.extend([source_picker, target_picker])

    
    source_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path(), height=height_button, tooltip='בחירת תיקיה המכילה את המוזיקה שברצונך לסדר', style=round_button,)
    target_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path(), height=height_button, tooltip='בחירת תיקית יעד אליה יוכנסו תיקיות  המוזיקה שיווצרו', style=round_button)


    # Checkboxes
    copy_mode_checkbox = ft.Checkbox(label="העתק קבצים (העברה היא ברירת המחדל)")
    tree_folders_checkbox = ft.Checkbox(label="סרוק תיקיה ראשית בלבד", )
    singles_folder_checkbox = ft.Checkbox(label='צור תיקיות סינגלים פנימיות', value=True)
    exist_only_checkbox = ft.Checkbox(label="השתמש בתיקיות קיימות בלבד")
    abc_sort_checkbox = ft.Checkbox(label="צור תיקיות ראשיות לפי ה-א' ב'")

    # Progress bar
    page.window_progress_bar='0.0'
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
            progress_num = progress / 100
            progress_bar.value = progress_num
            page.window_progress_bar = str(progress_num)
            page.update()

        # Call the scan_dir function with arguments and progress callback
        scan_dir(source_dir, target_dir, copy_mode, abc_sort, exist_only, singles_folder, tree_folders, progress_callback)
        output_text.value = "מיון הקבצים הסתיים!"
        page.window_progress_bar = '0.0'
        page.update()

    # Organize button
    organize_button = ft.ElevatedButton(content=ft.Text("הפעל כעת", size=20), on_click=organize_files, style=round_button, height='70', width='180')
    #organize_button.disabled = True

    page.add(
   
        ft.Container(
            content=ft.Column(
                [
                    ft.Row([source_dir_button, source_dir_input], alignment=ft.MainAxisAlignment.CENTER),
                    ft.Row([target_dir_button, target_dir_input], alignment=ft.MainAxisAlignment.CENTER),
                ],
    
                spacing='20',
                alignment=ft.MainAxisAlignment.CENTER,
                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
            ),
 
            margin = ft.margin.only(40, 30, 40, 30)

        ),


        ft.Container(
            content=ft.Column(
                [

                    ft.Text("אפשרויות נוספות", size=20, color="#ff27447D", weight=ft.FontWeight.BOLD),
                    
                    ft.Column(
                        [
                        
                        ft.Text("הגדרות בסיסיות"),
                        
                        copy_mode_checkbox,
                        tree_folders_checkbox,
                        
                        ft.Text("מתקדם"),
                        
                        singles_folder_checkbox,
                        exist_only_checkbox,
                        abc_sort_checkbox],
                        
                        alignment=ft.MainAxisAlignment.CENTER,
                    ),
                ],
                alignment=ft.MainAxisAlignment.CENTER,
                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
            ),

            
            margin = ft.margin.only(40, 5, 40, 10),
            border=ft.border.all(2, color="#27447D"),
            border_radius=15,
            padding=20,
            alignment=ft.alignment.center,
        ),


        ft.Container(
            content=ft.Column(
               [          
                ft.Row(
                    [
                        output_text,
                        organize_button,
                    ],

                    alignment=ft.MainAxisAlignment.CENTER,
                ),

                ft.Row(
                    [
                        progress_bar,
                    ],

                    alignment=ft.MainAxisAlignment.CENTER,
                ),

                ],
                

                spacing='30',
            ),

            margin = ft.margin.all(20),
            padding=10,
            alignment=ft.alignment.center,
        )
    )
    

ft.app(target=main)
