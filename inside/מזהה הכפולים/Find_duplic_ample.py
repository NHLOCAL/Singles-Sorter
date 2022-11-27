# -*- coding: utf-8 -*-
import os
import sys

# פונקציה זו עוברת על הקוד הבינארי של הקובץ ושומרת מדגם שלו למשתנה
def duplic_scan(myfile):
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

# פונקציה זו עוברת על קבוצת קבצים שבתיקיה ומפעילה עליהם את הפונצקיה "duplic_scan"
def duplic_files(pathdir):
    my_dir = os.listdir(pathdir)    
    global files_list
    files_list = []
    global files_dict
    files_dict = {}
    global dict_to_del
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
        mut_list = files_list.copy()
        mut_list.remove(item[1])
        if item[1] in mut_list:
            file_num += 1
            dict_to_del[file_num] = item[0]
            print(str(file_num) + ": " + item[0])
            
    if file_num >= 1:
        select_file = input(">>>")   
        os.remove(pathdir + "\\" + dict_to_del[int(select_file)])
        print(dict_to_del[int(select_file)] + " deleted!")
    else:
        print("not found!")
            
def main():
    dir_path = str(sys.argv[1])
    if (dir_path != "") and (os.path.exists(dir_path)):
        duplic_files(sys.argv[1])


if __name__ == '__main__':
    main()
    os.system('pause')