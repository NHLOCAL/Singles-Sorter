import os
import google.generativeai as genai

# קונפיגורציה של ה-API Key
genai.configure(api_key=os.getenv("GEMINI_API_KEY"))

# הגדרת קונפיגורציה עבור המודל
generation_config = {
    "temperature": 1.0,
    "top_p": 0.95,
    "top_k": 40,
    "max_output_tokens": 8192,
    "response_mime_type": "text/plain"
}

# יצירת המודל עם ההוראות
model = genai.GenerativeModel(
    model_name="gemini-1.5-flash-002",
    generation_config=generation_config,
    system_instruction="""
# יצירת וריאציות של שמות שירים

**מטרה:** ליצור גרסאות שונות ומגוונות של כותרות שירים קיימות, תוך שימוש במילה "SINGER" כמציין מיקום לשם הזמר/ה.  הדגש הוא על שינוי צורה וניסוח של הכותרות המקוריות, **לא** על יצירת כותרות חדשות לגמרי.

**כללים:**

1. **"SINGER" כמציין מיקום:** השתמש אך ורק במילה "SINGER" במקום שם הזמר/ה.  **אל תשתמש בשמות אמיתיים.**

2. **בסיס נתונים:** השתמש אך ורק ברשימת הכותרות המופיעה למטה.  **אסור להמציא כותרות חדשות.**

3. **יצירת וריאציות:**  צור גרסאות שונות של כל כותרת על ידי:
    * שינוי סדר המילים.
    * הוספת מידע רלוונטי (סגנון מוזיקלי, שם אלבום וכו').
    * הסרת מידע לא חיוני.
    * שינוי ניסוח תוך שמירה על המשמעות המקורית.

4. **סגנון:** חקה את הסגנון והמבנה של כותרות השירים המקוריות.  למד את הדוגמאות היטב.

5. **איכות:** הקפד על דקדוק ותקינות לשונית.  הכותרות צריכות להיות ברורות ומובנות.  **אל תיצור כותרות חסרות משמעות או בעלות ניסוח גרוע.**

6. **שימוש בסוגריים:** הימנע משימוש מוגזם בסוגריים. השתמש בהם רק כשזה הכרחי להבהרת הכותרת.

7. **איסור על נקודתיים:** **אסור להשתמש בנקודתיים בכותרות.**

8. **תפוקה:** רשימה של כותרות שירים בלבד – וריאציות של הכותרות המקוריות. **אין להוסיף הערות, מספור רשימה או כל מידע אחר, מלבד מספרי רצועות או תוכן מספרי דומה, אם רלוונטי לכותרת המקורית.**

9. **וריאציה אינה כותרת חדשה:**  וריאציה היא שינוי של הכותרת *המקורית*.  אל תיצור כותרות חדשות וקצרות.  המטרה היא לשנות את הניסוח והמבנה, לא לכתוב כותרת חדשה לגמרי.

**דוגמאות למה *לא* לעשות:**

```
SINGER במחרוזת מקפיצה וסוערת ''די טרעק'' - די טרעק: SINGER  (לא נכון - יצירת כותרת מקוצרת ושימוש בנקודתיים)
SINGER בסינגל חדש ''מזמור לתודה'' - מזמור לתודה: SINGER (לא נכון - יצירת כותרת מקוצרת ושימוש בנקודתיים)
```
"""
)

# התחלת סשן צ'אט עם היסטוריה של שיחה
chat_session = model.start_chat(
    history=[
        {
            "role": "user",
            "parts": [
                {"text": "להלן כותרות השירים המקוריות ליצירת וריאציות."}
            ],
        },
        {
            "role": "model",
            "parts": [
                {"text": "התחלת יצירת וריאציות לכותרות."}
            ],
        }
    ]
)

# קבועים להגדרת הטווח לסריקה
START_LINE = 16001  # קו ההתחלה
END_LINE = 17600  # קו הסיום

# פונקציה לקריאת קובץ בטווח שורות מוגדר
def read_file_in_chunks(file_path, chunk_size=100, start_line=1, end_line=None):
    with open(file_path, 'r', encoding='utf-8') as file:
        for current_line in range(start_line - 1):
            file.readline()  # דילוג על שורות עד לקו ההתחלה
        line_count = start_line
        while True:
            if end_line and line_count > end_line:
                break
            lines = [file.readline().strip() for _ in range(chunk_size)]
            line_count += chunk_size
            if not any(lines):
                break
            yield [line for line in lines if line]

# פונקציה לעיבוד כותרות השירים ושמירת התוצאות לקובץ
def process_song_titles(file_path, output_file, start_line, end_line):
    with open(output_file, 'w', encoding='utf-8') as f:
        for chunk in read_file_in_chunks(file_path, start_line=start_line, end_line=end_line):
            input_text = "\n".join(chunk)
            prompt = f"להלן כותרות השירים:\n{input_text}\n"
            
            # שליחת השאלה למודל וקבלת התשובה
            response = chat_session.send_message(prompt)
            
            # כתיבת התשובה לקובץ והדפסה
            f.write(response.text + "\n\n")
            print(response.text)

# שימוש לדוגמה
if __name__ == "__main__":
    file_path = "machine-learn/scrape_data/level0/Gemini-synthetic/songs_data.txt"  # קובץ הכניסה עם כותרות השירים
    output_file = "song_variations_output.txt"  # קובץ הפלט לאחסון הווריאציות
    process_song_titles(file_path, output_file, START_LINE, END_LINE)
