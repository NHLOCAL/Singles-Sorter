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

import csv, os, shutil, identify_similarities

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


def merge_folders(singers_data):
    """
מבצע מיזוג של תיקיות זמרים בעלי שם שונה במקצת
    פרמטר = רשימת זמרים
    תוצאה = אין   
    """
    for source_name, target_name in singers_data:   
        # אם שם תיקית המקור והיעד זהים תתבצע חזרה להמשך הלולאה
        if source_name == target_name: continue

        old_path = os.path.join(os.getcwd(), source_name)
        new_path = os.path.join(os.getcwd(), target_name)
               
        # אם לא קיים נתיב יעד יתבצע שינוי שם לשם הרצוי
        if os.path.exists(old_path) and not os.path.exists(new_path):
            os.rename(old_path, new_path)
       
        # אם קיים נתיב יעד, תתבצע העברה של הקבצים שבמקור אל היעד
        elif os.path.exists(old_path):       
            for filename in os.listdir(old_path):
                source_path = os.path.join(old_path, filename)
                destination_path = os.path.join(new_path, filename)
                shutil.move(source_path, destination_path)
            # מחיקת תיקית המקור לאחר סיום ההעברה
            shutil.rmtree(old_path)


def main():
    file_path = r"C:\Users\COLMI\AppData\Roaming\singles-sorter\singer-list.csv"
    singers_data = read_csv(file_path)
    merge_folders(singers_data)

if __name__ == '__main__':
    main()