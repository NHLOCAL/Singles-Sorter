import random

# בקשת שם הקובץ מהמשתמש
file_name = input("הכנס את שם קובץ הטקסט (כולל סיומת .txt): ")

# קריאת תוכן הקובץ
with open(file_name, 'r', encoding='utf-8') as file:
    lines = file.readlines()

# ערבוב הרשימה
random.shuffle(lines)

# הצגת הרשימה המעורבבת
print("הרשימה המעורבבת היא:")
for line in lines:
    print(line.strip())

# אופציונלי - שמירת הרשימה המעורבבת בקובץ חדש
save_option = input("האם לשמור את הרשימה המעורבבת לקובץ חדש? (כן/לא): ")
if save_option.lower() == 'כן':
    output_file = input("הכנס שם לקובץ החדש (כולל סיומת .txt): ")
    with open(output_file, 'w', encoding='utf-8') as output:
        output.writelines(lines)
    print(f"הרשימה המעורבבת נשמרה בקובץ {output_file}")
