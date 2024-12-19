import json

def validate_json_structure(tagged_data):
    valid = True
    for idx, record in enumerate(tagged_data):
        print(f"{idx}, {record}, in {tagged_data}")
        if not isinstance(record, list) or len(record) != 2:
            print(f"שגיאה במבנה הרשומה מספר {idx}: לא רשימה עם שני אלמנטים")
            valid = False
            continue
        song_name, entities = record
        if not isinstance(song_name, str):
            print(f"שגיאה במבנה הרשומה מספר {idx}: שם השיר אינו מחרוזת")
            valid = False
        if not isinstance(entities, dict) or "entities" not in entities:
            print(f"שגיאה במבנה הרשומה מספר {idx}: אין אובייקט 'entities'")
            valid = False
        else:
            for entity in entities["entities"]:
                if not (isinstance(entity, list) and len(entity) == 3):
                    print(f"שגיאה במבנה הישויות ברשומה מספר {idx}: {entity}")
                    valid = False
                else:
                    start, end, entity_type = entity
                    if not (isinstance(start, int) and isinstance(end, int) and isinstance(entity_type, str)):
                        print(f"שגיאה בסוגי הנתונים של הישויות ברשומה מספר {idx}: {entity}")
                        valid = False
                    if entity_type not in ["SINGER", "SONG", "ALBUM", "GENRE", "MISC"]:
                        print(f"שגיאה בסוג הישות ברשומה מספר {idx}: {entity_type}")
                        valid = False
    return valid

def main():
    output_file = 'tagged_songs.json'
    with open(output_file, 'r', encoding='utf-8') as f:
        tagged_data = json.load(f)
    
    if validate_json_structure(tagged_data):
        print("כל הרשומות תקינות!")
    else:
        print("נמצאו שגיאות במבנה הנתונים.")

if __name__ == "__main__":
    main()
