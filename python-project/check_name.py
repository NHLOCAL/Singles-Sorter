import os
import re
import csv

def artist_from_song(my_file):
    """
    הפונקציה בודקת את שם האמן בשם הקובץ על סמך מסד נתונים ומאחסנת את שם האמן במשתנה.
        אם השם לא קיים, הוא סורק את המטא נתונים של השיר ומאחסן את שם האמן במשתנה.

        טיעונים:
            my_file (str): שם הקובץ שיש לסרוק.

        החזרות:
            str: הערך המכיל את שם האמן מהקובץ.
    """
    
    PATTERN = r'( ו|[^א-ת])'
    
    # קבל את שם הקובץ ללא הנתיב המלא
    split_file = os.path.split(my_file)[1]
    split_file = os.path.splitext(split_file)[0]

    # הסר תווים לא רצויים משם הקובץ
    split_file = split_file.replace('_', ' ')
    split_file = split_file.replace('-', ' ')

    # Import the list of singers from a CSV file
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

            # בדקו אם שם הזמר מופיע בתחילת ובסוף שם הקובץ
            if split_file.startswith(source_name) and split_file.endswith(source_name):
                print(1)
                return artist

            # בדוק אם שם הזמר מופיע בתחילת שם הקובץ
            elif split_file.startswith(source_name):
                next_char = split_file[len(source_name)]
                if next_char in [" ", ".", ","]:
                    print(2)
                    return artist

            # בדקו אם שם הזמר מופיע בסוף שם הקובץ
            elif split_file.endswith(source_name):
                index = split_file.index(source_name)
                
                # הגדרת התו הקדמי בהתאם למיקום שלו במחרוזת
                previous_char = split_file[index - 1] if index == 1 else split_file[index - 2:index]
                
                if re.search(PATTERN, previous_char):
                    print(3)
                    return artist
                elif split_file.find(previous_char) == 0 and previous_char in [" ", "ו"]:
                    print(4)
                    return artist

            # בדקו אם שם הזמר מופיע באמצע שם הקובץ
            elif source_name in split_file[1:-1]:
                index = split_file.index(source_name)
                
                # הגדרת התו הקדמי בהתאם למיקום שלו במחרוזת
                previous_char = split_file[index - 1] if index == 1 else split_file[index - 2:index]

                next_char = split_file[index + len(source_name)]
                
                # בדיקה אם המחרוזת
                if re.search(PATTERN, previous_char) and re.search(PATTERN, next_char):
                    print(5)
                    return artist

    return None

if __name__ == '__main__':
   print(artist_from_song('מוטי ויואלי קליין .mp3'))


# previous_char in [" ", " ו", "("] and next_char in [" ", ".", ",", ")"]