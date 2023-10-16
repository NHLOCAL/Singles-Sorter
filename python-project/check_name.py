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
    Check if the artist's name appears exactly in the filename, even if preceded by "ו".

    Parameters:
    filename (str): The filename or metadata.
    artist_to_search (str): The artist's name to search for.

    Returns:
    bool: True if the artist's name is found exactly in the filename (even if preceded by "ו"), False otherwise.
    """
    
    # Remove leading spaces in the filename
    filename = filename.lstrip()
    
    # Escape special characters in the artist's name
    escaped_artist = re.escape(artist_to_search)
    
    # Define a pattern to match the exact artist name, even if preceded by "ו"
    exact_match_pattern = fr'(^|[^א-ת])ו?{escaped_artist}\b'

    # Search for the exact artist name in the filename
    if re.search(exact_match_pattern, filename):
        return True

    return False

    
  
  
if __name__ == '__main__':

    
    list_ = ['ח בני פרידמן, מוטי שטיינמ.mp3', '@יואלי קליין=.mp3', 'ואברהם פריד.mp3', 'שיר נוסף - מוטי שטיינמץל מ.mp3'] 
    
    for i in list_:
        print(artist_from_song(i))