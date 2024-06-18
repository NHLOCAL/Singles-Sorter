import requests
import pickle

# שמירת וטעינת הגדרות משתמש
def load_config():
    with open('app/config.pkl', 'rb') as f:
        user_config = pickle.load(f)
        return user_config
    
# שמירת הגדרות משתמש לקובץ
def save_config(e, copy_mode, main_folder_only, singles_folder, exist_only, abc_sort):

    user_config = load_config()
    
    user_config['general']['copy_mode'] = copy_mode
    user_config['general']['main_folder_only'] = main_folder_only
    user_config['general']['singles_folder'] = singles_folder
    user_config['general']['exist_only'] = exist_only
    user_config['general']['abc_sort'] = abc_sort
    
    with open('app/config.pkl', 'wb') as f:
        pickle.dump(user_config, f)




# ישום עדכון וזיהוי גרסה חדשה
def check_for_update(current_version):
    # URL של קובץ הגרסה באתר
    url = "https://nhlocal.github.io/Singles-Sorter/versions.data/new-ver-exist"
    
    try:
        # בקשה לקובץ הגרסה באתר
        response = requests.get(url)
        response.raise_for_status()  # לבדוק אם התקבלה תשובה תקינה
        latest_version = response.text.strip()  # להוריד רווחים מיותרים
        
        # להשוות בין גרסה נוכחית לגרסה באתר
        if latest_version > current_version:
            return latest_version
        else:
            return False
        
    except requests.RequestException as e:
        print(f"Error fetching version data: {e}")
        return False
    
if __name__ == '__main__':


    from singles_sorter_v3 import MusicSorter

    # גרסה נוכחית מהתוכנה שלך
    current_version = MusicSorter.VERSION

    # קריאה לפונקציה
    new_version = check_for_update(current_version)
    if new_version:
        print(f"גרסה חדשה זמינה: {new_version} 🚀")
    else:
        print(f"אין גרסה חדשה זמינה. 🙁")
