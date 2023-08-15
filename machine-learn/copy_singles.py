# coding: utf-8

import os
import shutil

def copy_single_song(my_dir, destination_folder):
    
    for main_folder in os.listdir(my_dir):
            
        singles_folder = os.path.join(my_dir, main_folder, 'סינגלים')
        
        if os.path.exists(singles_folder) and os.path.isdir(singles_folder):
            songs = [file for file in os.listdir(singles_folder) if os.path.isfile(os.path.join(singles_folder, file))]
            
            if songs and len(songs) >= 3:
                
                for song_to_copy in songs:               
                    source_path = os.path.join(singles_folder, song_to_copy)
                    destination_path = os.path.join(destination_folder, song_to_copy)
                
                    # Copy the selected song to the destination folder
                    shutil.copy(source_path, destination_path)
                    print(f"Copied {song_to_copy} to {destination_folder}")
        else:
            print(f"No 'סינגלים' folder found in {main_folder}")
            

root_dir = r'J:\שמע\מסודר מחדש'
destination_folder = r'C:\Users\משתמש\Documents\song_list'
list_of_dirs = [ name for name in os.listdir(root_dir) if os.path.isdir(os.path.join(root_dir, name)) ]
for my_dir in list_of_dirs:
    path_dir = os.path.join(root_dir, my_dir)
    print(path_dir)
    copy_single_song(path_dir, destination_folder)
