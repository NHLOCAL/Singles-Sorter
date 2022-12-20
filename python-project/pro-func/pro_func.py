# -*- coding: utf-8 -*-
import os

import sys

# פונקציה להעתקת והעברת קבצים
import shutil

# יבוא פונקציה לקריאת מטאדאטה של קובץ
import music_tag

# יבוא פונקציה לקריאת עץ תיקיות
from os.path import join, getsize

# יבוא פונקציה עבור תצוגת האותיות העבריות
from bidi.algorithm import get_display

# יבוא פונקציה להמרת ג'יבריש לעברית תקינה
from jibrish_to_hebrew import jibrish_to_hebrew

# יבוא פונקציה לזיהוי דמיון בין מחרוזות
from identify_similarities import true_or_false

def artist_from_song(my_file, root):
    """
פונקציית סורקת את המטאדאטה של השיר ומכניסה את האמן למשתנה
    
תנאים:
    my_file (str) - שם הקובץ שנסרק
    root (str) - נתיב התיקייה האב
    
תוצאה:
    ערך המכיל את שם הקובץ, וערך המכיל את שם האמן שלו
    """
    try:
        # יצירת נתיב מלא לקובץ
        my_file = root + "\\" + my_file
        # בדיקה האם הקובץ הוא קובץ MP3
        if not (my_file.endswith(".mp3") or my_file.endswith(".wav") or my_file.endswith(".wma")):
            return
        # טעינת מטאדאטה של השיר
        artist_file = music_tag.load_file(my_file)
        # קבלת אמן מטאדאטה של השיר
        artist = artist_file['artist']
        # הכנסת נתוני האמן למשתנה הגלובלי
        if artist:
            return my_file, artist
    except:
        pass



def scan_dir(dir_path, target_dir=None, copy_mode=False):
    """
הפונקציה המרכיבת את הפקודה הראשית של התכנית. היא סורקת את התיקיות והקבצים תחת נתיב שצוין ויוצרת רשימה של קבצים להעתקה.
בסוף התהליך היא מעתיקה אותם אם הוכנס פרמטר של תיקית יעד.
    
תנאים:
    פרמטר 1 = נתיב תיקיה לסריקה
    פרמטר 2 = נתיב תיקית יעד להעברה אליה (אופציונלי)
    פרמטר 3 = הפעלת מצב העתקה (ברירת המחדל היא העברה).
מוגדר על ידי Trte או False.
    
תוצאה:
    מדפיס את רשימת האמנים שמופיעים במטאדאטה של השירים, ומעתיק אותם ליעד.
    """
    
    # הגדרת רשימת מחרוזות יוצאות דופן, עליהם המערכת מוגדרת לדלג
    unusual_list = ["סינגלים", "אבגדהוזחט", "סינגל"]
    
    # יצירת רשימה ריקה להכנסת מידע על הקבצים
    info_list = []
    
    # מעבר על עץ תיקיות והפעלת פונקציה לבדיקת שם אמן הקובץ
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                info_file = artist_from_song(my_file, root)
                if info_file:
                    info_list.append(info_file)
                    
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist_item in info_list:
        artist = artist_item.value
        
        # חזרה לתחילת הלולאה אם שם האמן קיים ברשימת יוצאי הדופן
        # או אם הוא דומה לפריט כלשהו ברשימת יוצאי הדופן
        if artist in unusual_list or true_or_false(artist, unusual_list):
            print(artist + "not good!")
            continue
            
        elif len(artist.split()) >= 4 or len(artist.split()) <= 0:
            continue
            
        elif any(c in "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ" for c in artist):
            artist = jibrish_to_hebrew(artist)

        if all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳ " for c in artist):
            if target_dir == None:
                print(file_path + " == " + artist)
            else:
                target_path = target_dir + "\\" + artist
                
                # יצירת תיקית יעד אם אינה קיימת
                if not os.path.isdir(target_path):
                    os.makedirs(target_path)
                if copy_mode:
                    # העתקת הקובץ לתיקית האמן התואמת
                    shutil.copyfile(file_path, target_path)
                else:
                    # העברת הקובץ לתיקית האמן התואמת
                    shutil.move(file_path, target_path)


def main():
    # קבלת נתיב משתנה
    dir_path = str(sys.argv[1])
    if sys.argv[2:]:
        target_dir = str(sys.argv[2])
        # הפעלת פונקצית סריקת הקבצים עם שתי פרמטרים
        scan_dir(dir_path, target_dir)
    else:
        # הפעלת פונקצית סריקת הקבצים עם פרמטר יחיד
        scan_dir(dir_path)
    

    
    
    os.system('pause')

if __name__ == '__main__':
    main()