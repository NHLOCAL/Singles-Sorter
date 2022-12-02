# -*- coding: utf-8 -*-
import os
import sys
# יבוא פונצקיה לקריאת מטאדאטה של קובץ
import music_tag
# יבוא פונקציה לקריאת עץ תיקיות
from os.path import join, getsize
# יבוא פונצקיה עבור תצוגת האותיות העבריות
from bidi.algorithm import get_display
   


def pro_scanner(my_file, root):
# הפונקציה סורקת את המטאדאטה של השיר ומכניסה אותו למשתנה
    try:
        my_file = root + "\\" + my_file
        artist_file = music_tag.load_file(my_file)
        artist = artist_file['artist']
        target_dict[my_file] = artist
    except:
        pass

   
def main():
    global target_dict
    target_dict = {}
    dir_path = str(sys.argv[1])
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            #print("{}".format(root))
            #continue
            for my_file in files:
                pro_scanner(my_file, root)
        dict_list = target_dict.items()          
        for item in dict_list:
            file_name = str(item[0])
            art_name = str(item[1])
            if art_name.isdigit():
                continue
            print("move " + file_name + " to " + art_name)
            
    os.system('pause')

if __name__ == '__main__':
    main()