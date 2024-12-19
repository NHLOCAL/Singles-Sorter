import os
import json
import google.generativeai as genai
import sys

# קונפיגורציה של ה-API Key
genai.configure(api_key=os.getenv("GEMINI_API_KEY"))

# קביעת גודל ה-chunks
CHUNK_SIZE = 50

# הגדרת הקונפיגורציה ליצירת המודל
generation_config = {
    "temperature": 0.9,
    "top_p": 0.95,
    "top_k": 40,
    "max_output_tokens": 8192,
    "response_mime_type": "text/plain",
}

# הגדרת המודל והנחיות המערכת
model = genai.GenerativeModel(
  model_name="gemini-2.0-flash-exp",
  generation_config=generation_config,
  system_instruction="""
אתה מתייג נתונים עבור אימון מודל NER לזיהוי ישויות מוזיקליות משמות שירים. 

מטרתך: להחזיר עבור כל שם שיר מבנה נתונים הכולל את שם השיר המקורי ומערך entities. 

אם אתה מזהה ישויות מסוג:
- SONG: שם השיר המלא.
- SINGER: שם האמן/מקהלה/להקה ללא תארים.
- ALBUM: שם האלבום המלא, אם קיים.
- GENRE: ז'אנר מוזיקלי (כמו "מזרחי", "ווקאלי", "חסידי", "ג'אז", "פופ", "רוק", "מוזיקה מקפיצה").
- MISC: מידע או טקסט לא ברור או מידע טכני כמו שנים ("2021"), תאריכים וכדו'.

אל תכלול מספרי רצועה כ-MISC או בכל קטגוריה אחרת.

פורמט הפלט עבור כל שיר:
[
  "<שם השיר המקורי>",
  {
    "entities": [
      ["<ישות כפי שמופיעה בשיר>", "<סוג הישות>"],
      ...
    ]
  }
]

אם אינך מזהה שום ישות רלוונטית, החזר entities כ-[].

הקפד להחזיר ישויות בצורת הטקסט המלא המופיע בשם השיר ללא חיתוכים. תייג כל אמן בנפרד, ללא תארים.

דוגמאות:
[ "חיים ישראל - מתנות קטנות (1)", { "entities": [ ["חיים ישראל", "SINGER"], ["מתנות קטנות (1)", "SONG"] ] } ]

""",
)

def read_in_chunks(file_object, chunk_size, start_line, end_line):
    """Generator לקריאת קובץ ב-chunks של מספר שורות מוגדר, עם מגבלה על טווח שורות."""
    chunk = []
    line_count = 0
    for line_number, line in enumerate(file_object, start=1):
        line = line.strip()
        if line and line_number >= start_line and line_number <= end_line:
            chunk.append(line)
            line_count += 1
            if len(chunk) == chunk_size:
                yield chunk
                chunk = []
            
    if chunk:
        yield chunk

def process_chunk(chat_session, songs_chunk):
    """מעבד חבילת שירים ושולח למודל ג'מיני."""
    # הכנת הקלט למודל - כל שיר בשורה נפרדת
    input_text = "\n".join(songs_chunk)
    
    # שליחת ההודעה למודל
    response = chat_session.send_message(input_text)
    
    # החזרת הטקסט הגולמי מהתגובה
    return response.text

def parse_response(response_text):
    """ממיר את הטקסט שהוחזר מהמודל למבנה JSON."""
    # הסרת רווחים וסימני קוד אם קיימים
    response_text = response_text.strip()
    if response_text.startswith("```"):
        # הסרת סימוני הקוד
        response_text = '\n'.join(response_text.split('\n')[1:-1]).strip()
    
    try:
        # ניסיון לפענח את הטקסט כ-JSON
        data = json.loads(response_text)
        return data
    except json.JSONDecodeError as e:
        print("שגיאה בפיענוח ה-JSON:", e)
        print("התוכן שהוחזר:", response_text)
        return None

def main():
    if len(sys.argv) != 3:
        print("שימוש: python script.py <שורה_התחלה> <שורה_סיום>")
        sys.exit(1)

    try:
        start_line = int(sys.argv[1])
        end_line = int(sys.argv[2])
        if start_line <= 0 or end_line <= 0 or start_line > end_line:
          raise ValueError("מספרי שורות לא תקינים")

    except ValueError as e:
        print(f"שגיאה בקלט: {e}, יש להזין מספרי שורות חיוביים שלמים")
        sys.exit(1)
        
    input_file = "machine-learn/scrape_data/level0/Gemini-synthetic-v2/all_songs.txt"
    output_file = 'tagged_songs.json'
    
    # יצירת חוברת הצ'אט
    chat_session = model.start_chat(history=[])
    
    # פתיחת הקובץ לכתיבה
    with open(output_file, 'w', encoding='utf-8') as out_f:
        out_f.write('[\n')  # התחלת מערך JSON
        first_entry = True  # לצורך הוספת פסיק בין האיברים
        
        with open(input_file, 'r', encoding='utf-8') as f:
            for chunk_number, songs_chunk in enumerate( read_in_chunks(f, chunk_size=100, start_line=start_line, end_line=end_line), start=1):
                print(f"מעבד חבילה מספר {chunk_number} עם {len(songs_chunk)} שירים...")
                response_text = process_chunk(chat_session, songs_chunk)
                print(f"תשובה מחבילה מספר {chunk_number}:")
                print(response_text)  # הדפסת התגובה כדי לבדוק את הפורמט
                
                tagged_data = parse_response(response_text)
                if tagged_data:
                    # בדיקה אם הנתונים הם רשימה של רשימות
                    if isinstance(tagged_data, list):
                        for entry in tagged_data:
                            if not first_entry:
                                out_f.write(',\n')
                            json.dump(entry, out_f, ensure_ascii=False, indent=4)
                            first_entry = False
                    else:
                        print(f"התגובה מחבילה מספר {chunk_number} אינה ברשימה. דלג עליה.")
                else:
                    print(f"לא ניתן לעבד את החבילה מספר {chunk_number}. דלג עליה.")
        
        out_f.write('\n]')  # סיום מערך JSON
    
    print(f"תהליך התיוג הושלם. התוצאות נשמרו בקובץ {output_file}.")

if __name__ == "__main__":
    main()
