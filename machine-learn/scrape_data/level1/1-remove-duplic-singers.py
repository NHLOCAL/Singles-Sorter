import csv

def remove_duplicates(input_file, output_file):
    # קריאת הקובץ ושמירת השורות הייחודיות תוך שמירה על הסדר
    unique_rows = []
    seen = set()
    with open(input_file, 'r', newline='', encoding='utf-8') as f:
        reader = csv.reader(f)
        for row in reader:
            row_tuple = tuple(row)
            if row_tuple not in seen:
                seen.add(row_tuple)
                unique_rows.append(row)
    
    # כתיבת השורות הייחודיות חזרה לקובץ
    with open(output_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerows(unique_rows)

# שימוש בפונקציה
input_file = 'singers_list.csv'
output_file = 'singers_list2.csv'
remove_duplicates(input_file, output_file)
print("הוסרו שורות כפולות בהצלחה!")