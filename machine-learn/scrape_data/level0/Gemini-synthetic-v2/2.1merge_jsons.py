import json
import glob
import zipfile

# הגדר תיקייה
zip_folder = "song-tags-output"  # תיקייה שבה נמצאים קבצי ה-ZIP
output_file = "tagged_songs.json"  # שם קובץ JSON הממוזג

# רשימה לאחסון הנתונים מכל הקבצים
merged_data = []

# קריאת כל קבצי ZIP בתיקייה
for zip_path in glob.glob(f"{zip_folder}/*.zip"):
    with zipfile.ZipFile(zip_path, 'r') as zip_ref:
        # מציאת כל הקבצים עם סיומת .json בתוך קובץ ה-ZIP
        json_files = [f for f in zip_ref.namelist() if f.endswith('.json')]
        for json_file in json_files:
            # קריאת תוכן קובץ JSON ישירות מתוך ה-ZIP
            with zip_ref.open(json_file) as f:
                data = json.load(f)
                merged_data.extend(data)  # הוספת הנתונים לרשימה הממוזגת

# כתיבת הנתונים הממוזגים לקובץ JSON חדש
with open(output_file, 'w', encoding='utf-8') as f:
    json.dump(merged_data, f, ensure_ascii=False, indent=4)

# הודעה על הצלחה
print(f"כל הנתונים מקבצי ה-ZIP מוזגו לקובץ {output_file}")
