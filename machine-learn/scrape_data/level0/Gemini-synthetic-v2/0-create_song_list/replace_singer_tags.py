import csv
import random

# קובץ CSV וקובץ טקסט
singers_file = 'singers_list.csv'
songs_file = 'all_songs_synthetic.txt'
output_file = 'all_songs2.txt'


# שלב 1: טוען את רשימת הזמרים
def load_singers(file_path):
    with open(file_path, 'r', encoding='utf-8') as file:
        return [row[0] for row in csv.reader(file) if row]

# שלב 2: החלפת תגי SINGER בזמרים אקראיים
def replace_singer_tags(songs, singers):
    updated_songs = []
    for line in songs:
        while 'SINGER' in line:
            line = line.replace('SINGER', random.choice(singers), 1)
        updated_songs.append(line)
    return updated_songs

# שלב 3: קריאה, ערבוב ושמירה
def randomize_and_save_songs(singers_file, songs_file, output_file):
    # טוען זמרים
    singers = load_singers(singers_file)
    
    # טוען את תוכן קובץ השירים
    with open(songs_file, 'r', encoding='utf-8') as file:
        songs = file.readlines()
    
    # מחליף תגי SINGER
    updated_songs = replace_singer_tags(songs, singers)
    
    # מערבב את השורות
    random.shuffle(updated_songs)
    
    # שומר את הקובץ החדש
    with open(output_file, 'w', encoding='utf-8') as file:
        file.writelines(updated_songs)

# הרצת הפונקציה
randomize_and_save_songs(singers_file, songs_file, output_file)

print(f'קובץ חדש נוצר: {output_file}')
