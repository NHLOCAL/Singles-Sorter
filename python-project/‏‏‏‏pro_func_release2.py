# -*- coding: utf-8 -*-
import os
from sys import argv
# פונקציה להעתקת והעברת קבצים
from shutil import copy, move
# יבוא פונקציה לקריאת מטאדאטה של קובץ
from music_tag import load_file
# יבוא פונקציה לקריאת עץ תיקיות
from os.path import join, getsize
# יבוא פונקציה להמרת ג'יבריש לעברית תקינה
from jibrish_to_hebrew import jibrish_to_hebrew
# יבוא פונקציה לזיהוי דמיון בין מחרוזות
from identify_similarities import similarity_sure
# פונקציה לקריאת קבצי csv
import csv


# מעבר על עץ התיקיות שהוגדר
def scan_dir(dir_path, target_dir=None, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, tree_folders=False):
    """
הפונקציה המרכיבת את הפקודה הראשית של התכנית. היא סורקת את התיקיות והקבצים תחת נתיב שצוין ויוצרת רשימה של קבצים להעתקה.
בסוף התהליך היא מעתיקה אותם אם הוכנס פרמטר של תיקית יעד.
    
תנאים:
    פרמטר 1 = נתיב תיקיה לסריקה
    פרמטר 2 = נתיב תיקית יעד להעברה אליה (אופציונלי)
    פרמטר 3 = הפעלת מצב העתקה (ברירת המחדל היא העברה)
    פרמטר 4 = מיון בתיקיות לפי א' ב'
    פרמטר 5 = העברה לתיקיות קיימות בלבד
    פרמטר 6 = יצירת תיקית "סינגלים" פנימית
    פרמטר 7 = מיון תיקיה ראשית בלבד/עץ תיקיות
מוגדר על ידי True או False.
    
תוצאה:
    מדפיס את רשימת האמנים שמופיעים במטאדאטה של השירים, ומעתיק אותם ליעד.
    """
    # סריקת עץ התיקיות או התיקיה הראשית בהתאם לבחירת המשתמש והכנסת שם הקבצים ושם האמן שלהם לרשימה
    info_list = []  
    if tree_folders is False:
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                file_path = os.path.join(root, my_file)
                if my_file.endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

    # סריקת התיקיה הראשית בלבד ללא תיקיות פנימיות
    elif tree_folders:
        for my_file in os.listdir(dir_path):
            file_path = os.path.join(dir_path, my_file)
            if os.path.isfile(file_path):
                if my_file.endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

        
    # הגדרות עבור תצוגת אחוזים
    len_dir = len(info_list)
    len_item = 0
    
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist in info_list:   
        # תצוגת אחוזים מתחלפת
        len_item += 1
        show_len = len_item * 100 // len_dir
        print("" * 39, str(show_len), "% ", "הושלמו",end='\r')
        
        # הפעלת פונקציה המבצעת בדיקות על שם האמן
        check_answer = check_artist(artist)
        if check_answer == False:
            continue
                       
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
    csv_path = r"F:\גיבויים\מסמכים\GitHub\Singles-Sorter\source_code\singer-list.csv"
    with open(csv_path, 'r') as file:
        csv_reader = csv.reader(file)
        singer_list = [tuple(row) for row in csv_reader]
    
    # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בשם השיר
    for source_name, target_name in singer_list:
        if source_name in split_file:
            artist = target_name
            return artist



    # אם שם הקובץ לא נמצא יתבצע חיפוש במטאדאטה של הקובץ
    try:
        # בדיקה האם הקובץ הוא קובץ שמע
        if not my_file.endswith((".mp3",".wma", ".wav")):
            return
        # טעינת מטאדאטה של השיר
        artist_file = load_file(my_file)
        # קבלת אמן מטאדאטה של השיר
        artist = artist_file['artist']
        artist = artist.value
        # הכנסת נתוני האמן למשתנה הגלובלי
        if artist:
            # המרת שם האמן אם הוא פגום
            if any(c in "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ" for c in artist):
                artist = jibrish_to_hebrew(artist)               
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
    try:
        dir_path = os.path.join(argv[1]) # נתיב תיקית מקור
        target_dir = os.path.join(argv[2]) # נתיב תיקית יעד  
        copy_mode = True if str(argv[3]) == "xcopy" else False  # קביעת העתקה או העברה  
        abc_sort = True if eval(argv[4]) else False # מיון לפי א' ב'
        exist_only = True if eval(argv[5]) else False # העברה לתיקיות קיימות בלבד
        # הוספת תיקית סינגלים פנימית
        singles_folder = True if eval(argv[6]) else False
        # מעבר על עץ תיקיות
        tree_folders = False if eval(argv[7]) else True

        # הרצת הפונצקיה עם כל הפרמטרים
        scan_dir(str(argv[1]), str(argv[2]), copy_mode, abc_sort, exist_only, singles_folder, tree_folders)
    except:
        print("""מסדר הסינגלים - סריקת קבצי מוזיקה ומיון שלהם לפי אמנים.
    
תנאים:
    פרמטר 1 = נתיב תיקיה לסריקה
    פרמטר 2 = נתיב תיקית יעד להעברה אליה (אופציונלי)
    פרמטר 3 = הפעלת מצב העתקה (ברירת המחדל היא העברה)
    פרמטר 4 = מיון בתיקיות לפי א' ב'
    פרמטר 5 = העברה לתיקיות קיימות בלבד
    פרמטר 6 = יצירת תיקית "סינגלים" פנימית
    פרמטר 7 = מיון תיקיה ראשית בלבד/עץ תיקיות
מוגדר על ידי True או False.
    
תוצאה:
    מדפיס את רשימת האמנים שמופיעים במטאדאטה של השירים, ומעתיק אותם ליעד.
""")
    
    
if __name__ == '__main__':
    main()