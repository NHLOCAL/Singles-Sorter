def read_list_from_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as file:
        return set(file.read().splitlines())

def find_intersection(file1_path, file2_path):
    list1 = read_list_from_file(file1_path)
    list2 = read_list_from_file(file2_path)
    
    intersection = list1.intersection(list2)
    
    return sorted(intersection)

def write_intersection_to_file(intersection, output_file_path):
    with open(output_file_path, 'w', encoding='utf-8') as file:
        for item in intersection:
            file.write(f"{item}\n")

# שימוש בפונקציות
file1_path = 'singers1.txt'
file2_path = 'singers2.txt'
output_file_path = 'singers.txt'

intersection = find_intersection(file1_path, file2_path)
write_intersection_to_file(intersection, output_file_path)

print(f"נוצרה רשימה חדשה עם {len(intersection)} ערכים משותפים.")
print(f"הרשימה נשמרה בקובץ: {output_file_path}")
