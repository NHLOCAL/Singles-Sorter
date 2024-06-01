# -*- coding: utf-8 -*-
import os
import argparse
# פונקציה להעתקת והעברת קבצים
from shutil import copy, move
# יבוא פונקציה לקריאת מטאדאטה של קובץ
from music_tag import load_file
# יבוא פונקציה להמרת ג'יבריש לעברית תקינה
from jibrish_to_hebrew import fix_jibrish
# יבוא פונקציה לזיהוי דמיון בין מחרוזות
from identify_similarities import similarity_sure
# פונקציה לקריאת קבצי csv
import csv
# פונקציה לבדיקת דיוק שם האמן במחרוזת
from check_name import check_exact_name


# גרסת התוכנה
global VERSION
VERSION = '12.9.1'

# הגדרות עבור תצוגת אחוזים
def progress_display(len_amount):
    for len_item in range(1, len_amount + 1):
        show_len = len_item * 100 // len_amount
        yield show_len


# בדיקת שגיאות מתקדמת
def check_errors(source_dir, target_dir):
    """
    Checks for potential errors related to source and target directories.

    Args:
        source_dir (str): The path to the source directory.
        target_dir (str): The path to the target directory.

    Raises:
        FileNotFoundError: If the source or target directory does not exist.
        PermissionError: If the script does not have write access to the target directory.
        ValueError: If the source and target directories are the same.
    """
    if not os.path.exists(source_dir):
        raise FileNotFoundError("תיקיית המקור לא נמצאה")

    if not os.path.exists(target_dir):
        raise FileNotFoundError("תיקיית היעד לא נמצאה")

    if not os.access(target_dir, os.W_OK):
        raise PermissionError("אין הרשאת כתיבה לתיקיית היעד")

    if os.path.samefile(source_dir, target_dir):
        raise ValueError("תיקיית המקור ותיקיית היעד לא יכולות להיות זהות")
    
    if not os.listdir(source_dir):
        raise ValueError("תיקיית המקור ריקה")


# הסרת תוכן מיותר משמות הקבצים
def clean_names(dir_path):
    pass








# מעבר על עץ התיקיות שהוגדר
def scan_dir(dir_path, target_dir, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, main_folder_only=False, progress_callback=None):
    """
    Main function of the program. Scans the specified directory and creates a list of files for copying.
    At the end of the process, it copies them if a target directory parameter is provided.
    
    Parameters:
        dir_path = Directory path to scan
        target_dir = Target directory path for transfer
        copy_mode = Enable copy mode (default is move)
        abc_sort = Sort folders alphabetically
        exist_only = Transfer to existing folders only
        singles_folder = Create an internal "singles" folder
        main_folder_only = Sort only the main folder
        Defined by True or False.
    
    Result:
        Prints the list of artists that appear in the song metadata and copies them to the target.
    """

    # בדיקת שגיאות בארגומנטים של המשתמש
    check_errors(dir_path, target_dir)


    # סריקת עץ התיקיות או התיקיה הראשית בהתאם לבחירת המשתמש והכנסת שם הקבצים ושם האמן שלהם לרשימה
    info_list = []  
    if main_folder_only is False:
        for root, _, files in os.walk(dir_path):
            for my_file in files:
                file_path = os.path.join(root, my_file)
                if my_file.lower().endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

    # סריקת התיקיה הראשית בלבד ללא תיקיות פנימיות
    elif main_folder_only:
        for my_file in os.listdir(dir_path):
            file_path = os.path.join(dir_path, my_file)
            if os.path.isfile(file_path):
                if my_file.lower().endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

    len_dir = len(info_list)
    progress_generator = progress_display(len_dir)

    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist in info_list:   
        show_len = next(progress_generator)
        print(" " * 30, str(show_len), "% ", "הושלמו",end='\r')
        if progress_callback:  # Call the callback with progress
            progress_callback(show_len)
                       
        # הגדרת משתנה עבור תיקית יעד בהתאם להתאמות האישיות של המשתמש
        if singles_folder and abc_sort:
            main_target_path = os.path.join(target_dir, artist[0], artist)
            target_path = os.path.join(target_dir, artist[0], artist, "סינגלים")
        elif singles_folder:
            main_target_path = os.path.join(target_dir, artist)
            target_path = os.path.join(target_dir, artist, "סינגלים")
        elif abc_sort:
            main_target_path = os.path.join(target_dir, artist[0], artist)
            target_path = os.path.join(target_dir, artist[0], artist)
        else:
            main_target_path = os.path.join(target_dir, artist)
            target_path = os.path.join(target_dir, artist)
        
        # יצירת תיקית יעד בתנאים מסויימים
        if exist_only is False:
            if not os.path.isdir(target_path):
                try: os.makedirs(target_path)
                except: pass
                
        elif exist_only and singles_folder:
            if os.path.isdir(main_target_path) and not os.path.isdir(target_path):
                try: os.makedirs(target_path)
                except: pass
        else:
            pass #לא תיווצר תיקיה חדשה


        # העברה או העתקה בהתאם להגדרות המשתמש
        if copy_mode and os.path.isdir(target_path):
            try: copy(file_path, target_path)
            except: pass
        elif os.path.isdir(target_path):
            try: move(file_path, target_path)
            except: pass

    return True, None

 
