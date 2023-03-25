'''
1. קבלת תוכן קובץ CSV והכנסתו לרשימה.
2. מיזוג תיקיות זמרים לפי הרשימה.
3. מעבר על התיקיות שנותרו עם פונקציית זיהוי דמיונות.
4. לתת רשימה של כל השמות הדומים ולאפשר למשתמש לבחור מה באמת דומה ומה לא.
(יש לשים לב שאין את אותם השמות פעמיים)
5. לקבץ שמות דומים לטאפלים של שתיים או יותר,
ולשאול את המשתמש באיזה שם ברירת מחדל לבחור.
6. ליצור רשימה עם על אחד מהשמות שבטאפלים עם השם שנבחר.
7. להכניס את התוכן הזה לקובץ CSV חדש (בתנאי ששם היעד מכיל 2 מילים או יותר).
8. לבצע מיזוג תיקיות לפי הרשימה.
'''

import csv, os, sys, shutil
# יבוא פונקציה לזיהוי דמיון בין מחרוזות
from identify_similarities import similarity_sure
# פונקציית ניקוי מסך
from click import clear


# יבוא רשימת זמרים מקובץ csv
def read_csv(file_path):
    """
קריאת תוכן קובץ CSV והכנסה לרשימה
    פרמטר = נתיב קובץ
    תוצאה = רשימת נתונים
    """
    with open(file_path, 'r') as file:
        csv_reader = csv.reader(file)
        singers_data = [tuple(row) for row in csv_reader]
    return singers_data


# מבצע מיזוג של תיקיות זמרים בעלי שם שונה במקצת
def merge_folders(singers_data, dir_path):
    """
מבצע מיזוג של תיקיות זמרים בעלי שם שונה במקצת
    פרמטר 1 = רשימת זמרים
    פרמטר 2 = נתיב תיקיה לסריקה
    תוצאה = אין
    """
    # הגדרת התיקיה שהוכנסה כתיקיה הנוכחית
    os.chdir(dir_path)
    
    # מעבר על רשימת שמות הזמרים וחיפוש שלהם בתיקיה
    for source_name, target_name in singers_data:
        # אם שם תיקית המקור והיעד זהים תתבצע חזרה להמשך הלולאה
        if source_name == target_name: continue
        
        # בדיקה אם שם האמן קיים ברשימת התיקיות
        if not source_name in os.listdir(): continue
        
        # הגדרת נתיבי תיקית מקור ותיקית יעד
        old_path = os.path.join(os.getcwd(), source_name)
        new_path = os.path.join(os.getcwd(), target_name)

        # אם לא קיים נתיב יעד יתבצע שינוי שם לשם הרצוי
        if os.path.exists(old_path) and not os.path.exists(new_path):
            os.rename(old_path, new_path)
            print(old_path + ' --> \n' + new_path)
       
        # אם קיים נתיב יעד, תתבצע העברה של הקבצים שבמקור אל היעד
        elif os.path.exists(old_path):
            for filename in os.listdir(old_path):
                source_path = os.path.join(old_path, filename)
                destination_path = os.path.join(new_path, filename)
                shutil.move(source_path, destination_path)
            print(old_path + ' --> \n' + new_path)
            # מחיקת תיקית המקור לאחר סיום ההעברה
            shutil.rmtree(old_path)


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



# יצירת רשימת שמות דומים לפי בחירת המשתמש
def creat_similarity_list(dir_path):
    '''
יוצר רשימה של זמרים דומים לפי בחירות המשתמש
    
    פרמטר = נתיב תיקית זמרים
    
    תוצאה = רשימה המכילה קבוצות זמרים לפי דמיון
    '''
    # הגדרת סט שמות אמנים דומים    
    similarity_set = set()
    not_similarity_set = set()
    # יצירת רשימת תיקיות תוך התעלמות מקבצים
    folders_list = [item for item in os.listdir(dir_path) if os.path.isdir(os.path.join(dir_path, item))]

    for artist in folders_list:
        # הפעלת בדיקה אם שם אמן דומה כבר קיים ביעד
        similarity_str = check_similarity(dir_path, artist)
        set_item = (artist, similarity_str)
          
        # בדיקה אם השם כבר מופיע ברשימת הדומים / הלא דומים
        if similarity_str and not set_item in similarity_set \
            and not set_item in not_similarity_set:
            # מתן אפשרות למשתמש לבחור אם למזג את שמות הזמרים
            print('{}\n"{}" {} "{}"\n{}'.format("נמצאו שמות דומים - למזג?", artist, "<-->", similarity_str, "הקש 1 לאישור או 2 להמשך"))
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
    
    # מיזוג זוגות שמות דומים לקבוצות
    merges_list = merge_tuples(similarity_set)
    
    return merges_list


# מיזוג טאפלים המכילים שמות זמרים זהים
def merge_tuples(input_set):
    '''
ממזג טאפלים המכילים מחרוזת אחת זהה לרשימה חדשה המכילה טאפלים

    פרמטר = סט המכיל טאפלים בני 2 מחרוזות
    
    תוצאה = רשימה עם סטים ממוזגים    
    '''
    result = list(input_set)
    i = 0
    while i < len(result):
        v = result[i]
        for j, tup in enumerate(result):
            if v == tup:
                continue
            if not set(v).isdisjoint(set(tup)):
                result[j] = set(v).union(set(tup))
                result.pop(i)
                break
        else:
            i += 1

    return result



def main():
    dir_path = str(sys.argv[1])
    file_path = r"C:\Users\COLMI\AppData\Roaming\singles-sorter\singer-list.csv"
    singers_data = read_csv(file_path)
    merge_folders(singers_data, dir_path)
    creat_similarity_list(dir_path)

if __name__ == '__main__':
    main()