import flet as ft

def main(page: ft.Page):
    page.title = "My Flet App"


    if page.platform == ft.PagePlatform.ANDROID:
        padding_num = 10  # שוליים צרות יותר לאנדרואיד
        page.scroll = ft.ScrollMode.AUTO  # מאפשר גלילה
    else:
        padding_num = 100  # שוליים רחבות יותר למערכות הפעלה אחרות
        page.scroll = None

    container = ft.Container(
        content=ft.Column(
            controls=[
                ft.Text("ברוכים הבאים לאפליקציה שלי! 😊"),
                ft.Text("התוכן כאן מותאם למסכים קטנים וגדולים כאחד."),
                # הוסף כאן את שאר התוכן שלך
            ],
        ),
        padding=padding_num,
        #expand=True,
    )

    page.add(container)

ft.app(target=main)
