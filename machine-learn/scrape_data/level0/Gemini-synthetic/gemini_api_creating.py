import os
import google.generativeai as genai

# קונפיגורציה של ה-API Key
genai.configure(api_key=os.getenv("GEMINI_API_KEY"))

# הגדרת קונפיגורציה עבור המודל
generation_config = {
    "temperature": 0.9,
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
# יצירת וריאציות של כותרות שירים בעברית

**מטרת המשימה:**  ליצור כותרות שירים מגוונות בעברית, תוך שימוש במילה  "SINGER" כמחזיק מקום לשם הזמר/המקהלה.  הדגש הוא על יצירת וריאציות שונות של אותן כותרות, ולא על כתיבת כותרות חדשות לחלוטין.

**כללי עבודה:**

* **שימוש ב- "SINGER":**  השתמש אך ורק במילה "SINGER" כמציין מיקום לשם הזמר או מקהלה.  **אל תחליף אותה בשם זמר אמיתי.**

* **בסיס הנתונים:**  תשתמש ברשימת הכותרות הבאה כבסיס ליצירת הווריאציות.  **אסור להוסיף כותרות חדשות שלא מופיעות ברשימה זו.**

* **ווריאציות:**  צור וריאציות שונות של כל כותרת ברשימה, על ידי:
    * **שינוי סדר מילים:**  תשנה את סדר המילים בכותרת, תוך שמירה על המשמעות.
    * **הוספת פרטים:**  הוסף מידע רלוונטי, כמו סגנון מוזיקלי, שם אלבום, או פרטים נוספים הקשורים לשיר.
    * **הסרת פרטים:**  הסר מידע שאינו חיוני מהכותרת המקורית.
    * **שינוי ניסוח:**  שנה את הניסוח של הכותרת, תוך שמירה על המשמעות המקורית.
    * *שימוש בסוגריים:**  הימנע משימוש מוגזם בסוגריים בכותרות השירים. תשתמש בסוגריים כמה שפחות!
    * **איסור על נקודותיים:** הקפד שלא להשתמש בנקודותיים בכותרות השירים!

* **איכות הכותרות:**  הקפד על כתיבה תקינה, נכונה דקדוקית ובשפה ברורה.  **אל תיצור כותרות חסרות משמעות או גרועות.**

* **תפוקת המערכת:**  תפוקת המערכת תהיה רשימה של כותרות שירים חדשות, שהן וריאציות על הכותרות המקוריות.  **אל תוסיף הערות, מספור, או כל מידע נוסף מלבד כותרות השירים.**
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
START_LINE = 1001  # קו ההתחלה
END_LINE = 3000  # קו הסיום

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
