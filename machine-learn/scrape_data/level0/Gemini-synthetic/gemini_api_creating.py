import os
import google.generativeai as genai

# Configure the API key from environment variables (GitHub Actions Secret)
genai.configure(api_key=os.getenv("GEMINI_API_KEY"))

# Define the configuration for the generation model
generation_config = {
    "temperature": 0.9,
    "top_p": 0.95,
    "top_k": 40,
    "max_output_tokens": 8192,
    "response_mime_type": "text/plain"
}

# Load the model (latest version of Gemini Flash)
model = genai.GenerativeModel(
    model_name="gemini-1.5-flash-002",
    generation_config=generation_config,
)

# Define the task instruction for the model
system_instruction = """
# יצירת וריאציות של כותרות שירים בעברית

**מטרת המשימה:**  ליצור כותרות שירים מגוונות בעברית, תוך שימוש במילה  "SINGER" כמחזיק מקום לשם הזמר/המקהלה.  הדגש הוא על יצירת וריאציות שונות של אותן כותרות, ולא על כתיבת כותרות חדשות לחלוטין.

**כללי עבודה:**

* **שימוש ב- "SINGER":**  השתמש אך ורק במילה "SINGER" כמציין מיקום לשם הזמר או מקהלה.  **אל תחליף אותה בשם זמר אמיתי.**

* **בסיס הנתונים:**  תשתמש ברשימת הכותרות הבאה כבסיס ליצירת הווריאציות.  **אסור להוסיף כותרות חדשות שלא מופיעות ברשימה זו.**

* **ווריאציות:**  צור וריאציות שונות של כל כותרת ברשימה, על ידי:
    * **שינוי סדר מילים:**  שינה את סדר המילים בכותרת, תוך שמירה על המשמעות.
    * **הוספת פרטים:**  הוסף מידע רלוונטי, כמו סגנון מוזיקלי, שם אלבום, או פרטים נוספים הקשורים לשיר.
    * **הסרת פרטים:**  הסר מידע שאינו חיוני מהכותרת המקורית.
    * **שינוי ניסוח:**  שנה את הניסוח של הכותרת, תוך שמירה על המשמעות המקורית.

* **איכות הכותרות:**  הקפד על כתיבה תקינה, נכונה דקדוקית ובשפה ברורה.  **אל תיצור כותרות חסרות משמעות או גרועות.**

* **תפוקת המערכת:**  תפוקת המערכת תהיה רשימה של כותרות שירים חדשות, שהן וריאציות על הכותרות המקוריות.  **אל תוסיף הערות, מספור, או כל מידע נוסף מלבד כותרות השירים.**
"""

# Function to read chunks of the file (100 lines at a time)
def read_file_in_chunks(file_path, chunk_size=100):
    with open(file_path, 'r', encoding='utf-8') as file:
        while True:
            lines = [file.readline().strip() for _ in range(chunk_size)]
            if not any(lines):
                break
            yield [line for line in lines if line]

# Process song titles and save results to a file
def process_song_titles(file_path, output_file):
    chat_session = model.start_chat(
        history=[
            {"role": "system", "content": system_instruction}
        ]
    )
    
    with open(output_file, 'w', encoding='utf-8') as f:
        for chunk in read_file_in_chunks(file_path):
            input_text = "\n".join(chunk)
            prompt = f"להלן 100 כותרות שירים:\n{input_text}\n"
            
            # Send the request to the model
            response = chat_session.send_message(prompt)
            
            # Write the response to the file and print it for feedback
            f.write(response.text + "\n\n")
            print(response.text)

# Example usage
if __name__ == "__main__":
    file_path = "machine-learn/scrape_data/level0/Gemini-synthetic/songs_data.txt"  # Input file with song titles
    output_file = "song_variations_output.txt"  # Output file to store variations
    process_song_titles(file_path, output_file)
