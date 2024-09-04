import csv

# קריאת קובץ הזמרים מתוך ה-CSV
def load_singers_from_csv(csv_file):
    singers = []
    with open(csv_file, 'r', encoding='utf-8') as file:
        reader = csv.reader(file)
        for row in reader:
            if row:  # אם השורה לא ריקה
                singers.append(row[0].strip())  # להוסיף את הזמרים לרשימה
    return singers

# קריאת קובץ השירים מתוך קובץ הטקסט
def load_songs_from_text(text_file):
    with open(text_file, 'r', encoding='utf-8') as file:
        songs = file.read()
    return songs

# כתיבת הרשימות לקבצים חדשים
def write_list_to_file(lst, filename):
    with open(filename, 'w', encoding='utf-8') as file:
        for item in lst:
            file.write(f"{item}\n")

# פונקציה ראשית לביצוע ההשוואה ושמירת התוצאות
def process_singers_and_songs(csv_file, text_file):
    singers = load_singers_from_csv(csv_file)
    songs_text = load_songs_from_text(text_file)
    
    found_singers = []
    not_found_singers = []

    for singer in singers:
        if singer in songs_text:
            found_singers.append(singer)
        else:
            not_found_singers.append(singer)

    write_list_to_file(found_singers, 'found_singers.txt')
    write_list_to_file(not_found_singers, 'not_found_singers.txt')

# קריאה לפונקציה עם שמות הקבצים שלך
process_singers_and_songs('singers_list.csv', 'list_all_songs.txt')
