import random
from collections import Counter

# קריאת שמות מתוך קובץ עם סינון
def load_names_from_file(filename):
    with open(filename, 'r', encoding='utf-8') as file:
        names = file.readlines()
    
    filtered_names = []
    excluded_names = []
    for name in names:
        name = name.strip()
        parts = name.split()
        if len(parts) == 2:
            if "מקהלת" in name or "להקת" in name:
                excluded_names.append(name)
            else:
                filtered_names.append(name)
        else:
            excluded_names.append(name)  # שמות עם מילה אחת או שלוש ומעלה נכללים ברשימת ההחרגה
    
    return filtered_names, excluded_names

# חלוקת השמות לשמות פרטיים ושמות משפחה
def split_names(names):
    first_names = []
    last_names = []
    
    for name in names:
        parts = name.split()
        first_names.append(parts[0])
        last_names.append(parts[1])
    
    return first_names, last_names

# הסרת שמות נפוצים מרשימה או הגבלתם ל-5 מופעים
def limit_common_names(name_list, max_occurrences=5):
    name_counts = Counter(name_list)
    limited_list = []
    for name in name_list:
        if name_counts[name] > max_occurrences:
            limited_list.extend([name] * max_occurrences)
            name_counts[name] = 0  # הגבלת השם ל-5 מופעים בלבד
        elif name_counts[name] > 0:
            limited_list.append(name)
            name_counts[name] = 0  # הוספת השם רק פעם אחת לאחר שעברנו עליו
    return limited_list

# חיבור שמות פרטיים ושמות משפחה באופן אקראי
def combine_names_randomly(first_names, last_names):
    combined_names = []
    random.shuffle(first_names)
    random.shuffle(last_names)
    for first_name, last_name in zip(first_names, last_names):
        combined_names.append(f"{first_name} {last_name}")
    return combined_names

# שמירת השמות לקובץ
def save_names_to_file(names, filename):
    with open(filename, 'w', encoding='utf-8') as file:
        for name in names:
            file.write(f"{name}\n")

# פונקציה ראשית
def process_names(input_file, output_file):
    # שלב 1: קריאת השמות וסינון ראשוני
    names, excluded_names = load_names_from_file(input_file)
    
    # שלב 2: חלוקת השמות לשמות פרטיים ושמות משפחה
    first_names, last_names = split_names(names)
    
    # שלב 3: הגבלת שמות נפוצים ל-5 מופעים
    limited_first_names = limit_common_names(first_names)
    limited_last_names = limit_common_names(last_names)
    
    # שלב 4: חיבור שמות פרטיים ושמות משפחה באופן אקראי
    combined_names = combine_names_randomly(limited_first_names, limited_last_names)
    
    # שלב 5: הוספת השמות שהוחרגו מראש
    combined_names.extend(excluded_names)
    
    # שלב 6: שמירת הרשימה החדשה לקובץ
    save_names_to_file(combined_names, output_file)

# קריאה לפונקציה עם שם הקובץ שלך
process_names('not_found_singers.txt', 'processed_singers.txt')
