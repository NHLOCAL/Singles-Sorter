"""Android permission policy for folder-based music organization."""

import flet_permission_handler as fph


def required_android_permissions() -> tuple[fph.Permission, ...]:
    """Return non-overlapping runtime requests for arbitrary folder access.

    ``MANAGE_EXTERNAL_STORAGE`` is requested on its own because Android opens a
    dedicated settings surface for it. Starting another permission request at
    the same time causes ``PermissionHandler`` to reject the second request.
    """

    return (fph.Permission.MANAGE_EXTERNAL_STORAGE,)
