"""Full-screen, adaptive information pages."""

import flet as ft


def information_sheet(*, title: str, markdown: str, icon, on_close) -> ft.BottomSheet:
    body = ft.Markdown(
        markdown,
        selectable=True,
        extension_set=ft.MarkdownExtensionSet.GITHUB_WEB,
        auto_follow_links=True,
    )
    scrolling_content = ft.Column([body], expand=True, scroll=ft.ScrollMode.AUTO)
    return ft.BottomSheet(
        content=ft.Container(
            content=ft.Column(
                [
                    ft.Row(
                        [
                            ft.Icon(icon, size=28),
                            ft.Text(title, size=24, weight=ft.FontWeight.BOLD, expand=True),
                        ],
                        rtl=True,
                    ),
                    ft.Divider(height=1),
                    scrolling_content,
                    ft.FilledButton(
                        "סגירה",
                        icon=ft.Icons.CLOSE_ROUNDED,
                        on_click=on_close,
                        height=48,
                    ),
                ],
                expand=True,
                spacing=12,
            ),
            expand=True,
            padding=ft.Padding.only(left=24, top=12, right=24, bottom=12),
        ),
        fullscreen=True,
        dismissible=True,
        show_drag_handle=True,
        use_safe_area=True,
    )
