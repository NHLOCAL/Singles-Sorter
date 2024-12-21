import json

# קריאה מקובץ JSON
input_file = "tagged_songs.json" # שים את שם הקובץ שלך כאן
output_file_1 = 'last_250_items.json'
output_file_2 = 'tagged_songs.json'

# טוען את הנתונים
with open(input_file, 'r', encoding='utf-8') as file:
    data = json.load(file)

# בדיקה שהנתונים הם רשימה
if not isinstance(data, list):
    raise ValueError("The JSON structure is not a list")

# פיצול הרשימה לשתי חלקים
last_250_items = data[-250:]
remaining_items = data[:-250]

# כתיבה לקבצים חדשים
with open(output_file_1, 'w', encoding='utf-8') as file:
    json.dump(last_250_items, file, ensure_ascii=False, indent=4)

with open(output_file_2, 'w', encoding='utf-8') as file:
    json.dump(remaining_items, file, ensure_ascii=False, indent=4)

print("The JSON file has been split successfully!")
