# -*- coding: utf-8 -*-
import os
import sys

# יבוא פונקציה לקריאת מטאדאטה של קובץ
import music_tag

# יבוא פונקציה לקריאת עץ תיקיות
from os.path import join, getsize

# יבוא פונקציה עבור תצוגת האותיות העבריות
from bidi.algorithm import get_display

from jibrish_to_hebrew import jibrish_to_hebrew


def artist_from_song(my_file, root):
    """
    פונקציית סורקת את המטאדאטה של השיר ומכניסה את האמן למשתנה
    
    תנאים:
    my_file (str) - שם הקובץ שנסרק
    root (str) - נתיב התיקייה האב
    
    תוצאה:
    מכניס את האמן שנמצא במטאדאטה של השיר למשתנה target_dict
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
            target_dict[my_file] = artist
    except:
        pass


def main():
    """
    הפונקציה המרכיבת את הפקודה הראשית של התכנית. היא סורקת את התיקיות והקבצים תחת נתיב שצוין ומכניסה את האמנים שמופיעים במטאדאטה של השירים למשתנה גלובלי
    
    תנאים:
    אין
    
    תוצאה:
    מכניס את האמנים שמופיעים במטאדאטה של השירים למשתנה גלובלי
    """
    # הגדרת משתנה גלובלי למעט את התגיות הראשיות
    global target_dict
    target_dict = {}
    # קבלת נתיב משתנה
    dir_path = str(sys.argv[1])
    try:
        target_dir = str(sys.argv[2])
    except:
        pass

    # מעבר על עץ תיקיות והפעלת פונקציה לבדיקת שם אמן הקובץ
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                artist_from_song(my_file, root)
     
    # המרת מילון המכיל את תוצאות הסריקה לרשימה
    dict_list = target_dict.items()
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_name, artist_item in dict_list:
        artist = artist_item.value
        if len(artist.split()) >= 4:
            continue
        elif any(c in "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ" for c in artist):
            artist = jibrish_to_hebrew(artist)
            
        if all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ' " for c in artist):
            try:
                os.system("md " + '"' + target_dir + "\\" + artist +'"')
                os.system('move "' + file_name + '" "' +  target_dir + "\\" + artist + '"')
            except:
                print("move " + file_name + " to " + artist)

if __name__ == '__main__':
    main()
    os.system('pause')
