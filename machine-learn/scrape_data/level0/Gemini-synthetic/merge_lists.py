import os

def merge_text_files(directory, output_file):
    unique_lines = set()
    
    # איסוף כל קבצי הטקסט מהתיקיה
    for filename in os.listdir(directory):
        if filename.endswith(".txt"):
            file_path = os.path.join(directory, filename)
            
            with open(file_path, 'r', encoding='utf-8') as file:
                for line in file:
                    clean_line = line.strip()  # הסרת רווחים והורדת שורות ריקות
                    if clean_line:  # בדיקה אם השורה אינה ריקה
                        unique_lines.add(clean_line)  # הוספת השורה לקבוצה (כדי למנוע כפילויות)

    # כתיבה לקובץ פלט
    with open(output_file, 'w', encoding='utf-8') as output:
        for line in sorted(unique_lines):  # אפשר למיין את השורות אם רוצים
            output.write(line + '\n')

# דוגמה להפעלה:
directory_path = input('>>>')  # החלף בנתיב התיקיה שלך
output_file_path = 'merged_output.txt'  # שם הקובץ לפלט

merge_text_files(directory_path, output_file_path)
