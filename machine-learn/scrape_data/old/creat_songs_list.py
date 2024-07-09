# coding: utf-8

import os
import shutil


def read_single_songs(my_dir):
    
    songs_set = []
    
    for main_folder in os.listdir(my_dir):
            
        singles_folder = os.path.join(my_dir, main_folder, 'סינגלים')
        
        if os.path.exists(singles_folder) and os.path.isdir(singles_folder):
            songs = [file for file in os.listdir(singles_folder) if os.path.isfile(os.path.join(singles_folder, file))]
            
            if songs and len(songs) >= 3:
                
                for song in songs:
                    songs_set.append(song)
    
    return songs_set
    
    

root_dir = r'J:\שמע\כל המוזיקה'

# בנה רשימה של תיקיות משנה
list_of_dirs = [ name for name in os.listdir(root_dir) if os.path.isdir(os.path.join(root_dir, name)) ]

# הגדר קבוצה ריקה
songs_set = set()

# הרץ את הפונקציה וקבל את רשימת הסינגלים
for my_dir in list_of_dirs:
    path_dir = os.path.join(root_dir, my_dir)
    songs = read_single_songs(path_dir)
    songs_without_ext = [os.path.splitext(song)[0] for song in songs]
    songs_set.update(songs_without_ext)
    




# קבל את רשימת השירים מתוך קובץ
with open('songs_list.txt', mode='r', newline='', encoding='utf-8') as file:
    content = file.readlines()    
    songs_list = [os.path.splitext(song)[0].strip() for song in content]

# הוסף את התוכן הישן לתוכן החדש
old_songs_set = len(songs_list)
songs_set.update(songs_list)
print(len(songs_set))
print('----')
print(len(songs_set) - old_songs_set)

# הוסף תוכן לקובץ
with open('songs_list.txt', mode='w', newline='', encoding='utf-8') as file:
    for song in songs_set:
        file.write(song + "\n")