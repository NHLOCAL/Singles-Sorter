# -*- coding: utf-8 -*-
import os
from sys import argv
# פונקציית ניקוי מסך
from click import clear
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

def artist_from_song(my_file):
    """
פונקציית סורקת את המטאדאטה של השיר ומכניסה את האמן למשתנה
    
תנאים:
    my_file (str) - שם הקובץ שנסרק
    root (str) - נתיב התיקייה האב
    
תוצאה:
    ערך המכיל את שם אמן הקובץ
    """
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
 

# בדיקה אם שם האמן קיים כבר בצורה דומה
def check_similarity(target_dir, artist):
    """
בדיקה אם שם אמן קיים ברשימת תיקיות

פרמטרים:
    פרמטר 1 = נתיב תיקיה
    פרמטר 2 = שם אמן

תוצאה:
    שם האמן הדומה או "None"
    """
    list_dirs = os.listdir(target_dir)
    # יציאה מהפונקציה במקרה ורשימת הקבצים ריקה
    if list_dirs == []:
        return None
    # בדיקת דמיון בין מחרוזות כדי לבדוק אם קיים שם אמן דומה בתיקית היעד
    answer, similarity_str = similarity_sure(artist, list_dirs, False)
    if answer:
        return similarity_str
    else:
        return None


# מעבר על עץ התיקיות שהוגדר
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
    # סריקת עץ התיקיות והכנסת שם הקבצים ושם האמן שלהם לרשימה
    if (dir_path != "") and (os.path.exists(dir_path)):
        info_list = [(os.path.join(root, my_file), artist_from_song(os.path.join(root, my_file)))
            for root, dirs, files in os.walk(dir_path)
            for my_file in files if artist_from_song(os.path.join(root, my_file))]
    else:
        return
        
    # הגדרות עבור תצוגת אחוזים
    len_dir = len(info_list)
    len_item = 0

    # הגדרת סט שמות אמנים דומים    
    similarity_set = set()
    not_similarity_set = set()
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist in info_list:   
        # תצוגת אחוזים מתחלפת
        len_item += 1
        show_len = len_item * 100 // len_dir
        print(str(show_len) + "% " + "הושלמו",end='\r')
        # הפעלת פונקציה המבצעת בדיקות על שם האמן
        check_answer = check_artist(artist)
        if check_answer == False:
            continue

        if target_dir == None:
            print(file_path + " == " + artist)
            continue
              
        # הפעלת בדיקה אם שם אמן דומה כבר קיים ביעד
        similarity_str = check_similarity(target_dir, artist)
        set_item = (artist, similarity_str)
        if similarity_str and not set_item in similarity_set \
            and not set_item in not_similarity_set:
            # מתן אפשרות למשתמש לבחור אם למזג את שמות הזמרים
            print('{}\n"{}" {} "{}"\n{}'.format("נמצאו שמות דומים - למזג?", artist, "-->", similarity_str, "הקש 1 לאישור או 2 להמשך"))
            answer = input(">>>")
            # ניקוי מסך
            clear()
            try:
                if int(answer) == 1:
                    similarity_set.add(set_item)
                    artist = similarity_str
                elif int(answer) == 2:
                    not_similarity_set.add(set_item)
            except:
                pass

        elif (artist, similarity_str) in similarity_set:
            artist = similarity_str
                       
        # יצירת תיקית יעד אם אינה קיימת        
        target_path =  os.path.join(target_dir,artist)
        if not os.path.isdir(target_path):
            os.makedirs(target_path)
        if copy_mode:
            # העתקת הקובץ לתיקית האמן התואמת
            copy(file_path, target_path)
        else:
            # העברת הקובץ לתיקית האמן התואמת
            try:
                move(file_path, target_path)
            except:
                pass

def main():
    # קבלת נתיב משתנה
    dir_path = str(argv[1])
    if argv[3:]:
        copy_mode=eval(argv[3])
        target_dir = str(argv[2])
        # הפעלת פונקצית סריקת קבצים עם שלוש פרמטרים
        scan_dir(dir_path, target_dir, copy_mode)
    elif argv[2:]:
        target_dir = str(argv[2])
        # הפעלת פונקצית סריקת הקבצים עם שתי פרמטרים
        scan_dir(dir_path, target_dir)
    else:
        # הפעלת פונקצית סריקת הקבצים עם פרמטר יחיד
        scan_dir(dir_path)

    os.system('pause')

if __name__ == '__main__':
    main()