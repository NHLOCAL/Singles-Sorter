import flet as ft

def main(page: ft.Page):
    page.title = "טבלה ניתנת לעריכה"
    page.rtl = True  # הגדרת תצוגת RTL

    # אייקונים
    singer_icon = ft.Icon(ft.icons.PERSON_2_OUTLINED, size=24)
    folder_icon = ft.Icon(ft.icons.FOLDER_OPEN_ROUNDED, size=24)

    # הגדרת תא טבלה ניתנת לעריכה
    TABLE_CELL = ft.DataCell(ft.TextField(value="", border=ft.InputBorder.NONE, width=150, expand=True, text_align=ft.TextAlign.RIGHT, rtl=True), show_edit_icon=True,)

    # פונקציה להוספת שורה
    def add_row_clicked(e):
        table.rows.append(
            ft.DataRow(
                cells=[
                    TABLE_CELL,
                    TABLE_CELL,
                ]
            )
        )
        page.update()

    # פונקציה לאישור
    def confirm_clicked(e):
        # טיפול בנתונים מהטבלה
        print("אישור נלחץ")
        # סגירת הדיאלוג
        page.dialog.open = False
        page.update()

    # פונקציה לביטול
    def cancel_clicked(e):
        # סגירת הדיאלוג
        page.dialog.open = False
        page.update()

    # יצירת הטבלה
    table = ft.DataTable(
        vertical_lines=ft.BorderSide(1, ft.colors.ON_PRIMARY_CONTAINER),
        horizontal_lines=ft.BorderSide(1, ft.colors.ON_PRIMARY_CONTAINER),
        border=ft.border.all(2, ft.colors.OUTLINE),
        border_radius=15,
        column_spacing=20,
        horizontal_margin=20,
        divider_thickness=1,
        #heading_row_color=ft.colors.ON_PRIMARY,  # צבע שורת הכותרות
        columns=[
            ft.DataColumn(ft.Row([singer_icon, ft.Text("שם זמר", text_align=ft.TextAlign.RIGHT, width=150, rtl=True, size=16)], rtl=True, alignment=ft.MainAxisAlignment.CENTER),  # כותרת עמודה א'
                          numeric=False),
            ft.DataColumn(ft.Row([folder_icon, ft.Text("שם תיקיה", text_align=ft.TextAlign.RIGHT, width=150, rtl=True, size=16)], rtl=True, alignment=ft.MainAxisAlignment.CENTER),  # כותרת עמודה ב'
                          numeric=False),
        ],
        rows=[
            ft.DataRow(
                cells=[
                    TABLE_CELL,
                    TABLE_CELL,
                ]
            ),
            ft.DataRow(
                cells=[
                    TABLE_CELL,
                    TABLE_CELL,
                ]
            ),
            ft.DataRow(
                cells=[
                    TABLE_CELL,
                    TABLE_CELL,
                ]
            ),
            ft.DataRow(
                cells=[
                    TABLE_CELL,
                    TABLE_CELL,
                ]
            ),

        ]
    )

    # יצירת הדיאלוג
    dlg = ft.AlertDialog(
        title=ft.Text("הוספת זמרים", text_align=ft.TextAlign.CENTER),
        content=ft.Column(
            [
                table,
                ft.Row(
                    [
                        ft.ElevatedButton("הוסף שורה", on_click=add_row_clicked),
                    ],
                    alignment=ft.MainAxisAlignment.CENTER,
                    rtl=True,
                ),
                ft.Row(
                    [
                        ft.ElevatedButton("אישור", on_click=confirm_clicked),
                        ft.ElevatedButton("ביטול", on_click=cancel_clicked),
                    ],
                    alignment=ft.MainAxisAlignment.CENTER,
                    rtl=True,
                ),
            ],
            scroll="auto"
        ),
        modal=True,
    )

    page.dialog = dlg

    # פונקציה לפתיחת הדיאלוג
    def open_dlg_modal(e):
        page.dialog.open = True
        page.update()

    page.add(
        ft.ElevatedButton("פתח טבלה", on_click=open_dlg_modal),
    )

ft.app(target=main)