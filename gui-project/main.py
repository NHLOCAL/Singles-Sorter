# -*- coding: utf-8 -*-
import flet as ft
from singles_sorter_v2 import scan_dir

def main(page: ft.Page):
    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.SPACE_BETWEEN
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True
    page.padding = ft.padding.only(45, 30, 45, 30)
    #page.bgcolor = ft.colors.SURFACE_VARIANT
    page.window_height = 820
    page.window_width = 900

    # Consistent button style definition
    round_button = ft.ButtonStyle(shape=ft.RoundedRectangleBorder(radius=15))
    
    
    # App bar
    page.appbar = ft.AppBar(
        title=ft.Row(
            [
                ft.Image(src="assets/icon.png", width=40),  # Adjusted icon size
                ft.Text(
                    "מסדר הסינגלים 13.0",
                    size=24,  # Adjusted title size
                    text_align=ft.TextAlign.CENTER,
                    color=ft.colors.ON_PRIMARY,  # Assumed color for better contrast
                    weight=ft.FontWeight.BOLD,
                ),
            ],
            alignment=ft.MainAxisAlignment.CENTER,
        ),
        center_title=True,
        bgcolor=ft.colors.PRIMARY,
        elevation=4,  # Added elevation for visual depth
        toolbar_height='60',
    )


    # הגדרת סרגל תחתון
    page.bottom_appbar = ft.BottomAppBar(
        ft.Text(
                            "© כל הזכויות שמורות ל-nh.local11@gmail.com",
                            size=10,
                            text_align=ft.TextAlign.CENTER,
                            color=ft.colors.ON_SECONDARY,
                        ),
        bgcolor=ft.colors.ON_PRIMARY_CONTAINER,
        shape=ft.NotchShape.CIRCULAR,
        height='40',
    )


    # Input fields

    height_button = '55'
    round_text_field = ft.border_radius.only(15, 10, 15, 10)

    source_dir_input = ft.TextField(label="תיקית הסינגלים שלך", autofocus=True, rtl=True, expand=True, border_radius=round_text_field, height='60', hint_text=r"C:\Music\סינגלים",)

    target_dir_input = ft.TextField(label="תיקית יעד", rtl=True, expand=True,  border_radius=round_text_field, height='60', hint_text=r"C:\Music\כל המוזיקה",)
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input))
    
    page.overlay.extend([source_picker, target_picker])

    
    source_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path(), height=height_button, tooltip='בחירת תיקיה המכילה את המוזיקה שברצונך לסדר', style=round_button,)
    target_dir_button = ft.ElevatedButton("בחר תיקיה", icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path(), height=height_button, tooltip='בחירת תיקית יעד אליה יוכנסו תיקיות  המוזיקה שיווצרו', style=round_button)


    # Checkboxes
    copy_mode_checkbox = ft.Checkbox(label="העתק קבצים (העברה היא ברירת המחדל)")
    main_folder_only_checkbox = ft.Checkbox(label="סרוק תיקיה ראשית בלבד", )
    singles_folder_checkbox = ft.Checkbox(label='צור תיקיות סינגלים פנימיות', value=True)
    exist_only_checkbox = ft.Checkbox(label="השתמש בתיקיות קיימות בלבד")
    abc_sort_checkbox = ft.Checkbox(label="צור תיקיות ראשיות לפי ה-א' ב'")

    # Progress bar
    page.window_progress_bar='0.0'
    progress_bar = ft.ProgressBar(width=400, value=0)

    # Output text
    output_text = ft.Text()
    


    # Organize button
    organize_button = ft.ElevatedButton(
        content=ft.Text("הפעל כעת", size=20),
        on_click=lambda e: organize_files(e, page),
        style=round_button,
        height='70',
        width='180',
        )

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
 
            margin = ft.margin.only(0, 0, 0, 20)

        ),


        ft.Container(
            content=ft.Column(
                [

                    ft.Text("אפשרויות נוספות", size=20, color=ft.colors.PRIMARY, weight=ft.FontWeight.BOLD),
                    
                    ft.Column(
                        [
                            ft.Text("הגדרות בסיסיות", weight=ft.FontWeight.BOLD),
                            copy_mode_checkbox,
                            main_folder_only_checkbox,
                        ]
                    ),

                    ft.Column(
                        [
                            ft.Text("מתקדם", weight=ft.FontWeight.BOLD), 
                            singles_folder_checkbox,
                            exist_only_checkbox,
                            abc_sort_checkbox,
                        ]
                    ),
                ],
                alignment=ft.MainAxisAlignment.CENTER,
                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
            ),

            
            margin = ft.margin.all(0),
            border=ft.border.all(2, color="#27447D"),
            border_radius=15,
            padding=10,
            alignment=ft.alignment.center,
        ),


        ft.Container(
            content=ft.Column(
               [

                ft.Row(
                    [
                        progress_bar,
                    ],

                    alignment=ft.MainAxisAlignment.CENTER,
                ),
                  
                ft.Row(
                    [
                        output_text,
                        organize_button,
                    ],

                    alignment=ft.MainAxisAlignment.CENTER,
                ),

                ],
                
                spacing='30',
            ),

            margin = ft.margin.all(10),
            padding=10,
            alignment=ft.alignment.center,
        )
    )




    def update_path(e: ft.FilePickerResultEvent, target_input: ft.TextField):
        try:
            target_input.value = e.path if e.path else None
            target_input.update()
        except Exception as error:
            # Display an error message to the user or log the error
            print(f"Error updating path: {error}")
            # Consider using a Snackbar or AlertDialog to display the error to the user
        

    def organize_files(e, page: ft.Page):
        source_dir = source_dir_input.value
        target_dir = target_dir_input.value

        show_snackbar = lambda message_text, color, mseconds=1000, : ft.SnackBar(content=ft.Text(message_text), bgcolor=color, duration=mseconds)
        
        if not source_dir or not target_dir:
            page.snack_bar = show_snackbar("אנא בחר תיקיית מקור ותיקיית יעד!", ft.colors.ERROR)
            page.snack_bar.open = True
            page.update()
            return

        # Get checkbox values
        copy_mode = copy_mode_checkbox.value
        main_folder_only = main_folder_only_checkbox.value
        singles_folder = singles_folder_checkbox.value
        exist_only = exist_only_checkbox.value
        abc_sort = abc_sort_checkbox.value

        def progress_callback(progress):
            progress_num = progress / 100
            progress_bar.value = progress_num
            page.window_progress_bar = str(progress_num)
            page.update()

        # Call the scan_dir function with arguments and progress callback
        try:
            # השבתת כפתור הפעל בעת הרצת הסריקה
            organize_button.disabled = True
            page.update()
            
            scan_dir(
                source_dir, 
                target_dir, 
                copy_mode, 
                abc_sort, 
                exist_only, 
                singles_folder, 
                main_folder_only, 
                progress_callback
            )
            page.snack_bar = show_snackbar("מיון הקבצים הסתיים בהצלחה", ft.colors.GREEN, 10000)

        except FileNotFoundError as error:
            page.snack_bar = show_snackbar(f"{error}", ft.colors.ERROR)
        except PermissionError as error:
            page.snack_bar =  show_snackbar(f"{error}", ft.colors.ERROR)
        except Exception as error:
            page.snack_bar = show_snackbar(f"שגיאה במיון הקבצים: {error}", ft.colors.ERROR)

        finally: 
            page.window_progress_bar = '0.0'
            page.snack_bar.open = True
            organize_button.disabled = False
            page.update()



ft.app(target=main)
