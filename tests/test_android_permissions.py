import flet_permission_handler as fph

from singlesorter_gui.services.permissions import required_android_permissions


def test_folder_sorting_requests_one_non_overlapping_android_permission():
    assert required_android_permissions() == (fph.Permission.MANAGE_EXTERNAL_STORAGE,)
