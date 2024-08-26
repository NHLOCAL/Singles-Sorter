# file: add_singer_dialog.py

import flet as ft
import csv

def create_add_singer_dialog(page: ft.Page, ANDROID_MODE=False, csv_file="app/personal-singer-list.csv"):
    # אייקונים
    singer_icon = ft.Icon(ft.icons.PERSON_2_OUTLINED, size=24)
    folder_icon = ft.Icon(ft.icons.FOLDER_OPEN_ROUNDED, size=24)
    add_row_icon = ft.icons.EXPAND_MORE

    # פונקציה ליצירת תא טבלה ניתנת לעריכה
    def create_table_cell():
        return ft.DataCell(ft.TextField(value="", border=ft.InputBorder.NONE, text_align=ft.TextAlign.RIGHT, rtl=True))

    # טעינת נתונים מקובץ CSV
    def load_data():
        try:
            with open(csv_file, "r", encoding="utf-8") as file:
                reader = csv.reader(file)
                data = [ft.DataRow(cells=[
                    ft.DataCell(ft.TextField(value=row[1], border=ft.InputBorder.NONE, text_align=ft.TextAlign.RIGHT, rtl=True)),
                    ft.DataCell(ft.TextField(value=row[0], border=ft.InputBorder.NONE, text_align=ft.TextAlign.RIGHT, rtl=True))
                ]) for row in reader if any(row)]
                # אם אין נתונים, הוסף 5 שורות ריקות
                if not data:
                    data = [ft.DataRow(cells=[create_table_cell(), create_table_cell()]) for _ in range(5)]
                return data
        except FileNotFoundError:
            return [ft.DataRow(cells=[create_table_cell(), create_table_cell()]) for _ in range(5)]

    # פונקציה להוספת שורה
    def add_row_clicked(e):
        table.rows.append(
            ft.DataRow(
                cells=[
                    create_table_cell(),
                    create_table_cell(),
                ]
            )
        )
        page.update()

    # פונקציה לאישור
    def confirm_clicked(e):
        # סינון שורות ריקות או עם תא אחד ריק
        valid_rows = [row for row in table.rows if all(cell.content.value for cell in row.cells)]

        # שמירת נתונים לקובץ CSV
        with open(csv_file, "w", newline="", encoding="utf-8") as file:
            writer = csv.writer(file)
            for row in valid_rows:  # שמירת שורות תקינות בלבד
                writer.writerow([row.cells[1].content.value, row.cells[0].content.value])
        # סגירת הדיאלוג
        dialog.open = False
        page.update()

    # פונקציה לביטול
    def cancel_clicked(e):
        # סגירת הדיאלוג
        dialog.open = False
        page.update()
    
    cell_width = None if ANDROID_MODE else 150

    # יצירת הטבלה
    table = ft.DataTable(
        vertical_lines=ft.BorderSide(1, ft.colors.ON_PRIMARY_CONTAINER),
        horizontal_lines=ft.BorderSide(1, ft.colors.ON_PRIMARY_CONTAINER),
        border=ft.border.all(2, ft.colors.OUTLINE),
        border_radius=15,
        column_spacing=20,
        horizontal_margin=10,
        divider_thickness=1,
        columns=[
            ft.DataColumn(ft.Row([folder_icon, ft.Text("שם תיקיה", text_align=ft.TextAlign.RIGHT, rtl=True, size=16)], width=cell_width, rtl=True, alignment=ft.MainAxisAlignment.CENTER),  # כותרת עמודה א'
                          numeric=False),
            ft.DataColumn(ft.Row([singer_icon, ft.Text("שם זמר", text_align=ft.TextAlign.RIGHT, rtl=True, size=16)], width=cell_width, rtl=True, alignment=ft.MainAxisAlignment.CENTER),  # כותרת עמודה ב'
                          numeric=False),
        ],
        rows=load_data()
    )

    # יצירת הדיאלוג
    dialog = ft.AlertDialog(
        inset_padding=0 if ANDROID_MODE else 24,
        title=ft.Row([
            ft.Icon(ft.icons.PERSON_ADD, size=40,  color=ft.colors.ON_PRIMARY_CONTAINER),
            ft.Text("הוספת זמרים",  color=ft.colors.ON_PRIMARY_CONTAINER, text_align=ft.TextAlign.CENTER, weight=ft.FontWeight.BOLD),
        ], 
        rtl=True,
        alignment=ft.MainAxisAlignment.CENTER,
        ),
        content=ft.Column(
            [
                table,
                ft.Row(
                    [
                        ft.IconButton(icon=add_row_icon, on_click=add_row_clicked, icon_size=30, tooltip="הוסף שורה",),
                    ],
                    alignment=ft.MainAxisAlignment.CENTER,
                    rtl=True,
                ),
                ft.Row(
                    [
                        ft.ElevatedButton("ביטול", on_click=cancel_clicked),
                        ft.ElevatedButton("אישור", on_click=confirm_clicked),
                    ],
                    alignment=ft.MainAxisAlignment.CENTER,
                    rtl=True,
                ),
            ],
            scroll="auto"
        ),
        modal=True,
    )

    return dialog