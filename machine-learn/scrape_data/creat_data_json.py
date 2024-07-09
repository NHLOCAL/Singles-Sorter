import csv
import json
import re

# קריאת שמות השירים
with open("list_all_songs_clean.txt", mode='r', encoding='utf-8') as file:
    song_names = [row.strip() for row in file]

# קריאת שמות הזמרים ויצירת set
with open('singers_list.csv', mode='r', newline='', encoding='utf-8') as file:
    reader = csv.reader(file)
    singer_names = {row[0].strip() for row in reader}

# יצירת מילון של ביטויים רגולריים מראש
singer_regex = {singer: re.compile(r'\b' + re.escape(singer) + r'\b') for singer in singer_names}

# פתיחת קובץ JSON לכתיבה
with open('new-data.json', 'w', encoding='utf-8') as json_file:
    json_file.write('[\n')
    first_item = True

    # עיבוד כל שיר
    for song_name in song_names:
        entities = []
        for singer, regex in singer_regex.items():
            for match in regex.finditer(song_name):
                entities.append([match.start(), match.end(), "SINGER"])
        
        if entities:
            if not first_item:
                json_file.write(',\n')
            json.dump([song_name, {"entities": entities}], json_file, ensure_ascii=False)
            first_item = False

    json_file.write('\n]')

print('Dataset saved to new-data.json')
