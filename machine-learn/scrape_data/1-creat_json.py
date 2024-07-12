import csv
import json
import re

# קריאת שמות השירים
with open(r"level1\list_all_songs_random.txt", mode='r', encoding='utf-8') as file:
    song_names = [row.strip() for row in file]

# קריאת שמות הזמרים ויצירת set
with open(r'level1\singers_list.csv', mode='r', newline='', encoding='utf-8') as file:
    reader = csv.reader(file)
    singer_names = {row[0].strip() for row in reader}

# יצירת מילון של ביטויים רגולריים מראש
singer_regex = {singer: re.compile(r'\b' + re.escape(singer) + r'\b') for singer in singer_names}

# פונקציה לעדכון אופסטים בהתאם לתווים מיוחדים
def adjust_offsets(text, start, end):
    adjusted_start = start
    adjusted_end = end
    for i, char in enumerate(text[:start]):
        if char == '-':
            adjusted_start -= 1
    for i, char in enumerate(text[:end]):
        if char == '-':
            adjusted_end -= 1
    return adjusted_start, adjusted_end

# פתיחת קובץ JSON לכתיבה
with open('new-data.json', 'w', encoding='utf-8') as json_file:
    json_file.write('[\n')
    first_item = True

    # עיבוד כל שיר
    for song_name in song_names:
        entities = []
        for singer, regex in singer_regex.items():
            for match in regex.finditer(song_name):
                start, end = match.start(), match.end()
                start, end = adjust_offsets(song_name, start, end)
                entities.append([start, end, "SINGER"])
        
        if entities:
            if not first_item:
                json_file.write(',\n')
            json.dump([song_name, {"entities": entities}], json_file, ensure_ascii=False)
            first_item = False

    json_file.write('\n]')

print('Dataset saved to new-data.json')
