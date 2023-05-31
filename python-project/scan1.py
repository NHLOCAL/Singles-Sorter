import csv
import os.path

def artist_from_song(my_file):
    
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
            

"""
הגדרת תוים ספציפיים לפני ואחרי שם הזמר לשיפור הדיוק
"""