import flet as ft


def main(page: ft.Page):
    page.scroll = ft.ScrollMode.ADAPTIVE
    page.appbar = ft.AppBar(title=ft.Text("PermissionHandler Tests"))
    ph = ft.PermissionHandler()
    page.overlay.append(ph)

    def check_permission(e):
        o = ph.check_permission(e.control.data)
        page.add(ft.Text(f"Checked {e.control.data.name}: {o}"))

    def request_permission(e):
        o = ph.request_permission(e.control.data)
        page.add(ft.Text(f"Requested {e.control.data.name}: {o}"))

    def open_app_settings(e):
        o = ph.open_app_settings()
        page.add(ft.Text(f"App Settings: {o}"))

    page.add(
        ft.OutlinedButton(
            "Check STORAGE Permission",
            data=ft.PermissionType.STORAGE,
            on_click=check_permission,
        ),
        ft.OutlinedButton(
            "Request STORAGE Permission",
            data=ft.PermissionType.STORAGE,
            on_click=request_permission,
        ),
        ft.OutlinedButton(
            "Open App Settings",
            on_click=open_app_settings,
        ),

        ft.OutlinedButton(
            "Check AUDIO Permission",
            data=ft.PermissionType.AUDIO,
            on_click=check_permission,
        ),
        ft.OutlinedButton(
            "Request AUDIO Permission",
            data=ft.PermissionType.AUDIO,
            on_click=request_permission,
        ),

        ft.OutlinedButton(
            "Check SYSTEM_ALERT_WINDOW Permission",
            data=ft.PermissionType.SYSTEM_ALERT_WINDOW,
            on_click=check_permission,
        ),
        ft.OutlinedButton(
            "Request SYSTEM_ALERT_WINDOW Permission",
            data=ft.PermissionType.SYSTEM_ALERT_WINDOW,
            on_click=request_permission,
        ),

        ft.OutlinedButton(
            "Check MANAGE_EXTERNAL_STORAGE Permission",
            data=ft.PermissionType.MANAGE_EXTERNAL_STORAGE,
            on_click=check_permission,
        ),
        ft.OutlinedButton(
            "Request MANAGE_EXTERNAL_STORAGE Permission",
            data=ft.PermissionType.MANAGE_EXTERNAL_STORAGE,
            on_click=request_permission,
        ),

        ft.OutlinedButton(
            "Check ACCESS_MEDIA_LOCATION Permission",
            data=ft.PermissionType.BLUETOOTH,
            on_click=check_permission,
        ),
        ft.OutlinedButton(
            "Request ACCESS_MEDIA_LOCATION Permission",
            data=ft.PermissionType.BLUETOOTH,
            on_click=request_permission,
        ),



    )


ft.app(main)