import csv
import random

# קריאת קובץ ה-CSV
with open('../level1/singers_list.csv', 'r', encoding='utf-8') as file:
    reader = csv.reader(file)
    singers_list = [row[0] for row in reader if row]  # הנחת עבודה שהשמות הם בעמודה הראשונה

# קריאת הקובץ list_all_songs_clean.txt
with open('../level1/list_all_songs_clean.txt', 'r', encoding='utf-8') as file:
    songs = file.readlines()

# בחירת 100 שירים אקראיים
random_songs = random.sample(songs, 100)

def replace_singers(song, singers_list):
    for singer in singers_list:
        song = song.replace(singer, "SINGER")
    return song

# החלפת שמות הזמרים בשירים שנבחרו
modified_songs = [replace_singers(song, singers_list) for song in random_songs]

# שמירת השירים בקובץ חדש
with open('random_songs_sample.txt', 'w', encoding='utf-8') as file:
    file.writelines(modified_songs)

# הסרת השירים הנבחרים מהקובץ list_all_songs_clean.txt
remaining_songs_clean = [song for song in songs if song not in random_songs]

with open('../level1/list_all_songs_clean.txt', 'w', encoding='utf-8') as file:
    file.writelines(remaining_songs_clean)

# קריאת הקובץ list_all_songs.txt והסרת השירים הנבחרים
with open('../level1/list_all_songs.txt', 'r', encoding='utf-8') as file:
    all_songs = file.readlines()

remaining_songs_all = [song for song in all_songs if song not in random_songs]

with open('../level1/list_all_songs.txt', 'w', encoding='utf-8') as file:
    file.writelines(remaining_songs_all)