def artist_from_song(my_file):
    """
הפונקציה בודקת את שם אמן הקובץ בשם הקובץ לפי מסד נתונים ומכניסה את שם האמן למשתנה
אם השם לא קיים היא סורקת את המטאדאטה של השיר ומכניסה את שם האמן למשתנה
    
תנאים:
    my_file (str) - שם הקובץ שנסרק
    
תוצאה:
    ערך המכיל את שם אמן הקובץ
    """
 
    # מעבר על רשימת הזמרים בדאטה וחיפוש שלהם בשם הקובץ
    
    # קבלת שם הקובץ ללא נתיב מלא
    split_file = os.path.split(my_file)[1]
    
    # הסרת תווים מטעים בשם הקובץ
    split_file = split_file.replace('_', ' ')
    split_file = split_file.replace('-', ' ')
    
    # יבוא רשימת זמרים מקובץ csv
    if not 'singer_list' in globals():
        # Get the directory of the script file
        script_dir = os.path.dirname(os.path.abspath(__file__))

        # Construct the path to the CSV file
        csv_path = os.path.abspath("singer-list.csv")
        
        global singer_list
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]
        
        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)
    
    # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בשם השיר
    for source_name, target_name in singer_list:
        if source_name in split_file:
        
            # בדיקת דיוק שם הקובץ
            exact = check_exact_name(split_file, source_name)
            
            if exact:
                artist = target_name
                return artist

    # אם שם הקובץ לא נמצא יתבצע חיפוש במטאדאטה של הקובץ
    try:
        # טעינת מטאדאטה של השיר
        metadata_file = load_file(my_file)
        # קבלת אמן מטאדאטה של השיר
        artist = metadata_file['artist']
        artist = artist.value

        if artist:
            # המרת שם האמן אם הוא פגום
            artist = fix_jibrish(artist, "heb")
            
            # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בתגית האמן
            for source_name, target_name in singer_list:
                if source_name in artist:
                    """
                    # בדיקת דיוק שם הקובץ
                    exact = check_exact_name(artist, source_name)  
                    """
                    artist = target_name
                    return artist
                
            # הפעלת פונקציה המבצעת בדיקות על שם האמן
            check_answer = check_artist(artist)
            if check_answer == False:
                return
                
            return artist
        
        # אם לא נמצא שם אמן תקין יתבצע חיפוש בכותרת הקובץ
        else:
            # קבלת כותרת השיר
            title = metadata_file['title']
            title = title.value
            
            if title:
                # המרת שם האמן אם הוא פגום
                title = fix_jibrish(title, "heb")
                
                # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בתגית האמן
                for source_name, target_name in singer_list:
                    if source_name in title:
                        # בדיקת דיוק שם הקובץ
                        exact = check_exact_name(title, source_name)
                        
                        if exact:
                            artist = target_name
                            return artist
    except:
        return


# בדיקות שונות על שם האמן
def check_artist(artist):
    # הגדרת רשימת מחרוזות יוצאות דופן, עליהם המערכת מוגדרת לדלג
    unusual_list = ["סינגלים", "סינגל", "אבגדהוזחטיכלמנסעפצקרשתךםןץ", "אמן לא ידוע", "טוב", "לא ידוע", "תודה לך ה"]
    # החזרת שקר אם שם האמן קיים ברשימת יוצאי הדופן
    # או אם הוא דומה לפריט כלשהו ברשימת יוצאי הדופן       
    unusual_str = similarity_sure(artist, unusual_list, True)        
    if artist in unusual_list or unusual_str[0]:
        return False
       
    # בדיקה אם המחרוזת אינה ארוכה מידי
    if len(artist.split()) >= 4 or len(artist.split()) <= 0:
        return False
    
    # בדיקה אם המחרוזת מכילה תוים תקינים בלבד
    if all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳ " for c in artist):
        return True
    else:
        return False


def main():
    parser = argparse.ArgumentParser(description=f"Singles Sorter {VERSION} - Scan and organize music files into folders by artist using advanced automation.")
    parser.add_argument('dir_path', help="Path to the source directory")
    parser.add_argument('target_dir', help="Path to the target directory", nargs='?')

    parser.add_argument('--copy_mode', help="Enable copy mode (default is move mode)", action='store_true')
    parser.add_argument('--abc_sort', help="Sort folders alphabetically", action='store_true')
    parser.add_argument('--exist_only', help="Transfer to existing folders only", action='store_true')
    parser.add_argument('--no_singles_folder', help="Do not create an internal 'singles' folder", action='store_false', dest='singles_folder', default=True)
    parser.add_argument('--main_folder_only', help="Sort only the main folder (default: False)", action='store_true')

    args = parser.parse_args()

    try:
        dir_path = os.path.join(args.dir_path) # Source directory path
        target_dir = os.path.join(args.target_dir) # Target directory path
        copy_mode = args.copy_mode  # Set copy mode
        abc_sort = args.abc_sort  # Alphabetical sorting
        exist_only = args.exist_only  # Transfer to existing folders only        
        singles_folder = args.singles_folder  # Internal singles folder
        main_folder_only = args.main_folder_only  # Main folder only

        # Run the clean names function
        clean_names(dir_path)
    
        # Run the scan directory function with all parameters
        scan_dir(dir_path, target_dir, copy_mode, abc_sort, exist_only, singles_folder, main_folder_only)
    except Exception as e:
        print("Error: {}".format(e))
    
    
if __name__ == '__main__':
    main()