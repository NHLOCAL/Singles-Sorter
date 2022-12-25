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

        # בדיקה האם הקובץ הוא קובץ MP3
        if not (my_file.endswith(".mp3") or my_file.endswith(".wav") or my_file.endswith(".wma")):
            return
        # טעינת מטאדאטה של השיר
        artist_file = music_tag.load_file(my_file)
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
    
    # יצירת רשימה ריקה להכנסת מידע על הקבצים
    info_list = []
    
    # מעבר על עץ תיקיות והפעלת פונקציה לבדיקת שם אמן הקובץ
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                # יצירת נתיב מלא לקובץ
                my_file = root + "\\" + my_file
                # הפעלת פונקציה לקבלת שם אמן
                info_file = artist_from_song(my_file)
                if info_file:
                    info_list.append((my_file, info_file))
    else:
        return
        
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist in info_list:
        
        # הפעלת פונקציה המבצעת בדיקות על שם האמן
        check_answer = check_artist(artist)
        if check_answer == False:
            continue
            
        if target_dir == None:
            print(file_path + " == " + artist)
            
        else:  
            # הפעלת בדיקה אם שם אמן דומה כבר קיים ביעד
            similarity_str = check_similarity(target_dir, artist)
            if similarity_str:                   
                # מתן אפשרות למשתמש לבחור אם למזג את שמות הזמרים
                print('{}\n"{}" {} "{}"\n{}'.format("נמצאו שמות דומים - למזג?", artist, "-->", similarity_str, "הקש 1 לאישור או 2 להמשך"))
                answer = input(">>>")
                try:
                    if int(answer) == 1:
                        artist = similarity_str
                except:
                    pass   
                
            # יצירת תיקית יעד אם אינה קיימת
            target_path = target_dir + "\\" + artist
            if not os.path.isdir(target_path):
                os.makedirs(target_path)
            if copy_mode:
                # העתקת הקובץ לתיקית האמן התואמת
                shutil.copy(file_path, target_path)
            else:
                # העברת הקובץ לתיקית האמן התואמת
                try:
                    shutil.move(file_path, target_path)
                except:
                    pass
                        

def main():
    # קבלת נתיב משתנה
    dir_path = str(sys.argv[1])
    if sys.argv[3:]:
        copy_mode=bool(sys.argv[3])
        target_dir = str(sys.argv[2])
        # הפעלת פונקצית סריקת קבצים עם שלוש פרמטרים
        scan_dir(dir_path, target_dir, copy_mode)
    elif sys.argv[2:]:
        target_dir = str(sys.argv[2])
        # הפעלת פונקצית סריקת הקבצים עם שתי פרמטרים
        scan_dir(dir_path, target_dir)
    else:
        # הפעלת פונקצית סריקת הקבצים עם פרמטר יחיד
        scan_dir(dir_path)

    os.system('pause')

if __name__ == '__main__':
    main()