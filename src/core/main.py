# -*- coding: utf-8 -*-
import flet as ft
import csv
from shutil import copy

# קבצי התוכנה
from singles_sorter_v5 import MusicSorter, __VERSION__
from update_config import check_for_update
from add_singer_dialog import create_add_singer_dialog


# גרסת התוכנה
VERSION = __VERSION__


def main(page: ft.Page):

    # הגדרת זיהוי הפעלה על אנדרואיד
    ANDROID_MODE = page.platform != ft.PagePlatform.ANDROID

    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.SPACE_BETWEEN
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True
    page.theme = ft.Theme(color_scheme_seed="#2196f3")
    ph = ft.PermissionHandler()
    page.overlay.append(ph)

    # הגדרות דינאמיות בהתאם לפלטפורמה
    if ANDROID_MODE:
        page.padding = ft.padding.only(20, 10, 20, 0)
        page.scroll = ft.ScrollMode.HIDDEN  # הסתרת גלילה באנדרואיד
        scroll_mode = ft.ScrollMode.HIDDEN
        auto_focus = False
        width_button = '100%'  # כפתורים ברוחב מלא באנדרואיד
        describe_button = 'בחר'
        organize_button_title = "מיין"
        fix_button_title = "תקן"
        width_fix_button = None
        width_organize_button = None

    else:
        page.padding = ft.padding.all(20)
        page.window.height = 700
        page.window.width = 1050
        page.scroll = ft.ScrollMode.ADAPTIVE
        scroll_mode = ft.ScrollMode.AUTO
        auto_focus = True
        width_button = '150'
        describe_button = 'בחר תיקיה'
        organize_button_title = "מיין שירים"
        fix_button_title = "תקן שמות"
        width_fix_button = 170
        width_organize_button = 180

    # פונקצייה להצגת הודעה קופצת בתחתית המסך עם פרמטרים שונים
    show_snackbar = lambda message_text, color, mseconds=3000, : ft.SnackBar(content=ft.Text(message_text), bgcolor=color, duration=mseconds)


    # פונקציה לפתיחת הודעת מה חדש בהפעלה הראשונה של התוכנה
    def first_run_menu():

        first_run_status = page.client_storage.get("singlesorter.first_run") or '0.0'

        if first_run_status < VERSION:
            show_content('whats-new', 'מה חדש', ft.icons.NEW_RELEASES)
            page.client_storage.set("singlesorter.first_run", VERSION)  # סימון שההודעה הוצגה
            page.update()

    
    
    # App bar
    page.appbar = ft.AppBar(
        title=ft.Row(
            [
                ft.Image(src="assets/icon.png", width=40),  # Adjusted icon size
                ft.Text(
                    f"מסדר הסינגלים {VERSION}",
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
        toolbar_height=60,
    )


    # הגדרת סרגל תחתון
    page.bottom_appbar = ft.BottomAppBar(
        ft.Text(
            "להורדת סינגלים חינם - ",
            size=12,
            text_align=ft.TextAlign.CENTER,
            color=ft.colors.ON_SECONDARY,
            spans=[
                ft.TextSpan(
                    "שיר בוט",
                    ft.TextStyle(
                        decoration=ft.TextDecoration.UNDERLINE,
                        color=ft.colors.BLUE,
                        weight="BOLD",
                    ),
                    url="https://nhlocal.github.io/shir-bot?utm_source=singles_sorter_program&utm_medium=desktop"
                ),
            ],
        ),


        bgcolor=ft.colors.ON_PRIMARY_CONTAINER,
        shape=ft.NotchShape.CIRCULAR,
        padding=8,
        height='35',
    )


    # תפריט אפשרויות נוספות
    # Define menu items
    def on_menu_selected(e):
        if e.control.data == "upadte":
            show_update()
        elif e.control.data == "help":
            show_content('help', 'עזרה', ft.icons.HELP)        
        elif e.control.data == "about":
            show_content('about', 'אודות התוכנה', ft.icons.INFO)
        elif e.control.data == "whats_new":
            show_content('whats-new', 'מה חדש', ft.icons.NEW_RELEASES)
        elif e.control.data == "settings":
            show_settings()

        

    # תפריט אפשרויות נוספות
    menu_items = [
        ft.PopupMenuItem(text="עזרה", icon=ft.icons.HELP, data="help", on_click=on_menu_selected),
        ft.PopupMenuItem(text="אודות התוכנה", icon=ft.icons.INFO, data="about", on_click=on_menu_selected),
        ft.PopupMenuItem(text="מה חדש", icon=ft.icons.NEW_RELEASES, data="whats_new", on_click=on_menu_selected),
        ft.PopupMenuItem(text="הגדרות מתקדמות", icon=ft.icons.SETTINGS, data="settings", on_click=on_menu_selected),
    ]



    # כפתור אפשרויות נוספות בסרגל העליון
    # כולל הצגת התראה אדומה אם קיים עדכון זמין
    menu_button = ft.Badge(
        content=ft.PopupMenuButton(
            items=menu_items,
            icon=ft.icons.MORE_VERT,
            icon_color=ft.colors.ON_PRIMARY,
            icon_size=28,
            tooltip="אפשרויות נוספות",
        ),
        text='up',
        label_visible=False,
        offset=ft.transform.Offset(0, -2),
        )

    
    page.appbar.actions.append(menu_button)

    # Define handlers for menu items
    def show_content(type_content, header_content, icon_name=None):
        # Open the help file
        try:
            with open(f"app/{type_content}.md", "r", encoding="utf-8") as file:
                help_content = file.read()
        except FileNotFoundError:
            return

        # Create a BottomSheet to display the help content
        help_sheet = ft.BottomSheet(
            content=ft.Container(
                content=ft.Column(
                    [
                        # Adding a Row to include the icon and header
                        ft.Row(
                            [
                                ft.Icon(icon_name, size=30, color=ft.colors.ON_PRIMARY_CONTAINER) if icon_name else None,
                                ft.Text(
                                    header_content,
                                    theme_style="headlineMedium",
                                    weight=ft.FontWeight.BOLD,
                                    rtl=True,
                                    color=ft.colors.ON_PRIMARY_CONTAINER,
                                ),
                            ],
                            alignment=ft.MainAxisAlignment.CENTER,  # Center the icon and header
                            rtl=True,
                        ),
                        ft.Markdown(help_content, auto_follow_links=True),
                    ],
                    tight=True,
                    rtl=True,
                    scroll=scroll_mode,
                    alignment=ft.MainAxisAlignment.CENTER,
                    horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                    spacing="30",
                ),
                padding=40,
                expand=True,
                bgcolor=ft.colors.SURFACE_VARIANT,
            ),
            show_drag_handle=True,
            elevation=300,
            enable_drag=True,
            is_scroll_controlled=True,
        )

        page.overlay.append(help_sheet)
        help_sheet.open = True
        page.update()
        

    def show_update():

        def close_dialog(e):
            dialog.open = False
            page.update()

        def updating(e):
            """
            פונקציה לפתיחת כתובת האתר להורדת גרסה חדשה בדפדפן
            """
            dialog.open = False

            download_url = "https://nhlocal.github.io/Singles-Sorter/site/download?utm_source=singles_sorter_program&utm_medium=desktop"
            
            page.launch_url(download_url)

            
            # יישום שיטת עדכון אוטומטי
            page.update()
        
        dialog = ft.AlertDialog(
            modal=True,
            inset_padding=0 if ANDROID_MODE else None,
            title=ft.Row([
                ft.Icon(ft.icons.UPDATE, size=40, color=ft.colors.ON_PRIMARY_CONTAINER),
                ft.Text("עדכון גרסה", text_align="center", color=ft.colors.ON_PRIMARY_CONTAINER, weight=ft.FontWeight.BOLD),
            ],
            rtl=True,
            alignment=ft.MainAxisAlignment.CENTER,     
            ), 

            content=ft.Column([
                ft.Text(f"גרסה {update_available} זמינה להורדה", text_align="center", rtl=True, size=20),
                ft.Text(f"מה חדש?", text_align="center", rtl=True, size=18, weight='BOLD'),
                ft.Markdown(release_notes)
                ],
            rtl=True,
            scroll=scroll_mode,
            ),

            actions=[
                ft.TextButton("הורד כעת", on_click=updating),
                ft.TextButton("ביטול", on_click=close_dialog),
            ],
            actions_alignment=ft.MainAxisAlignment.SPACE_AROUND,
        )
        page.overlay.append(dialog)
        dialog.open = True
        page.update()


    def show_settings():
        # Open the help file
        try:
            with open(f"app/add_singers.md", "r", encoding="utf-8") as file:
                add_singers_info = file.read()
        except FileNotFoundError:
            pass
        

        def close_dialog(e):
            dialog.open = False
            page.update()

        def import_csv(e):
            """
            פונקציה לייבוא נתונים מקובץ CSV חיצוני
            """
            # Create the FilePicker instance
            input_file_picker = ft.FilePicker(on_result=lambda e: import_csv_result(e))
            page.overlay.extend([input_file_picker])
            page.update()
            input_file_picker.pick_files(allowed_extensions=["csv"], dialog_title="יבוא רשימת זמרים אישית", allow_multiple=False)

        def import_csv_result(e: ft.FilePickerResultEvent):
            """
            פונקציה לטיפול בתוצאת דיאלוג בחירת קובץ
            """

            file_path = str(e.files[0].path) if e.files else None

            if file_path:
                try:
                    import chardet

                    def detect_encoding(file_path):
                        with open(file_path, 'rb') as file:
                            result = chardet.detect(file.read())
                        return result['encoding']

                    # קריאת נתונים מקובץ CSV
                    encoding = detect_encoding(file_path)
                    with open(file_path, "r", encoding=encoding) as file:
                        reader = csv.reader(file)
                        data = list(reader)

                    # שמירת נתונים לקובץ CSV קיים
                    with open("app/personal-singer-list.csv", "a", newline="", encoding="utf-8") as file:
                        writer = csv.writer(file)
                        writer.writerow([])  # הוספת שורה ריקה לפני הוספת הנתונים החדשים
                        writer.writerows(data)

                    # הצגת הודעת הצלחה
                    snack_bar = show_snackbar("הנתונים יובאו בהצלחה!", ft.colors.GREEN)
                    page.overlay.append(snack_bar)
                    snack_bar.open = True
                    page.update()

                except Exception as error:
                    # הצגת הודעת שגיאה
                    snack_bar = show_snackbar(f"שגיאה: {error}", ft.colors.ERROR)
                    page.overlay.append(snack_bar)
                    snack_bar.open = True
                    page.update()

        def export_csv(e):
            """
            פונקציה לייצוא נתונים לקובץ CSV חיצוני
            """
            # פתיחת דיאלוג שמירת קובץ
            output_file_picker = ft.FilePicker(on_result=lambda e: export_csv_result(e))
            page.overlay.extend([output_file_picker])
            page.update()
            output_file_picker.save_file(allowed_extensions=["csv"], file_name="personal-singer-list.csv", dialog_title="יצוא רשימת זמרים אישית")


        def export_csv_result(e: ft.FilePickerResultEvent):
            """
            פונקציה לטיפול בתוצאת דיאלוג שמירת קובץ
            """
            if e.path:
                try:
                    # העתקת קובץ CSV קיים לקובץ חדש
                    src_path = "app/personal-singer-list.csv"
                    dest_path = e.path

                    # לוודא שנתיב היעד כולל סיומת .csv
                    if not dest_path.endswith(".csv"):
                        dest_path += ".csv"
                    
                    # העתקת הקובץ
                    copy(src_path, dest_path)

                    # הצגת הודעת הצלחה
                    snack_bar = show_snackbar(f"הנתונים יוצאו בהצלחה לקובץ {e.path}", ft.colors.GREEN, mseconds=4000)
                    page.overlay.append(snack_bar)
                    snack_bar.open = True
                    page.update()

                except FileNotFoundError:
                    # הצגת הודעה במקרה שלא נמצא קובץ
                    snack_bar = show_snackbar("לא יצרת עדיין רשימת זמרים אישית!", ft.colors.ERROR, mseconds=4000)
                    page.overlay.append(snack_bar)
                    snack_bar.open = True
                    page.update()

                except Exception as error:
                    # הצגת הודעת שגיאה
                    snack_bar = show_snackbar(f"שגיאה: {error}", ft.colors.ERROR)
                    page.overlay.append(snack_bar)
                    snack_bar.open = True
                    page.update()

        def open_csv(e):
            # יצירת הדיאלוג באמצעות הפונקציה מיובאת
            dialog = create_add_singer_dialog(page, ANDROID_MODE)

            page.overlay.append(dialog)
            dialog.open = True
            page.update()


        # הגדרת תצוגת מידע נוסף על הוספת זמרים
        is_expanded = False

        def toggle_content(e):
            nonlocal is_expanded
            is_expanded = not is_expanded
            info_add_singers.visible = is_expanded
            icon_arrow.icon = "ARROW_DROP_DOWN" if is_expanded else "ARROW_RIGHT"
            page.update()

        icon_arrow = ft.IconButton(
            icon=ft.icons.ARROW_RIGHT,
            on_click=toggle_content,
            tooltip="למידע נוסף"  # טקסט הסבר לחץ
        )
        title_add_singers = ft.Text("הוספת זמרים", weight=ft.FontWeight.BOLD, size=16)
        info_add_singers = ft.Markdown(add_singers_info, visible=False)

        add_singers_options = [
            ft.TextButton("ערוך רשימה", on_click=open_csv),
            ft.TextButton("יבא קובץ CSV", on_click=import_csv),
            ft.TextButton("יצא לקובץ CSV", on_click=export_csv, disabled=ANDROID_MODE),
        ]


        dialog = ft.AlertDialog(
            inset_padding=0 if ANDROID_MODE else 20,
            modal=True,
            #bgcolor=ft.colors.SURFACE_VARIANT,
            title=ft.Row([
                ft.Icon(ft.icons.SETTINGS, size=40, color=ft.colors.ON_PRIMARY_CONTAINER),
                ft.Text("הגדרות מתקדמות", text_align="center", color=ft.colors.ON_PRIMARY_CONTAINER, weight=ft.FontWeight.BOLD),
                ],
                rtl=True,
                alignment=ft.MainAxisAlignment.CENTER,
                ),

            content=ft.Container( # הוספת Container לשליטה ברוחב
                width=None if ANDROID_MODE else page.window.width * 0.7, # קביעת רוחב קבוע
                #height=page.window.width if ANDROID_MODE else page.window.width * 0.7
                content=ft.Column(
                    [
                        ft.Column(
                            [
                                ft.Text("מיון דואטים", weight=ft.FontWeight.BOLD, size=16),
                                ft.Text(
                                    "זוהי תכונה נסיונית, למילוי משוב על חווית השימוש ",
                                    spans=[
                                        ft.TextSpan(
                                            "מלאו טופס כעת",
                                            ft.TextStyle(
                                                decoration=ft.TextDecoration.UNDERLINE,
                                                color=ft.colors.BLUE
                                            ),
                                            url="https://docs.google.com/forms/d/e/1FAIpQLScOaX1wWW1YXXlX4cylMA6LWpO7yIb2fStmjzfSqmLc_V9CIw/viewform?usp=sf_link"
                                        ),
                                    ],
                                    color='red'
                                ),

                                # כפתורי בחירה בין מצב דואט למצב רגיל
                                duet_mode
                            ],
                        ),


                        ft.Column(
                            [
                                ft.Row([icon_arrow, title_add_singers]),
                                info_add_singers,

                                ft.Row(add_singers_options, wrap=ANDROID_MODE,)
                            ],
                            
                        ),


                    ],

                    spacing='25',
                    scroll=scroll_mode,
                    alignment=ft.MainAxisAlignment.START,
                    horizontal_alignment=ft.CrossAxisAlignment.START,
                    rtl=True,
                ),
            ),

            actions=[
                ft.TextButton("סגור", on_click=close_dialog),
            ],
            actions_alignment=ft.MainAxisAlignment.SPACE_AROUND,
        )
        page.overlay.append(dialog)
        dialog.open = True
        page.update()


    # Input fields
    height_button = '50'
    if ANDROID_MODE:
        width_button = '100'
        describe_button = 'בחר'
    else:
        width_button = '150'
        describe_button = 'בחר תיקיה'
    

    # הגדרת סגנון עקבי לכפתורים
    round_button = ft.ButtonStyle(
        shape=ft.RoundedRectangleBorder(radius=10),
        padding=10
    )

    # הגדרת סגנון עקבי לשדות טקסט – הסר round_text_field
    text_field_border = ft.border.all(1, color=ft.colors.OUTLINE_VARIANT)
    text_field_height = 50

    # שדות קלט - עיצוב משופר
    source_dir_input = ft.TextField(
        label="תיקית הסינגלים שלך",
        autofocus=auto_focus,
        rtl=True,
        expand=True,
        border=text_field_border,
        height=text_field_height,
        hint_text=r"C:\Music\סינגלים",
        read_only=ANDROID_MODE,
        content_padding=ft.padding.only(10,15,10,15),
        filled=True,
        border_color=ft.colors.OUTLINE_VARIANT,
    )

    target_dir_input = ft.TextField(
        label="תיקית יעד",
        rtl=True,
        expand=True,
        border=text_field_border,
        height=text_field_height,
        hint_text=r"C:\Music\המוזיקה שלך",
        read_only=ANDROID_MODE,
        content_padding=ft.padding.only(10,15,10,15),  # הוספת content padding
        filled=True, # הוספת filled
        border_color=ft.colors.OUTLINE_VARIANT,

    )
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input))
    
    page.overlay.extend([source_picker, target_picker])

    
    source_dir_button = ft.ElevatedButton(describe_button, icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקיה המכילה את המוזיקה שברצונך לסדר', style=round_button,)
    target_dir_button = ft.ElevatedButton(describe_button, icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקית יעד אליה יוכנסו תיקיות  המוזיקה שיווצרו', style=round_button)


    # Checkboxes
    global copy_mode, main_folder_only, singles_folder, exist_only, abc_sort, duet_mode

    # הגדרות בסיסיות - שימוש ב-Switch עם label דינאמי
    copy_mode = ft.Switch(
        tooltip="קבע אם להעתיק או להעביר את הקבצים לתיקיית היעד",
        label="העתק קבצים" if page.client_storage.get("copy_mode")    else "העבר קבצים",
        value=page.client_storage.get("copy_mode") or False,
        on_change=lambda e: update_switch_label(copy_mode) # עדכון label בעת שינוי
    )

    main_folder_only = ft.Switch(
        label="סרוק תיקיה ראשית בלבד" if page.client_storage.get("main_folder_only") else "סרוק עץ תיקיות",
        tooltip="בחר אם לסרוק את כל תיקיות המשנה או רק את תיקיית המקור הראשית",
        value=page.client_storage.get("main_folder_only") or False,
        on_change=lambda e: update_switch_label(main_folder_only) # עדכון label בעת שינוי
    )

    # פונקציה לעדכון label של ה-Switch
    def update_switch_label(switch):
        if switch == copy_mode:
            switch.label = "העתק קבצים" if switch.value else "העבר קבצים"
        elif switch == main_folder_only:
            switch.label = "סרוק תיקיה ראשית בלבד" if switch.value else "סרוק עץ תיקיות"
        switch.update()
    
    singles_folder = ft.Checkbox(
        label='צור תיקיות סינגלים',
        value=page.client_storage.get("singles_folder") if page.client_storage.get("singles_folder") is not None else True # True כברירת מחדל
    )

    exist_only = ft.Checkbox(
        label="שימוש בתיקיות קיימות",
        value=page.client_storage.get("exist_only") or False
    )

    abc_sort = ft.Checkbox(
        label="מיון אלפביתי",
        value=page.client_storage.get("abc_sort") or False
    )



    # duet_mode
    def on_duet_mode_changed(e):
        page.client_storage.set("duet_mode", duet_mode.value)

    duet_mode = ft.RadioGroup(
        content=ft.Row([
            ft.Radio(value=False, label="העתק לזמר הראשון בשם השיר", disabled=False),
            ft.Radio(value=True, label="העתק לכל הזמרים המופיעים בשם השיר", disabled=False)
        ],
        wrap=True,
        rtl=True),
        value=page.client_storage.get("duet_mode") or False,
        on_change=on_duet_mode_changed  # הוספת טיפול באירוע שינוי
    )
    


    # פונקציה לשמירת ההגדרות
    def open_save_config(e):
        try:
            page.client_storage.set("copy_mode", copy_mode.value)
            page.client_storage.set("main_folder_only", main_folder_only.value)
            page.client_storage.set("singles_folder", singles_folder.value)
            page.client_storage.set("exist_only", exist_only.value)
            page.client_storage.set("abc_sort", abc_sort.value)

            snack_bar = show_snackbar("ההגדרות נשמרו בהצלחה!", ft.colors.GREEN)
        except Exception as error:
            snack_bar = show_snackbar(f"התרחשה שגיאה בעת שמירת ההגדרות: {error}", ft.colors.ERROR)
        finally:
            page.overlay.append(snack_bar)
            snack_bar.open = True
            page.update()
            return


    
    # כפתור שמירת הגדרות
    save_config_button = ft.IconButton(
        icon=ft.icons.SAVE,
        on_click=open_save_config,
        bgcolor=ft.colors.BACKGROUND,
        tooltip='שמור הגדרות מותאמות אישית',
        disabled=False,
    )
    
    
    # Progress bar
    page.window.progress_bar='0.0'
    progress_bar = ft.ProgressBar(width=400, value=0)
    

    # הצגת הודעת אזהרה לפני הפעלת הסריקה
    def show_warning(e):
        mode = e.control.data

        def close_dialog(e):
            dialog.open = False
            page.update()

        def continue_action(e):
            dialog.open = False

            # בקשת הרשאת ניהול קבצים מהמשתמש            
            if ANDROID_MODE:
                def check_permission():
                    o = ph.check_permission(ft.PermissionType.MANAGE_EXTERNAL_STORAGE)
                    return o     

                def request_permission():
                    o = ph.request_permission(ft.PermissionType.MANAGE_EXTERNAL_STORAGE)
                    
                permission_status = check_permission()
                if str(permission_status) != "PermissionStatus.GRANTED":
                    request_permission()

            process_files(e, page, mode)
            
            page.update()
        
        # קביעת תוכן ההודעה והכותרת בהתאם לכפתור שנלחץ
        if mode == "organize":
            title_text = "אשר והתחל"
            content_text = "התוכנה תבצע שינויים בקבצים שלך\n לא ניתן לשחזר!"
        elif mode == "fix":
            title_text = "אשר והתחל"
            content_text = "פעולה זו תתקן ג'יבריש במאפייני הקובץ\nותסיר תוכן מיותר כמו 'חדשות המוזיקה' משמות הקבצים"

        dialog = ft.AlertDialog(
            modal=True,
            title=ft.Text(title_text, text_align="center"),
            content=ft.Text(content_text, text_align="center", rtl=True),
            actions=[
                ft.TextButton("אישור", on_click=continue_action),
                ft.TextButton("ביטול", on_click=close_dialog),
            ],
            actions_alignment=ft.MainAxisAlignment.SPACE_AROUND,
        )
        page.overlay.append(dialog)
        dialog.open = True
        page.update()

    
    # הגדרות עבור כפתורים ראשיים
    if ANDROID_MODE:
        organize_button_title = "מיין"
        fix_button_title = "תקן"
        width_fix_button = '120'
        width_organize_button = '120'

    else:
        organize_button_title = "מיין שירים"
        fix_button_title = "תקן שמות"
        width_fix_button = 170
        width_organize_button = 180


    organize_button = ft.ElevatedButton(
        content=ft.Row(
            [
                ft.Icon(ft.icons.AUTO_FIX_HIGH),
                ft.Text(organize_button_title, size=20),
            ],
            alignment=ft.MainAxisAlignment.CENTER, # מירכוז תוכן הכפתור
        ),
        on_click=show_warning,
        data="organize",
        tooltip="מיון מתקדם של הסינגלים שלך בתיקיות לפי אמנים",
        style=round_button,
        height=60,
        width=width_organize_button,
    )
    
    fixed_button = ft.ElevatedButton(
        content=ft.Row(
            [
                ft.Icon(ft.icons.CLEANING_SERVICES, size=22),
                ft.Text(fix_button_title, size=18),
            ],
            alignment=ft.MainAxisAlignment.CENTER, # מירכוז תוכן הכפתור
        ),
        on_click=show_warning,
        data="fix",
        tooltip="תיקון ג'יבריש במאפייני הקובץ\nוהסרת תוכן מיותר בשמות הקבצים",
        style=round_button,
        height='60',
        width=width_fix_button,
    )


    PADDING_ITEMS_LIST = ft.padding.only(10, 0, 10, 0)


    # הגדרות הממשק הגרפי של התוכנה
    page.add(
        ft.ResponsiveRow(  # Using ResponsiveRow
            controls=[
                 # עמודה לבחירת תיקיות, סרגל התקדמות וכפתורים – גמישה ברוחב
                ft.Column(
                    expand=True,
                    rtl=True,
                    col={"xs": 2, "sm": 1, "md": 1},
                    controls=[
                        ft.Container( # הוספנו Container
                            padding=0, # padding מסביב לכל התוכן
                            margin=10,  # margin מסביב ל-Container
                            expand=True,
                            content=ft.Column( # התוכן הקודם נכנס כאן
                                controls=[   
                                    ft.Row([source_dir_button, source_dir_input], alignment=ft.MainAxisAlignment.CENTER),
                                    ft.Row([target_dir_button, target_dir_input], alignment=ft.MainAxisAlignment.CENTER),

                                    ft.Row([progress_bar], alignment=ft.MainAxisAlignment.CENTER),
                                    
                                    ft.Row(
                                        [
                                            organize_button,
                                            fixed_button,
                                        ],
                                        alignment=ft.MainAxisAlignment.CENTER,
                                        expand=True,
                                    ),
                                ],
                                spacing=15,
                                alignment=ft.MainAxisAlignment.CENTER,
                                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                                expand=True,
                            ),
                        ),
                    ],
                ),

                # כרטיס "התאמה אישית" – גמיש ברוחב
                ft.Card(
                    expand=True,
                    col={"xs": 2, "sm": 1, "md": 1},
                    content=ft.Container(
                        padding=ft.padding.only(10, 20, 10, 10) if ANDROID_MODE else ft.padding.only(30, 30, 30, 20),
                        margin=ft.padding.all(0),
                        content=ft.Column(
                            [
                                ft.Row(
                                    [
                                        ft.Icon(ft.icons.TUNE, color=ft.colors.PRIMARY),
                                        ft.Text("אפשרויות מיון", size=20, color=ft.colors.PRIMARY, weight=ft.FontWeight.BOLD),
                                    ],
                                    alignment=ft.MainAxisAlignment.CENTER,
                                ),

                                ft.ListTile(
                                    title=ft.Text("בסיסי", weight=ft.FontWeight.BOLD, size=18),
                                    content_padding=ft.padding.only(5, 0, 10, 0),
                                    ),
                                
                                ft.ListTile(title=copy_mode,
                                    content_padding=PADDING_ITEMS_LIST,
                                    ),
                                ft.ListTile(title=main_folder_only,
                                    content_padding=PADDING_ITEMS_LIST,
                                    ),

                                ft.ListTile(
                                    title=ft.Text("מתקדם", weight=ft.FontWeight.BOLD, size=18),
                                    content_padding=ft.padding.only(5, 0, 10, 0),
                                    ),

                                ft.ListTile(title=singles_folder,
                                    content_padding=PADDING_ITEMS_LIST,
                                    subtitle=ft.Text('יצירת תיקייה ייעודית בשם "סינגלים" בתוך כל תיקיית אמן',)      
                                    ),
                                ft.ListTile(title=abc_sort,
                                    content_padding=PADDING_ITEMS_LIST,
                                    subtitle=ft.Text("יצירת תיקיות ראשיות לפי אותיות הא'-ב' וארגון תיקיות האמנים בתוכן"),
                                    ),
                                ft.ListTile(title=exist_only,
                                    content_padding=PADDING_ITEMS_LIST,
                                    subtitle=ft.Text(" העברת קבצים רק לתיקיות אמנים הקיימות כבר בתיקית היעד")
                                        ),

                                ft.Row(
                                    [save_config_button],
                                    alignment=ft.MainAxisAlignment.END
                                ),

                            ],
                            spacing=0,
                        ),
                    ),
                ),
            ],
            alignment=ft.MainAxisAlignment.START,  # Distribute space around items
            vertical_alignment=ft.CrossAxisAlignment.START, # align items to top
            spacing=20,
            run_spacing=10,
            columns=2,
            expand=True,
            rtl=True,
        )
    )



    def update_path(e: ft.FilePickerResultEvent, srctar_input: ft.TextField):
        try:
            srctar_input.value = e.path if e.path else None
            srctar_input.update()

        except Exception as error:
            # Display an error message to the user or log the error
            print(f"Error updating path: {error}")
            # Consider using a Snackbar or AlertDialog to display the error to the user

    # פונקציה אחת לטיפול במיון ותיקון שמות
    def process_files(e, page: ft.Page, mode):
        source_dir = source_dir_input.value
        target_dir = target_dir_input.value

        if mode == "organize" and not target_dir:
            snack_bar = show_snackbar("אנא בחר תיקיית מקור ותיקיית יעד!", ft.colors.ERROR)
            page.overlay.append(snack_bar)
            snack_bar.open = True
            page.update()
            return

        if not source_dir:
            snack_bar = show_snackbar("אנא בחר תיקיית מקור!", ft.colors.ERROR)
            page.overlay.append(snack_bar)
            snack_bar.open = True
            page.update()
            return


        def progress_callback(progress):
            progress_num = progress / 100
            progress_bar.value = progress_num
            page.window.progress_bar = str(progress_num)
            page.update()

        try:
            # השבתת כפתור בהתאם לפעולה
            if mode == "organize":
                organize_button.disabled = True
            elif mode == "fix":
                fixed_button.disabled = True
            page.update()

            sorter = MusicSorter(
                source_dir = source_dir, 
                target_dir = target_dir if mode == "organize" else None,  # העברת target_dir רק אם צריך
                copy_mode = copy_mode.value,
                abc_sort = abc_sort.value,
                exist_only = exist_only.value,
                singles_folder = singles_folder.value,
                main_folder_only = main_folder_only.value,
                duet_mode = duet_mode.value,
                progress_callback = progress_callback
            )

            if mode == "organize":
                # מיון הקבצים
                summary = sorter.scan_dir()  # קבלת סיכום הסריקה
                message = "מיון הקבצים הסתיים בהצלחה"

                # יצירת הודעת סיכום
                summary_message = f"""
סה"כ שירים שמויינו: {summary['songs_sorted']}
תיקיות אמנים שנוצרו: {summary['artist_folders_created']}
אלבומים שעובדו: {summary['albums_processed']}

5 האמנים המובילים:
{sorter._format_top_artists(summary['top_artists'])}
"""

                # הצגת הודעת הסיכום ב-AlertDialog
                dialog = ft.AlertDialog(
                    modal=True,
                    title=ft.Text("סיכום מיון", text_align="center"),
                    content=ft.Column(
                        [ft.Text(summary_message, text_align="center", rtl=True)],
                        rtl=True,
                        scroll=scroll_mode,
                        alignment=ft.MainAxisAlignment.CENTER,
                        horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                    ),
                    actions=[
                        ft.TextButton("אישור", on_click=lambda e: (setattr(dialog, 'open', False), page.update())),
                    ],
                    actions_alignment=ft.MainAxisAlignment.CENTER,
                    icon=ft.Icon(ft.icons.CHECK_CIRCLE_OUTLINE, color=ft.colors.GREEN)
                    
                )
                page.overlay.append(dialog)
                dialog.open = True
                page.update()

                

            elif mode == "fix":
                # ניקוי תוכן מיותר משמות הקבצים
                sorter.fix_names()
                message = "תיקון שמות הקבצים הסתיים בהצלחה"

            snack_bar = show_snackbar(message, ft.colors.GREEN, 10000)

        except FileNotFoundError as error:
            snack_bar = show_snackbar(f"{error}", ft.colors.ERROR)
        except PermissionError as error:
            snack_bar = show_snackbar(f"{error}", ft.colors.ERROR)
        except Exception as error:
            snack_bar = show_snackbar(f"שגיאה: {error}", ft.colors.ERROR)

        finally: 
            page.window.progress_bar = '0.0'
            page.overlay.append(snack_bar)
            snack_bar.open = True

            # הפעלת כפתור בהתאם לפעולה
            if mode == "organize":
                organize_button.disabled = False
            elif mode == "fix":
                fixed_button.disabled = False
            page.update()


     # בדיקה אם קיים עדכון זמין
    def update_view():
        try:
            update_available, release_notes = check_for_update(VERSION)
        except:
            update_available = False
            release_notes = None

        # הוספת פריט עדכון רק אם זמין
        if update_available:
            update_item = ft.PopupMenuItem(text="עדכן כעת", icon=ft.icons.UPDATE, data="upadte", on_click=on_menu_selected)
            menu_items.insert(0, update_item)  # הוספת פריט העדכון לתחילת הרשימה

        menu_button.label_visible = bool(update_available)
        menu_button.content.items = menu_items

        menu_button.update()

        return update_available, release_notes

    # הפעלת פונקצייה שפותחת הודעה "מה חדש" בהפעלה הראשונה
    first_run_menu()

    # בדיקה אם קיים עדכון זמין ועדכון התצוגה
    update_available, release_notes =  update_view()

ft.app(target=main)
