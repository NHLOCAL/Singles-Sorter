import json

def convert_json_for_ner(input_file, output_file):
    """
    ממיר קובץ JSON עם מחרוזות מיקום לקובץ JSON עם מיקומי תוים.

    Args:
      input_file (str): נתיב קובץ ה-JSON המקורי.
      output_file (str): נתיב קובץ ה-JSON החדש.
    """

    with open(input_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    converted_data = []
    for item in data:
        text = item[0]
        entities = item[1]["entities"]
        new_entities = []
        for entity in entities:
            entity_text = entity[0]
            entity_type = entity[1]
            start_index = text.find(entity_text)
            if start_index != -1:
                end_index = start_index + len(entity_text)
                new_entities.append([start_index, end_index, entity_type])
        
        converted_data.append([text, {"entities": new_entities}])

    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(converted_data, f, indent=4, ensure_ascii=False)

if __name__ == '__main__':
    input_file = 'tagged_songs.json'  # שנה לנתיב קובץ הקלט שלך
    output_file = 'tagged_songs_ner.json' # שנה לנתיב קובץ הפלט שלך
    convert_json_for_ner(input_file, output_file)
    print(f"הקובץ {input_file} הומר בהצלחה לקובץ {output_file}")
