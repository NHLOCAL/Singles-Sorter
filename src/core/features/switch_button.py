def copy_mode_changed(e):
    user_config['general']['copy_mode'] = e.control.value
    copy_mode.label = "העתק קבצים" if e.control.value else "העבר קבצים"
    page.update()

def main_folder_only_changed(e):
    user_config['general']['main_folder_only'] = e.control.value
    main_folder_only.label = "סרוק תיקיה ראשית בלבד" if e.control.value else "סרוק עץ תיקיות"
    page.update()

copy_mode = ft.Switch(
    label="העבר קבצים" if user_config['general']['copy_mode'] is False else "העתק קבצים",
    tooltip="סמן אם ברצונך לבצע העתקה של הקבצים כברירת מחדל תתבצע העברה",
    value=user_config['general']['copy_mode'],
    on_change=copy_mode_changed,
    
)

main_folder_only = ft.Switch(
    label="סרוק עץ תיקיות" if user_config['general']['main_folder_only'] is False else "סרוק תיקיה ראשית בלבד",
    tooltip="אם מסומן, התוכנה תסרוק רק את התיקייה הראשית ולא תתי תיקיות",
    value=user_config['general']['main_folder_only'],
    on_change=main_folder_only_changed
)