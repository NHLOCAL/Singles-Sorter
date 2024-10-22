import os
import google.generativeai as genai
import sys

# קונפיגורציה של ה-API Key
genai.configure(api_key=os.getenv("GEMINI_API_KEY"))

# הגדרת קונפיגורציה עבור המודל
generation_config = {
    "temperature": 1.2,
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
# צור וריאציות של כותרות שירים

**מטרה מרכזית:**  ליצור וראיציות של שמות שירים על ידי להוסיף ולהחליף מילים על בסיס רשימת המשתמש

## **הנחיות:**

1. **רק "SINGER" כשם הזמר/ת.** - אל תשתמש בשמות אמיתיים. אפשר להשתמש ב"SINGER" כמה פעמים.

2. **החלפת והוספת מילים:** - תחליף ותוסיף מילים לשמות השירים.

3. **תוכן פגום:** - הוסף מספרי רצועות ותוכן פגום כמו "copy" "#" או "$" לחלק מהשירים.

4. **תווים אסורים:**  **אסור להשתמש בתווים הבאים:**  `\ / : * ? " < > |`

5.  **תפוקה:**  רשימה של **שמות שירים** בלבד. אין הערות או מידע נוסף.

## דוגמאות:

### קלט המשתמש:
בוא לפה  ווקאלי SINGER וSINGER
סט להיטים קיץ 2021 SINGER
SINGER - סיפור אחר   שיר הנושא מתוך סרטו של אבי נשר
Keracheim Acapella - Simcha Leiner (ft. Meshorerim Choir) - כרחם - SINGER‬ - YouTube וואקלי
2 תוגת הלב- SINGER וSINGER
14-SINGER
SINGER וSINGER עם ילד הפלא SINGER - תפילין - שמע ישראל
הסינגל החדש של SINGER אילו פינו
03 SINGER שר מה אשיב במעמד אלפים

### תפוקה רצויה:
SINGER & SINGER בשיר ווקאלי - גש הנה!
11 SINGER מציג את הלהיטים המדהימים של אביב 2019
SINGER וSINGER - השיר מתוך הסרט של אביב נשר האיש בחליפה
Keracheim Acapella - Simcha Leiner (ft. Meshorerim Choir) - כרחם
09 לב עצוב SINGER עם SINGER
עותק של 19 SINGER @
הילדSINGER שר את תפילת שמע ישראל, תפילין
SINGER SINGER SINGER וSINGER בסינגל חדש ''ואילו'' ץץץ
SINGER ריגש את ההמונים עם מה אשיב
"""
)

# התחלת סשן צ'אט עם היסטוריה של שיחה
chat_session = model.start_chat(
    history=[
        {"role": "user", "parts": [{"text": "להלן כותרות השירים המקוריות ליצירת וריאציות."}]},
        {"role": "model", "parts": [{"text": "התחלת יצירת וריאציות לכותרות."}]}
    ]
)

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
    with open(output_file, 'a', encoding='utf-8') as f:  # Append mode
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
    
    # קריאה לארגומנטים משורת הפקודה
    if len(sys.argv) != 3:
        print("Usage: python gemini_api_creating.py <START_LINE> <END_LINE>")
        sys.exit(1)

    # קבלת הטווח משורת הפקודה
    start_line = int(sys.argv[1])
    end_line = int(sys.argv[2])

    process_song_titles(file_path, output_file, start_line, end_line)
