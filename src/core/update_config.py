# -*- coding: utf-8 -*-
import requests

def check_for_update(current_version):
    url = "https://api.github.com/repos/NHLOCAL/Singles-Sorter/releases/latest"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        latest_release = response.json()
        
        is_prerelease = latest_release['prerelease']
        latest_version = latest_release['tag_name'].strip()
        
        if latest_version > f"v{current_version}" and not is_prerelease:
            return (
                latest_version,
                latest_release.get('body', '')
            )
        else:
            return False
        
    except requests.RequestException as e:
        print(f"שגיאה בקבלת נתוני גרסה: {e}")
        return False

if __name__ == '__main__':
    from singles_sorter_v4 import __VERSION__
    current_version = __VERSION__

    update_info = check_for_update(current_version)
    if update_info:
        new_version, release_notes = update_info
        print(f"גרסה חדשה זמינה: {new_version} 🚀")
        print(f"הערות שחרור:\n{release_notes}")
    else:
        print("אין גרסה חדשה זמינה. 🙁")