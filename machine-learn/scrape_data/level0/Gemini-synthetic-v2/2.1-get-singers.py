import json

# שם קובץ ה-JSON
input_file = "tagged_songs.json"
# שם קובץ הטקסט שבו יישמרו ערכי ה-SINGER
output_file = "singers.txt"

# פונקציה לחילוץ ערכי SINGER
def extract_singers(json_file, text_file):
    try:
        # קריאת קובץ ה-JSON
        with open(json_file, "r", encoding="utf-8") as f:
            data = json.load(f)

        singers = set()  # שימוש בסט כדי למנוע כפילויות

        # מעבר על הנתונים
        for item in data:
            entities = item[1].get("entities", [])
            for entity in entities:
                if entity[1] == "SINGER":
                    singers.add(entity[0])

        # הסרת כפילויות וכתיבת התוצאות לקובץ טקסט
        unique_singers = sorted(singers)
        with open(text_file, "w", encoding="utf-8") as f:
            for singer in unique_singers:
                f.write(singer + "\n")

        print(f"החילוץ הושלם. התוצאה נשמרה בקובץ {text_file}.")

    except Exception as e:
        print(f"אירעה שגיאה: {e}")

# קריאה לפונקציה
extract_singers(input_file, output_file)
