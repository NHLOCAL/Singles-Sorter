# -*- coding: utf-8 -*-
import flet as ft
import os

# קבצי התוכנה
from singles_sorter_v4 import MusicSorter, __VERSION__
from update_config import check_for_update


# גרסת התוכנה
global VERSION
VERSION = __VERSION__



def main(page: ft.Page):

    # הגדרת זיהוי הפעלה על אנדרואיד
    global ANDROID_MODE
    ANDROID_MODE = True if page.platform == ft.PagePlatform.ANDROID else False

    page.title = "מסדר הסינגלים"
    page.vertical_alignment = ft.MainAxisAlignment.SPACE_BETWEEN
    page.theme_mode = ft.ThemeMode.LIGHT
    page.rtl = True
    #page.bgcolor = "#f5f5f5"
    page.theme = ft.Theme(color_scheme_seed="#2196f3")
    ph = ft.PermissionHandler()
    page.overlay.append(ph)

    # הגדרה אוטומטית מותאמת למערכת ההפעלה
    if ANDROID_MODE:
        page.padding = ft.padding.only(20, 10, 20, 0)
        page.scroll = ft.ScrollMode.HIDDEN
        scroll_mode = ft.ScrollMode.HIDDEN
        auto_focus=False

    else:
        page.padding = ft.padding.only(60, 20, 60, 20)
        page.window.height = 810
        page.window.width = 940
        page.scroll = ft.ScrollMode.ADAPTIVE
        scroll_mode = ft.ScrollMode.AUTO
        auto_focus=True

    # Consistent button style definition
    round_button = ft.ButtonStyle(shape=ft.RoundedRectangleBorder(radius=15))

    # פונקצייה להצגת הודעה קופצת בתחתית המסך עם פרמטרים שונים
    show_snackbar = lambda message_text, color, mseconds=3000, : ft.SnackBar(content=ft.Text(message_text), bgcolor=color, duration=mseconds)


    # פונקציה לפתיחת הודעת מה חדש בהפעלה הראשונה של התוכנה
    def first_run_menu():
        if not page.client_storage.get("first_run"):
            show_content('whats-new', 'מה חדש', ft.icons.NEW_RELEASES)
            page.client_storage.set("first_run", True)  # סימון שההודעה הוצגה
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


    try:
        update_available, release_notes = check_for_update(VERSION)
    except:
        update_available = False
        

    # תפריט אפשרויות נוספות
    menu_items = [
        ft.PopupMenuItem(text="עזרה", icon=ft.icons.HELP, data="help", on_click=on_menu_selected),
        ft.PopupMenuItem(text="אודות התוכנה", icon=ft.icons.INFO, data="about", on_click=on_menu_selected),
        ft.PopupMenuItem(text="מה חדש", icon=ft.icons.NEW_RELEASES, data="whats_new", on_click=on_menu_selected),
        ft.PopupMenuItem(text="הגדרות מתקדמות", icon=ft.icons.SETTINGS, data="settings", on_click=on_menu_selected),
    ]

    # הוספת פריט עדכון רק אם זמין
    if update_available:
        update_item = ft.PopupMenuItem(text="עדכן כעת", icon=ft.icons.UPDATE, data="upadte", on_click=on_menu_selected)
        menu_items.insert(0, update_item)  # הוספת פריט העדכון לתחילת הרשימה

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
        label_visible=bool(update_available),
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
            page.dialog.open = False
            page.update()

        def updating(e):
            """
            פונקציה לפתיחת כתובת האתר להורדת גרסה חדשה בדפדפן
            """
            page.dialog.open = False

            download_url = "https://nhlocal.github.io/Singles-Sorter/site/download?utm_source=singles_sorter_program&utm_medium=desktop"
            
            page.launch_url(download_url)

            
            # יישום שיטת עדכון אוטומטי
            page.update()
        
        page.dialog = ft.AlertDialog(
            modal=True,
            icon=ft.Icon(ft.icons.UPDATE, size=30, color=ft.colors.ON_SECONDARY_CONTAINER),
            title=ft.Text("עדכון גרסה", text_align="center"),
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
        page.dialog.open = True
        page.update()


    def show_settings():
        # Open the help file
        try:
            with open(f"app/add_singers.md", "r", encoding="utf-8") as file:
                add_singers_info = file.read()
        except FileNotFoundError:
            pass
        

        def close_dialog(e):
            page.dialog.open = False
            page.update()

        def import_csv(e):
            pass

        def export_csv(e):
            pass

        def open_csv(e):
            old_csv = os.path.abspath("app/disable-singer-list.csv")
            personal_csv = os.path.abspath("app/personal-singer-list.csv")

            # שינוי שם קובץ csv לפני ביצוע שינויים
            if os.path.exists(old_csv):
                try:
                    os.rename(old_csv, personal_csv)
                except PermissionError:
                    print(f"Permission denied for renaming the file '{old_csv}'.")
                except Exception as e:
                    print(f"An error occurred while trying to rename the file: {e}")

            # פתיחת קובץ ה-CSV לצורך עריכה על ידי המשתמש
            try:
                os.startfile(personal_csv)
            except AttributeError:
                print("This feature is not supported on Windows.")


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

        page.dialog = ft.AlertDialog(
            modal=True,
            #bgcolor=ft.colors.SURFACE_VARIANT,
            icon=ft.Icon(ft.icons.SETTINGS, size=30, color=ft.colors.ON_PRIMARY_CONTAINER),
            title=ft.Text("הגדרות מתקדמות", text_align="center", color=ft.colors.ON_PRIMARY_CONTAINER, weight=ft.FontWeight.BOLD),
            content=ft.Container( # הוספת Container לשליטה ברוחב
                width=page.window.width * 1.0 if ANDROID_MODE else page.window.width * 0.7, # קביעת רוחב קבוע
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

                                ft.Row(
                                    [
                                        ft.TextButton("ערוך קובץ", on_click=open_csv, disabled=ANDROID_MODE),
                                        ft.TextButton("ייבא קובץ", on_click=import_csv, disabled=True),
                                        ft.TextButton("ייצא קובץ", on_click=export_csv, disabled=True),
                                    ]
                                ),
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
        page.dialog.open = True
        page.update()


    # Input fields
    height_button = '50'
    if ANDROID_MODE:
        width_button = '100'
        describe_button = 'בחר'
    else:
        width_button = '150'
        describe_button = 'בחר תיקיה'
    
    round_text_field = ft.border_radius.only(15, 10, 15, 10)

    source_dir_input = ft.TextField(label="תיקית הסינגלים שלך", autofocus=auto_focus, rtl=True, expand=True, border_radius=round_text_field, border=ft.border.all(2, color=ft.colors.OUTLINE), height='50', hint_text=r"C:\Music\סינגלים", read_only=ANDROID_MODE)

    target_dir_input = ft.TextField(label="תיקית יעד", rtl=True, expand=True, border=ft.border.all(2, color=ft.colors.OUTLINE), border_radius=round_text_field, height='50', hint_text=r"C:\Music\המוזיקה שלך",  read_only=ANDROID_MODE)
    
    source_picker = ft.FilePicker(on_result=lambda e: update_path(e, source_dir_input, "src"))
    target_picker = ft.FilePicker(on_result=lambda e: update_path(e, target_dir_input, "tar"))
    
    page.overlay.extend([source_picker, target_picker])

    
    source_dir_button = ft.ElevatedButton(describe_button, icon=ft.icons.FOLDER_OPEN, on_click=lambda _: source_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקיה המכילה את המוזיקה שברצונך לסדר', style=round_button,)
    target_dir_button = ft.ElevatedButton(describe_button, icon=ft.icons.FOLDER_OPEN, on_click=lambda _: target_picker.get_directory_path(), height=height_button, width=width_button, tooltip='בחירת תיקית יעד אליה יוכנסו תיקיות  המוזיקה שיווצרו', style=round_button)


    # Checkboxes
    global copy_mode, main_folder_only, singles_folder, exist_only, abc_sort, duet_mode

    # טעינת הגדרות שמורות
    copy_mode = ft.Checkbox(
        label="העתק קבצים (העברה היא ברירת המחדל)",
        tooltip="סמן אם ברצונך לבצע העתקה של הקבצים כברירת מחדל תתבצע העברה",
        value=page.client_storage.get("copy_mode") or False  # False כברירת מחדל
    )
    main_folder_only = ft.Checkbox(
        label="סרוק תיקיה ראשית בלבד",
        tooltip="אם מסומן, התוכנה תסרוק רק את התיקייה הראשית ולא תתי תיקיות",
        value=page.client_storage.get("main_folder_only") or False
    )
    singles_folder = ft.Checkbox(
        label='צור תיקיות סינגלים פנימיות',
        tooltip="סמן אם ברצונך ליצור תיקיות פנימיות בתוך תיקיות הזמרים אליהם יועברו הסינגלים",
        value=page.client_storage.get("singles_folder") or True # True כברירת מחדל
    )
    exist_only = ft.Checkbox(
        label="השתמש בתיקיות קיימות בלבד",
        tooltip="אם מסומן, התוכנה תעביר קבצים רק לתיקיות זמרים קיימות ולא תיצור חדשות",
        value=page.client_storage.get("exist_only") or False
    )
    abc_sort = ft.Checkbox(
        label="צור תיקיות ראשיות לפי ה-א' ב'",
        tooltip="אם מסומן, התוכנה תיצור תיקיה ראשית לכל אות באלפבית",
        value=page.client_storage.get("abc_sort") or False
    )


    # duet_mode
    def on_duet_mode_changed(e):
        page.client_storage.set("duet_mode", duet_mode.value)

    duet_mode = ft.RadioGroup(
        content=ft.Column([
            ft.Radio(value=False, label="העתק לזמר הראשון בשם השיר", disabled=False),
            ft.Radio(value=True, label="העתק לכל הזמרים המופיעים בשם השיר", disabled=False)
        ],
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

            page.snack_bar = show_snackbar("ההגדרות נשמרו בהצלחה!", ft.colors.GREEN)
        except Exception as error:
            page.snack_bar = show_snackbar(f"התרחשה שגיאה בעת שמירת ההגדרות: {error}", ft.colors.ERROR)
        finally:
            page.snack_bar.open = True
            page.update()


    
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
            page.dialog.open = False
            page.update()

        def continue_action(e):
            page.dialog.open = False

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
            content_text = "התוכנה מיועדת לסינגלים בלבד\n מיון תיקיות אלבומים צפויה לשבש אותם!"
        elif mode == "fix":
            title_text = "אשר והתחל"
            content_text = "פעולה זו תתקן ג'יבריש במאפייני הקובץ\nותסיר תוכן מיותר כמו 'חדשות המוזיקה' משמות הקבצים"

        page.dialog = ft.AlertDialog(
            modal=True,
            title=ft.Text(title_text, text_align="center"),
            content=ft.Text(content_text, text_align="center", rtl=True),

            actions=[
                ft.TextButton("אישור", on_click=continue_action),
                ft.TextButton("ביטול", on_click=close_dialog),
            ],
            actions_alignment=ft.MainAxisAlignment.SPACE_AROUND,
        )
        page.dialog.open = True
        page.update()

    
    # הגדרות עבור כפתורים ראשיים
    if ANDROID_MODE:
        organize_button_title = "מיין"
        fix_button_title = "תקן"
        width_fix_button = None
        width_organize_button = None  

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



    # הגדרות הממשק הגרפי של התוכנה
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
 
            margin = ft.margin.only(0, 10, 0, 20)

        ),


        ft.Container(
            content=ft.Column(
                [
                    # התאמה אישית
                    ft.Row(
                        [
                            ft.Icon(ft.icons.TUNE),  # סמל כיוון
                            ft.Text("התאמה אישית", size=20, color=ft.colors.PRIMARY, weight=ft.FontWeight.BOLD),
                        ],
                        alignment=ft.MainAxisAlignment.CENTER,
                    ),

                    # הגדרות בסיסיות
                    ft.Row(
                        [
                            #ft.Icon(ft.icons.HOME),  # סמל בית
                            ft.Text("הגדרות בסיסיות", weight=ft.FontWeight.BOLD),
                        ],
                        alignment=ft.MainAxisAlignment.START,
                    ),
                    copy_mode,
                    main_folder_only,

                    # מתקדם
                    ft.Row(
                        [
                            #ft.Icon(ft.icons.BUILD),  # סמל בנייה
                            ft.Text("מתקדם", weight=ft.FontWeight.BOLD),
                        ],
                        alignment=ft.MainAxisAlignment.START,
                    ),
                    singles_folder,
                    exist_only,

                    ft.Row(
                        [
                            abc_sort,
                            save_config_button,
                        ],
                        alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
                    ),
                ],
                alignment=ft.MainAxisAlignment.CENTER,
                horizontal_alignment=ft.CrossAxisAlignment.CENTER,
            ),

            margin = ft.margin.only(0, 5, 0, 10),
            border=ft.border.all(2, color=ft.colors.OUTLINE),
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
                        organize_button,
                        fixed_button,
                    ],

                    alignment=ft.MainAxisAlignment.CENTER,
                ),

                ],
                
                spacing='20',
            ),

            margin = ft.margin.all(0),
            padding= ft.padding.only(10, 10, 10, 5),
            alignment=ft.alignment.center,
        )
    )




    def update_path(e: ft.FilePickerResultEvent, target_input: ft.TextField, src_tar):
        try:
            target_input.value = e.path if e.path else None
            target_input.update()

            if src_tar == "src":
                global source_path
                source_path = e.path
            elif src_tar == "tar":
                global target_path
                target_path = e.path

        except Exception as error:
            # Display an error message to the user or log the error
            print(f"Error updating path: {error}")
            # Consider using a Snackbar or AlertDialog to display the error to the user
        

    # פונקציה אחת לטיפול במיון ותיקון שמות
    def process_files(e, page: ft.Page, mode):
        source_dir = source_path if ANDROID_MODE else source_dir_input.value
        target_dir = target_path if ANDROID_MODE else target_dir_input.value

        if mode == "organize" and not target_dir:
            page.snack_bar = show_snackbar("אנא בחר תיקיית מקור ותיקיית יעד!", ft.colors.ERROR)
            page.snack_bar.open = True
            page.update()
            return

        if not source_dir:
            page.snack_bar = show_snackbar("אנא בחר תיקיית מקור!", ft.colors.ERROR)
            page.snack_bar.open = True
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
                page.dialog = ft.AlertDialog(
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
                        ft.TextButton("אישור", on_click=lambda e: (setattr(page.dialog, 'open', False), page.update())),
                    ],
                    actions_alignment=ft.MainAxisAlignment.CENTER,
                    icon=ft.Icon(ft.icons.CHECK_CIRCLE_OUTLINE, color=ft.colors.GREEN)
                    
                )
                page.dialog.open = True
                

            elif mode == "fix":
                # ניקוי תוכן מיותר משמות הקבצים
                sorter.fix_names()
                message = "תיקון שמות הקבצים הסתיים בהצלחה"

            page.snack_bar = show_snackbar(message, ft.colors.GREEN, 10000)

        except FileNotFoundError as error:
            page.snack_bar = show_snackbar(f"{error}", ft.colors.ERROR)
        except PermissionError as error:
            page.snack_bar = show_snackbar(f"{error}", ft.colors.ERROR)
        except Exception as error:
            page.snack_bar = show_snackbar(f"שגיאה: {error}", ft.colors.ERROR)

        finally: 
            page.window.progress_bar = '0.0'
            page.snack_bar.open = True

            # הפעלת כפתור בהתאם לפעולה
            if mode == "organize":
                organize_button.disabled = False
            elif mode == "fix":
                fixed_button.disabled = False
            page.update()

    # הפעלת פונקצייה שפותחת הודעה "מה חדש" בהפעלה הראשונה
    first_run_menu()



ft.app(target=main)
