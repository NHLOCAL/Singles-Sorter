import flet as ft

def main(page: ft.Page):
    page.title = "Responsive Layout Demo - Corrected Order"

    # יצירת הרכיבים
    component1 = ft.Container(
        content=ft.Text("רכיב 1", style="headlineMedium"),
        bgcolor=ft.colors.BLUE_100,
        padding=20,
    )
    component2 = ft.Container(
        content=ft.Text("רכיב 2", style="headlineMedium"),
        bgcolor=ft.colors.RED_100,
        padding=20,
    )
    component3 = ft.Container(
        content=ft.Text("רכיב 3", style="headlineMedium"),
        bgcolor=ft.colors.GREEN_100,
        padding=20,
    )

    page.add(
        ft.ResponsiveRow( # שורה רספונסיבית ראשית
            [
                ft.Column( # עמודה ראשונה - רכיב 1
                    [component1],
                    col={"md": 6, "xs":12},  # 6 עמודות ב-md ומעלה, 12 ב-xs
                    expand=True,
                    horizontal_alignment=ft.CrossAxisAlignment.CENTER,
                ),
                ft.Column( # עמודה שנייה - רכיב 3 *רק* במסך גדול
                    [component3],
                    col={"md": 6},  # 6 עמודות ב-md ומעלה. לא מוגדר ב-xs, אז לא יוצג
                    expand=True,
                    horizontal_alignment=ft.CrossAxisAlignment.CENTER,

                ),

            ],
            alignment=ft.MainAxisAlignment.SPACE_BETWEEN, # מרווח בין העמודות
            vertical_alignment=ft.CrossAxisAlignment.START, # יישור למעלה
            run_spacing=20,

        ),
        ft.Column( # עמודה *מחוץ* ל-ResponsiveRow - רכיב 3 *רק* במסך קטן + רכיב 2
            [
                ft.Container( # רכיב 3 *רק* במסך קטן
                    content=component3,
                    col={"xs": 12}, # 12 עמודות (רוחב מלא) ב-xs. לא מוגדר ב-md, אז לא יוצג.
                ),
                component2,
            ],
            col={"xs": 12},  # 12 עמודות ב-xs. לא מוגדר ב-md, אז לא ישפיע.
            horizontal_alignment=ft.CrossAxisAlignment.CENTER,
        )

    )

ft.app(target=main)