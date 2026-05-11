# -*- coding: utf-8 -*-
import os
import re
import csv
import difflib

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




def advanced_normalization(text):
    """מנקה את הטקסט ומנרמל אותו פונטית כדי להתגבר על כתיב חסר/מלא ושמות באידיש."""
    if not text:
        return ""
    # 1. הסרת תווים מיוחדים ומפרידים (גרש, מרכאות, מקפים)
    text = re.sub(r'[\'\"״׳\-_`\.]', '', text)
    # 2. החלפת 'ע' סופית ב-'ה' (אהרלע -> אהרלה)
    text = re.sub(r'ע\b', 'ה', text)
    # 3. הסרת א' ו-ע' שמשמשות כתנועות באמצע מילה (סאמעט -> סמט)
    def remove_yiddish_vowels(word):
        if len(word) <= 1: return word
        # משאיר את האות הראשונה כפי שהיא, ומוחק א/ע משאר המילה
        return word[0] + re.sub(r'[אע]', '', word[1:])
    words = [remove_yiddish_vowels(w) for w in text.split()]
    text = " ".join(words)
    # 4. צמצום כפילויות של ו' ו-י'
    text = re.sub(r'ו+', 'ו', text)
    text = re.sub(r'י+', 'י', text)
    return re.sub(r'\s+', ' ', text).strip()


def check_exact_name(filename, artist_to_search):
    """גרסה משופרת הכוללת נורמליזציה פונטית והתאמה חכמה (Fuzzy Matching)
    לזיהוי שגיאות כתיב וגרסאות איות שונות (כמו 'אהרל'ע סאמעט' ו-'מוטי שטיימץ')."""

    # --- שלב א': בדיקה קלאסית עם ו' החיבור ---
    # מונע פגיעה בשמות שתמיד עבדו כראוי
    filename_clean = filename.lstrip()
    escaped_artist = re.escape(artist_to_search)
    exact_match_pattern = fr'(^|[^א-ת])ו?{escaped_artist}\b'
    if re.search(exact_match_pattern, filename_clean):
        return True

    # --- שלב ב': נורמליזציה פונטית ---
    # מטפל במקרים כמו: אהרל'ע סאמעט -> אהרלה סמט
    norm_file = advanced_normalization(filename)
    norm_artist = advanced_normalization(artist_to_search)

    # בדיקת הכלה אחרי נורמליזציה (כולל טיפול ב-ו' החיבור)
    if re.search(fr'(^|[^א-ת])ו?{re.escape(norm_artist)}\b', norm_file):
        return True

    # --- שלב ג': Fuzzy Matching (התאמה חכמה ממוקדת שגיאות כתיב) ---
    # מטפל במקרים כמו: שטיימץ -> שטיינמץ (אות שנשמטה/התחלפה)
    # אנחנו מחלקים את שם השיר ל"חלונות" של מילים כדי לא להשוות אמן קצר לשם שיר ארוך מדי
    artist_words = norm_artist.split()
    file_words = norm_file.split()
    num_words = len(artist_words)
    if num_words > 0 and len(file_words) >= num_words:
        for i in range(len(file_words) - num_words + 1):
            chunk = " ".join(file_words[i:i+num_words])
            # אם המילה הראשונה ב"חלון" מתחילה ב-ו' (אולי ו' החיבור), נבדוק גם בלעדיה
            chunk_no_vav = chunk[1:] if chunk.startswith('ו') else chunk
            # בדיקת דמיון. יחס של 88% ומעלה תופס שגיאת כתיב של אות אחת בשם מלא,
            # אבל מונע זיהוי שגוי בין אמנים שונים.
            if difflib.SequenceMatcher(None, norm_artist, chunk).ratio() >= 0.88 or \
               difflib.SequenceMatcher(None, norm_artist, chunk_no_vav).ratio() >= 0.88:
                return True

    return False

    
  
  
if __name__ == '__main__':

    
    list_ = ['ח בני פרידמן, מוטי שטיינמ.mp3', '@יואלי קליין=.mp3', 'ואברהם פריד.mp3', 'שיר נוסף - מוטי שטיינמץל מ.mp3'] 
    
    for i in list_:
        print(artist_from_song(i))