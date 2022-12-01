# -*- coding: utf-8 -*-
import os
import sys
from bidi.algorithm import get_display

def duplic_scan(myfile):
# פונקציה זו עוברת על הקוד הבינארי של הקובץ ושומרת מדגם שלו למשתנה
    filename = myfile
    global break_num
    global id_file
    break_num = 0
    id_file = "id:"
    with open(myfile, 'rb') as input_file:
        for line in input_file:
            break_num += 1
            if ((break_num > 145) and (break_num < 1985)) or ((break_num > 2778) and (break_num < 4900)):
                continue
            line_to_str = str(line)
            id_file += line_to_str[7:83:19]
            if 5530 == break_num:
                break
            
    return(id_file)


def duplic_files(pathdir):
# פונקציה זו עוברת על קבוצת קבצים שבתיקיה ומפעילה עליהם את הפונקציה duplic_scan
    my_dir = os.listdir(pathdir)
    files_list = []
    files_dict = {}
    dict_to_del = {}
    
    # מעבר על רשימת הקבצים והוספת דגימה של הקידוד שלהם למשתנה
    for file in my_dir:
        # בדיקה אם שם הקובץ הפנימי מכיל סיומות ספציפיות
        if not (".mp3" in file) or (".wav" in file) or (".wma" in file):
            continue
        my_file = duplic_scan(pathdir + "\\" + file)
        files_dict[file] = id_file
        files_list.append(id_file)
        
    dict_list = files_dict.items()
    global file_num
    file_num = 0
    
    for item in dict_list:
        if files_list.count(item[1]) == 2:
            file_num += 1
            dict_to_del[file_num] = item[0]
            print(str(file_num) + ": " + get_display(item[0]))
            
    if file_num >= 1:
        select_file = input(get_display("\nהכנס מספר רצוי בכדי למחוק קובץ\nניתן להכניס כמה ספרות בהפרדה של פסיק ביניהם") + "\n>>>")
        select_file = select_file.split(',')
        try:
            for del_item in select_file:
                os.remove(pathdir + "\\" + dict_to_del[int(del_item)])
                print(get_display(dict_to_del[int(del_item)] + "  -- נמחק!"))
        except:
            print(get_display("עליך להכניס מספר בכדי למחוק קובץ מסוים!"))
    else:
        print(get_display("לא נמצא דבר!"))
            
def main():
    dir_path = str(sys.argv[1])
    if (dir_path != "") and (os.path.exists(dir_path)):
        print("\n>>" + get_display(sys.argv[1]))
        duplic_files(sys.argv[1])
        os.system('timeout 1')
        os.system('cls')

if __name__ == '__main__':
    main()