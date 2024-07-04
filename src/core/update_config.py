# -*- coding: utf-8 -*-
import requests

# ישום עדכון וזיהוי גרסה חדשה
def check_for_update(current_version):
    # URL של GitHub API לקבלת הגרסה האחרונה
    url = "https://api.github.com/repos/NHLOCAL/Singles-Sorter/releases/latest"
    
    try:
        # בקשה ל-API לקבלת הגרסה האחרונה
        response = requests.get(url)
        response.raise_for_status()  # לבדוק אם התקבלה תשובה תקינה
        latest_release = response.json()  # לקבל JSON
        
        # לבדוק אם מדובר בגרסת בטא
        is_prerelease = latest_release['prerelease']
        
        # לגרסה החדשה ביותר
        latest_version = latest_release['tag_name'].strip()
        
        # להשוות בין גרסה נוכחית לגרסה באתר
        if latest_version > f"v{current_version}" and not is_prerelease:
            return latest_version
        else:
            return False
        
    except requests.RequestException as e:
        #print(f"Error fetching version data: {e}")
        return False

if __name__ == '__main__':

    from singles_sorter_v3 import __VERSION__

    # גרסה נוכחית מהתוכנה שלך
    current_version = __VERSION__

    # קריאה לפונקציה
    new_version = check_for_update(current_version)
    if new_version:
        print(f"גרסה חדשה זמינה: {new_version} 🚀")
    else:
        print(f"אין גרסה חדשה זמינה. 🙁")
