import json
import logging
import random
from collections import defaultdict
import matplotlib.pyplot as plt
import pandas as pd

# הגדרת הלוגר
logging.basicConfig(
    filename='data_validation.log',
    filemode='w',
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

def load_tagged_data(file_path):
    """טען את הנתונים התויגים מקובץ JSON."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            tagged_data = json.load(f)
        logging.info(f"נפתח בהצלחה הקובץ {file_path}.")
        return tagged_data
    except FileNotFoundError:
        logging.error(f"הקובץ {file_path} לא נמצא.")
    except json.JSONDecodeError as e:
        logging.error(f"שגיאה בפיענוח JSON בקובץ {file_path}: {e}")
    return None

def validate_json_structure(tagged_data):
    """בדוק את מבנה ה-JSON."""
    valid = True
    for idx, record in enumerate(tagged_data):
        if not isinstance(record, list) or len(record) != 2:
            logging.error(f"שגיאה במבנה הרשומה מספר {idx}: לא רשימה עם שני אלמנטים.")
            valid = False
            continue
        song_name, entities = record
        if not isinstance(song_name, str):
            logging.error(f"שגיאה במבנה הרשומה מספר {idx}: שם השיר אינו מחרוזת.")
            valid = False
        if not isinstance(entities, dict) or "entities" not in entities:
            logging.error(f"שגיאה במבנה הרשומה מספר {idx}: אין אובייקט 'entities'.")
            valid = False
        else:
            for entity in entities["entities"]:
                if not (isinstance(entity, list) and len(entity) == 3):
                    logging.error(f"שגיאה במבנה הישויות ברשומה מספר {idx}: {entity}")
                    valid = False
                else:
                    start, end, entity_type = entity
                    if not (isinstance(start, int) and isinstance(end, int) and isinstance(entity_type, str)):
                        logging.error(f"שגיאה בסוגי הנתונים של הישויות ברשומה מספר {idx}: {entity}")
                        valid = False
                    if entity_type not in ["SINGER", "SONG", "ALBUM", "GENRE", "MISC"]:
                        logging.error(f"שגיאה בסוג הישות ברשומה מספר {idx}: {entity_type}")
                        valid = False
    if valid:
        logging.info("כל הרשומות תקינות מבחינת המבנה.")
    else:
        logging.warning("נמצאו שגיאות במבנה הנתונים.")
    return valid

def generate_statistics(tagged_data):
    """הפק סטטיסטיקות על הישויות."""
    stats = defaultdict(int)
    total_entities = 0
    for record in tagged_data:
        _, entities = record
        for entity in entities["entities"]:
            _, _, entity_type = entity
            stats[entity_type] += 1
            total_entities += 1
    logging.info(f"סה\"כ ישויות: {total_entities}")
    for entity_type, count in stats.items():
        logging.info(f"{entity_type}: {count}")
    return stats, total_entities

def visualize_statistics(stats):
    """הצג גרף של חלוקת סוגי הישויות."""
    entity_types = list(stats.keys())
    counts = list(stats.values())

    plt.figure(figsize=(10, 6))
    plt.bar(entity_types, counts, color='skyblue')
    plt.xlabel('סוג ישות')
    plt.ylabel('מספר הישויות')
    plt.title('חלוקת סוגי הישויות בתיוג הנתונים')
    plt.xticks(rotation=45)
    plt.tight_layout()
    plt.show()
    logging.info("הוצג גרף חלוקת סוגי הישויות ושמירה בקובץ 'entity_distribution.png'.")

def inspect_samples(tagged_data, sample_size=10):
    """בחר והצג דוגמאות אקראיות לבדיקת דיוק התיוג."""
    if len(tagged_data) < sample_size:
        sample_size = len(tagged_data)
    samples = random.sample(tagged_data, sample_size)
    for idx, sample in enumerate(samples, start=1):
        song, entities = sample
        print(f"\nדוגמה {idx}:")
        print(f"שיר: {song}")
        print("ישות:")
        for entity in entities["entities"]:
            start, end, entity_type = entity
            entity_text = song[start:end]
            print(f"  - {entity_type}: '{entity_text}' (מיקומים: {start}-{end})")
    logging.info(f"הוצגו {sample_size} דוגמאות אקראיות לבדיקה ידנית.")

def main():
    output_file = 'tagged_songs_ner.json'
    
    # טען את הנתונים התויגים
    tagged_data = load_tagged_data(output_file)
    if not tagged_data:
        print("לא ניתן לטעון את הנתונים התויגים. בדוק את הלוג למידע נוסף.")
        return
    
    # בדוק את מבנה ה-JSON
    is_valid = validate_json_structure(tagged_data)
    if not is_valid:
        print("נמצאו שגיאות במבנה הנתונים. בדוק את הלוג לפרטים.")
    else:
        print("מבנה ה-JSON תקין.")
    
    # הפק סטטיסטיקות
    stats, total = generate_statistics(tagged_data)
    print(f"\nסה\"כ ישויות: {total}")
    for entity_type, count in stats.items():
        print(f"{entity_type}: {count}")
    
    # הצג ויזואליזציה
    visualize_statistics(stats)
    
    # בדיקה ידנית של דוגמאות
    inspect_samples(tagged_data, sample_size=10)
    
    print("\nתהליך הבדיקה הושלם. בדוק את 'data_validation.log' לפרטים נוספים.")

if __name__ == "__main__":
    main()
