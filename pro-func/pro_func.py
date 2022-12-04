# -*- coding: utf-8 -*-
import os
import sys

# יבוא פונצקיה לקריאת מטאדאטה של קובץ
import music_tag

# יבוא פונצקיה לקריאת עץ תיקיות
from os.path import join, getsize

# יבוא פונצקיה עבור תצוגת האותיות העבריות
from bidi.algorithm import get_display


def pro_scanner(my_file, root):
    """
    פונקציית סורקת את המטאדאטה של השיר ומכניסה אותו למשתנה
    """
    try:
        if not my_file.endswith(".mp3"):
            return
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
            for my_file in files:
                pro_scanner(my_file, root)
                
        dict_list = target_dict.items()          
        for item in dict_list:
            file_name = str(item[0])
            art_name = str(item[1])
            if art_name.isdigit() or art_name.isalpha() or "&" in art_name or art_name == "" or "," in art_name or len(art_name.split()) >= 3:
                continue
            print("move " + file_name + " to " + art_name)


if __name__ == '__main__':
    main()
