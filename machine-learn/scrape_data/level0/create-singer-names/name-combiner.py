import random

def read_names(filename):
    with open(filename, 'r', encoding='utf-8') as file:
        return [line.strip() for line in file if line.strip()]

def remove_duplicates(names):
    return list(dict.fromkeys(names))

def write_names(names, filename):
    with open(filename, 'w', encoding='utf-8') as file:
        for name in names:
            file.write(name + '\n')

def combine_all_names(private_names, family_names):
    combined_names = []
    
    # שימוש בכל השמות הפרטיים
    for private_name in private_names:
        combined_names.append(f"{private_name} {random.choice(family_names)}")
    
    # הוספת שמות נוספים עם שמות משפחה שטרם נוצלו
    remaining_family_names = family_names.copy()
    random.shuffle(remaining_family_names)
    
    for family_name in remaining_family_names:
        if not any(name.endswith(family_name) for name in combined_names):
            combined_names.append(f"{random.choice(private_names)} {family_name}")
    
    random.shuffle(combined_names)
    return combined_names

def write_combined_names(combined_names, output_filename):
    with open(output_filename, 'w', encoding='utf-8') as file:
        for name in combined_names:
            file.write(name + '\n')

# קריאת השמות מהקבצים
private_names = read_names('private-names.txt')
family_names = read_names('family-names.txt')

# הסרת כפילויות מהשמות
private_names = remove_duplicates(private_names)
family_names = remove_duplicates(family_names)

# כתיבת השמות ללא כפילויות בחזרה לקבצים
write_names(private_names, 'private-names.txt')
write_names(family_names, 'family-names.txt')

# יצירת שמות מלאים
full_names = combine_all_names(private_names, family_names)

# כתיבת השמות המלאים לקובץ חדש
write_combined_names(full_names, 'full-names.txt')

print(f"נוצרו {len(full_names)} שמות מלאים ונשמרו בקובץ 'full-names.txt'")
print(f"מספר שמות פרטיים מקוריים (לאחר הסרת כפילויות): {len(private_names)}")
print(f"מספר שמות משפחה מקוריים (לאחר הסרת כפילויות): {len(family_names)}")