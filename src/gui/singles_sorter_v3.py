# -*- coding: utf-8 -*-
import os
import argparse
from shutil import copy, move
from music_tag import load_file
from jibrish_to_hebrew import fix_jibrish
import csv
from check_name import check_exact_name
import json
import datetime

class MusicSorter:

    VERSION = '12.9.4'

    def __init__(self, source_dir, target_dir, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, main_folder_only=False, progress_callback=None):
        self.unusual_list = ["סינגלים", "סינגל", "אבגדהוזחטיכלמנסעפצקרשתךםןץ", "אמן לא ידוע", "טוב", "לא ידוע", "תודה לך ה"]
        self.substrings_to_remove = [" -מייל מיוזיק", " - ציצו במייל", "-חדשות המוזיקה", " - חדשות המוזיקה", " - ציצו", " מוזיקה מכל הלב", " - מייל מיוזיק"]
        self.source_dir = source_dir
        self.target_dir = target_dir
        self.copy_mode = copy_mode
        self.abc_sort = abc_sort
        self.exist_only = exist_only
        self.singles_folder = singles_folder
        self.main_folder_only = main_folder_only
        self.log_files = []
        self.operating_details = [source_dir, target_dir, copy_mode, abc_sort, exist_only, singles_folder, main_folder_only]
        self.progress_callback = progress_callback
        self.singer_list =  self.list_from_csv()


    def progress_display(self, len_amount):
        for len_item in range(1, len_amount + 1):
            show_len = len_item * 100 // len_amount
            yield show_len


    def check_errors(self):
        """
        Checks for potential errors related to source and target directories.

        Args:
            source_dir (str): The path to the source directory.
            target_dir (str): The path to the target directory.

        Raises:
            FileNotFoundError: If the source or target directory does not exist.
            PermissionError: If the script does not have write access to the target directory.
            ValueError: If the source and target directories are the same.
        """
        if not os.path.exists(self.source_dir):
            raise FileNotFoundError("תיקיית המקור לא נמצאה")

        if not os.path.exists(self.target_dir):
            raise FileNotFoundError("תיקיית היעד לא נמצאה")

        if not os.access(self.target_dir, os.W_OK):
            raise PermissionError("אין הרשאת כתיבה לתיקיית היעד")

        if os.path.samefile(self.source_dir, self.target_dir):
            raise ValueError("תיקיית המקור ותיקיית היעד לא יכולות להיות זהות")
        
        if not os.listdir(self.source_dir):
            raise ValueError("תיקיית המקור ריקה")


    def clean_filename(self, filename):
        
        for substring in self.substrings_to_remove:
            filename = filename.replace(substring, "")
            
        return filename

    def clean_names(self):
        if self.main_folder_only is False:
            for root, _, files in os.walk(self.source_dir):
                for my_file in files:
                    if my_file.lower().endswith((".mp3", ".wma", ".wav")):
                        old_file_path = os.path.join(root, my_file)
                        new_file_name = self.clean_filename(my_file)
                        new_file_path = os.path.join(root, new_file_name)
                        os.rename(old_file_path, new_file_path)
                        
        elif self.main_folder_only:
            for my_file in os.listdir(self.source_dir):
                file_path = os.path.join(self.source_dir, my_file)
                if os.path.isfile(file_path) and my_file.lower().endswith((".mp3", ".wma", ".wav")):
                    new_file_name = self.clean_filename(my_file)
                    new_file_path = os.path.join(self.source_dir, new_file_name)
                    os.rename(file_path, new_file_path)


    def scan_dir(self):
        """
        Main function of the program. Scans the specified directory and creates a list of files for copying.
        At the end of the process, it copies them if a target directory parameter is provided.
        
        Parameters:
            source_dir = Directory path to scan
            target_dir = Target directory path for transfer
            copy_mode = Enable copy mode (default is move)
            abc_sort = Sort folders alphabetically
            exist_only = Transfer to existing folders only
            singles_folder = Create an internal "singles" folder
            main_folder_only = Sort only the main folder
            Defined by True or False.
        
        Result:
            Prints the list of artists that appear in the song metadata and copies them to the target.
        """

        # בדיקת שגיאות בארגומנטים של המשתמש
        self.check_errors()

        # סריקת עץ התיקיות או התיקיה הראשית בהתאם לבחירת המשתמש והכנסת שם הקבצים ושם האמן שלהם לרשימה
        info_list = []  
        if self.main_folder_only is False:
            for root, _, files in os.walk(self.source_dir):
                for my_file in files:
                    file_path = os.path.join(root, my_file)
                    if my_file.lower().endswith((".mp3",".wma", ".wav")):
                        artist = self.artist_from_song(file_path)
                        if artist: info_list.append((file_path, artist))

        # סריקת התיקיה הראשית בלבד ללא תיקיות פנימיות
        elif self.main_folder_only:
            for my_file in os.listdir(self.source_dir):
                file_path = os.path.join(self.source_dir, my_file)
                if os.path.isfile(file_path):
                    if my_file.lower().endswith((".mp3",".wma", ".wav")):
                        artist = self.artist_from_song(file_path)
                        if artist: info_list.append((file_path, artist))

        len_dir = len(info_list)
        progress_generator = self.progress_display(len_dir)

        # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
        for file_path, artist in info_list:   
            show_len = next(progress_generator)
            print(f"{show_len}% completed",end='\r')
            if self.progress_callback:  # Call the callback with progress
                self.progress_callback(show_len)
                        
            # הגדרת משתנה עבור תיקית יעד בהתאם להתאמות האישיות של המשתמש
            if self.singles_folder and self.abc_sort:
                main_target_path = os.path.join(self.target_dir, artist[0], artist)
                target_path = os.path.join(self.target_dir, artist[0], artist, "סינגלים")
            elif self.singles_folder:
                main_target_path = os.path.join(self.target_dir, artist)
                target_path = os.path.join(self.target_dir, artist, "סינגלים")
            elif self.abc_sort:
                main_target_path = os.path.join(self.target_dir, artist[0], artist)
                target_path = os.path.join(self.target_dir, artist[0], artist)
            else:
                main_target_path = os.path.join(self.target_dir, artist)
                target_path = os.path.join(self.target_dir, artist)
            
            # יצירת תיקית יעד בתנאים מסויימים
            if self.exist_only is False:
                if not os.path.isdir(target_path):
                    try: 
                        os.makedirs(target_path)
                    except Exception as e:
                        print(f"Error creating directory {target_path}: {e}")
                    
            elif self.exist_only and self.singles_folder:
                if os.path.isdir(main_target_path) and not os.path.isdir(target_path):
                    try: 
                        os.makedirs(target_path)
                    except Exception as e:
                        print(f"Error creating directory {target_path}: {e}")
            else:
                pass


            # העברה או העתקה בהתאם להגדרות המשתמש
            if self.copy_mode and os.path.isdir(target_path):
                try:
                    copy(file_path, target_path)
                    self.log_files.append((file_path, target_path))
                    print(f"Copied {file_path} to {target_path}")
                except Exception as e:
                    print(f"Failed to copy {file_path} to {target_path}: {e}")
            elif os.path.isdir(target_path):
                try:
                    move(file_path, target_path)
                    self.log_files.append((file_path, target_path))
                    print(f"Moved {file_path} to {target_path}")
                except Exception as e:
                    print(f"Failed to move {file_path} to {target_path}: {e}")

        return


    def list_from_csv(self):
        # יבוא רשימת זמרים מקובץ csv
        # Construct the path to the CSV file
        csv_path = os.path.abspath("app/singer-list.csv")
        
        with open(csv_path, 'r', encoding='utf-8') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]
        
        if os.path.isfile("app/personal-singer-list.csv"):
            with open("app/personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)

        return singer_list


    def artist_from_song(self, my_file):
        """
        הפונקציה בודקת את שם אמן הקובץ בשם הקובץ לפי מסד נתונים ומכניסה את שם האמן למשתנה
        אם השם לא קיים היא סורקת את המטאדאטה של השיר ומכניסה את שם האמן למשתנה
        
        תנאים:
            my_file (str) - שם הקובץ שנסרק
        
        תוצאה:
            ערך המכיל את שם אמן הקובץ
        """
    
        # מעבר על רשימת הזמרים בדאטה וחיפוש שלהם בשם הקובץ
        
        # קבלת שם הקובץ ללא נתיב מלא
        split_file = os.path.split(my_file)[1]
        
        # הסרת תווים מטעים בשם הקובץ
        split_file = split_file.replace('_', ' ')
        split_file = split_file.replace('-', ' ')
        
        
        # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בשם השיר
        for source_name, target_name in self.singer_list:
            if source_name in split_file:
            
                # בדיקת דיוק שם הקובץ
                exact = check_exact_name(split_file, source_name)
                
                if exact:
                    artist = target_name
                    return artist

        # אם שם הקובץ לא נמצא יתבצע חיפוש במטאדאטה של הקובץ
        try:
            # טעינת מטאדאטה של השיר
            metadata_file = load_file(my_file)
            # קבלת אמן מטאדאטה של השיר
            artist = metadata_file['artist']
            artist = artist.value

            if artist:
                # המרת שם האמן אם הוא פגום
                artist = fix_jibrish(artist, "heb")
                
                # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בתגית האמן
                for source_name, target_name in self.singer_list:
                    if source_name in artist:
                        """
                        # בדיקת דיוק שם הקובץ
                        exact = check_exact_name(artist, source_name)  
                        """
                        artist = target_name
                        return artist
                    
                # הפעלת פונקציה המבצעת בדיקות על שם האמן
                check_answer = self.check_artist(artist)
                if check_answer == False:
                    return
                    
                return artist
            
            # אם לא נמצא שם אמן תקין יתבצע חיפוש בכותרת הקובץ
            else:
                # קבלת כותרת השיר
                title = metadata_file['title']
                title = title.value
                
                if title:
                    # המרת שם האמן אם הוא פגום
                    title = fix_jibrish(title, "heb")
                    
                    # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בתגית האמן
                    for source_name, target_name in self.singer_list:
                        if source_name in title:
                            # בדיקת דיוק שם הקובץ
                            exact = check_exact_name(title, source_name)
                            
                            if exact:
                                artist = target_name
                                return artist
        except UnicodeDecodeError as e:
            print(f"Error decoding metadata in file {my_file}: {e}")
            return
        except Exception as e:
            print(f"An unexpected error occurred with file {my_file}: {e}")
            return


    def check_artist(self, artist):
        # החזרת שקר אם שם האמן קיים ברשימת יוצאי הדופן     
        if artist in self.unusual_list:
            return False
        
        # בדיקה אם המחרוזת אינה ארוכה מידי
        if len(artist.split()) >= 4 or len(artist.split()) <= 0:
            return False
        
        # בדיקה אם המחרוזת מכילה תוים תקינים בלבד
        if all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳ " for c in artist):
            return True
        else:
            return False

    def log_to_file(self):
        now = datetime.datetime.now()
        log_filename = f"log_{now.strftime('%Y%m%d_%H%M%S')}.json"
        log_directory = "log"
        
        log_data = {
            "operation_time": now.strftime('%d/%m/%Y %H:%M:%S'),
            "source_directory": self.operating_details[0],
            "target_directory": self.operating_details[1],
            "copy_mode": self.operating_details[2],
            "abc_sort": self.operating_details[3],
            "exist_only": self.operating_details[4],
            "singles_folder": self.operating_details[5],
            "main_folder_only": self.operating_details[6],
            "files": [{"old_path": old_path, "new_path": new_path} for old_path, new_path in self.log_files]
        }
        
        # יצירת התיקיה אם היא לא קיימת
        if not os.path.exists(log_directory):
            os.makedirs(log_directory)
        
        log_path = os.path.join(log_directory, log_filename)
        with open(log_path, 'w', encoding='utf-8') as log_file:
            json.dump(log_data, log_file, ensure_ascii=False, indent=4)

    def load_from_log(self, log_filename):
        with open(log_filename, 'r', encoding='utf-8') as log_file:
            log_data = json.load(log_file)
        return log_data
    


def main():
    parser = argparse.ArgumentParser(description=f"Singles Sorter {MusicSorter.VERSION} - Scan and organize music files into folders by artist using advanced automation.")
    parser.add_argument('source_dir', help="Path to the source directory")
    parser.add_argument('target_dir', help="Path to the target directory", nargs='?')
    parser.add_argument('-c', '--copy_mode', help="Enable copy mode (default is move mode)", action='store_true')
    parser.add_argument('-a', '--abc_sort', help="Sort folders alphabetically (default: False)", action='store_true')
    parser.add_argument('-e', '--exist_only', help="Transfer to existing folders only (default: False)", action='store_true')
    parser.add_argument('-n', '--no_singles_folder', help="Do not create an internal 'singles' folder", action='store_false', dest='singles_folder', default=True)
    parser.add_argument('-m', '--main_folder_only', help="Sort only the main folder (default: False)", action='store_true')

    args = parser.parse_args()

    try:
        sorter = MusicSorter(args.source_dir, args.target_dir, args.copy_mode, args.abc_sort, args.exist_only, args.singles_folder, args.main_folder_only)
        sorter.clean_names()
        sorter.scan_dir()
        sorter.log_to_file()
    except Exception as e:
        print("Error: {}".format(e))

if __name__ == '__main__':
    main()
