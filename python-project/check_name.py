import os
import re
import csv

def artist_from_song(my_file):
    """
    הפונקציה בודקת את שם האמן בשם הקובץ על סמך מסד נתונים ומאחסנת את שם האמן במשתנה.
        אם השם לא קיים, הוא סורק את המטא נתונים של השיר ומאחסן את שם האמן במשתנה.

        פרמטר:
            my_file (str): שם הקובץ שיש לסרוק.

        החזרות:
            str: הערך המכיל את שם האמן מהקובץ.
    """
      
    # קבל את שם הקובץ ללא הנתיב המלא
    split_file = os.path.split(my_file)[1]
    split_file = os.path.splitext(split_file)[0]

    # הסר תווים לא רצויים משם הקובץ
    split_file = re.sub(r'[_-]', ' ', split_file)


    # ייבא את רשימת הזמרים מקובץ csv
    if 'singer_list' not in globals():
        csv_path = "singer-list.csv"
        global singer_list
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]

        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)

    # חזור על רשימת השמות ובדוק אם אחד מהם קיים בשם הקובץ
    for source_name, target_name in singer_list:
        if source_name in split_file:
            artist = target_name
            
            # בדיקת דיוק שם הקובץ
            exact = check_exact_name(split_file, source_name)
            if exact: return artist

    return




def check_exact_name(filename, artist_to_search):
    """
    בדיקה אם שם האמן מופיע בצורה מדויקת בתוך שם הקובץ
    
    פרמטר - שם קובץ או מטאדאטה
    
    פרמטר 2 - שם אמן קיים בתוך שם הקובץ
    
    ערך החזרה - אמת או שקר
    
    """
    
    # הגדרת חיפוש מיוחד
    PATTERN = r'( ו|[^א-ת])'

    # בדקו אם שם הזמר זהה במדויק לשם הקובץ
    if filename == artist_to_search:
        return True

    # בדוק אם שם הזמר מופיע בתחילת שם הקובץ
    elif filename.startswith(artist_to_search):
        next_char = filename[len(artist_to_search)]
        if re.search(PATTERN, next_char):
            return True

    # בדקו אם שם הזמר מופיע בסוף שם הקובץ
    elif filename.endswith(artist_to_search):
        index = filename.index(artist_to_search)
        
        # הגדרת התו הקדמי בהתאם למיקום שלו במחרוזת
        previous_char = filename[index - 1] if index == 1 else filename[index - 2:index]
        
        if re.search(PATTERN, previous_char):
            return True
        elif filename.find(previous_char) == 0 and previous_char in [" ", "ו"]:
            return True

    # בדקו אם שם הזמר מופיע באמצע שם הקובץ
    elif artist_to_search in filename[1:-1]:
        index = filename.index(artist_to_search)
        
        # הגדרת התו הקדמי בהתאם למיקום שלו במחרוזת
        previous_char = filename[index - 1] if index == 1 else filename[index - 2:index]

        next_char = filename[index + len(artist_to_search)]
        
        # בדיקה אם אין אותיות עבריות צמודות לשם הזמר
        if re.search(PATTERN, previous_char) and re.search(PATTERN, next_char):
            return True
            
    return False
    
    
if __name__ == '__main__':
   print(artist_from_song('השיר החדש של מבני פרידמן -.mp3'))


# previous_char in [" ", " ו", "("] and next_char in [" ", ".", ",", ")"]