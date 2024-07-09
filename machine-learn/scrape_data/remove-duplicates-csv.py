import csv

def remove_duplicates(input_file, output_file):
    # קריאת הקובץ לתוך סט
    with open(input_file, 'r', newline='', encoding='utf-8') as f:
        reader = csv.reader(f)
        unique_rows = set(tuple(row) for row in reader)
    
    # כתיבת הסט חזרה לקובץ
    with open(output_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerows(unique_rows)

# שימוש בפונקציה
input_file = 'singers_list.csv'
output_file = 'singers_list2.csv'
remove_duplicates(input_file, output_file)
print("הוסרו שורות כפולות בהצלחה!")
