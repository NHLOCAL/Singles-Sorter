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
    try:
        artist_file = music_tag.load_file(root + "\\" + my_file)
        art = artist_file['artist']
        print(str(art))
    except:
        pass

   
def main():
    dir_path = str(sys.argv[1])
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            #print("{}".format(root))
            #continue
            for my_file in files:
                pro_scanner(my_file, root)
    os.system('pause')

if __name__ == '__main__':
    main()