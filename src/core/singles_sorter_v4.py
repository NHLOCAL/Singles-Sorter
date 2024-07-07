# -*- coding: utf-8 -*-
__VERSION__ = '13.2'

import os
import re
import sys
import argparse
from shutil import copy, move
import shutil
from music_tag import load_file
from jibrish_to_hebrew import fix_jibrish
import csv
from check_name import check_exact_name
import logging
import datetime

class MusicSorter:

    def __init__(self, source_dir, target_dir, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, main_folder_only=False, duet_mode=False, progress_callback=None, log_level=logging.INFO):
        self.unusual_list = ["סינגלים", "סינגל", "אבגדהוזחטיכלמנסעפצקרשתךםןץ", "אמן לא ידוע", "טוב", "לא ידוע", "תודה לך ה"]
        self.substrings_to_remove = [" -מייל מיוזיק", " - ציצו במייל", "-חדשות המוזיקה", " - חדשות המוזיקה", " - ציצו", " מוזיקה מכל הלב", " - מייל מיוזיק"]
        self.source_dir = source_dir
        self.target_dir = target_dir
        self.copy_mode = copy_mode
        self.abc_sort = abc_sort
        self.exist_only = exist_only
        self.singles_folder = singles_folder
        self.main_folder_only = main_folder_only
        self.duet_mode = duet_mode
        self.progress_callback = progress_callback
        self.log_files = []
        self.operating_details = [source_dir, target_dir, copy_mode, abc_sort, exist_only, singles_folder, main_folder_only, duet_mode]
        self.singer_list =  self.list_from_csv()

        # Set up logging
        self.setup_logging(log_level)


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



    def sanitize_filename(self, filename):
        # Remove invalid characters and replace spaces
        filename = re.sub(r'[<>:"/\\|?*]', '', filename)
        
        # Truncate filename if it's too long (max 255 characters)
        return filename[:255]

    def analyze_album(self, folder_path):
        """
        Analyzes a folder to determine if it's an album and if it should be processed or ignored.
        
        Args:
        folder_path (str): Path to the folder to analyze
        
        Returns:
        tuple: (is_album, should_process, album_name, artist_name)
            is_album (bool): True if the folder is considered an album
            should_process (bool): True if the album should be processed (not ignored)
            album_name (str): Name of the album (if applicable)
            artist_name (str): Name of the artist (if applicable)
        """
        try:
            # Check if it's a directory
            if not os.path.isdir(folder_path):
                self.logger.debug(f"{folder_path} is not a directory")
                return False, False, None, None

            # Get all audio files in the folder
            audio_files = [f for f in os.listdir(folder_path) if f.lower().endswith((".mp3", ".wma", ".wav"))]
            
            # Check if there are enough files to be considered an album
            if len(audio_files) < 4:
                self.logger.debug(f"{folder_path} doesn't have enough audio files to be an album")
                return False, False, None, None
            
            # Check if there are any subdirectories (albums typically don't have subdirectories)
            if any(os.path.isdir(os.path.join(folder_path, item)) for item in os.listdir(folder_path)):
                self.logger.debug(f"{folder_path} contains subdirectories, not considered an album")
                return False, False, None, None

            # Analyze metadata of the files
            album_names = []
            artists = set()
            for file in audio_files:
                file_path = os.path.join(folder_path, file)
                try:
                    metadata = load_file(file_path)
                    album = metadata.get('album')
                    artist = metadata.get('artist')
                    if album and artist:
                        album_names.append(album.value)
                        artists.add(artist.value)
                except Exception as e:
                    self.logger.error(f"Error reading metadata from {file_path}: {e}")

            # Check if we have valid metadata
            if not album_names:
                self.logger.debug(f"No valid album metadata found in {folder_path}")
                return False, False, None, None

            # Determine the most common album name
            most_common_album = max(set(album_names), key=album_names.count)
            album_name_consistency = album_names.count(most_common_album) / len(album_names)

            # Determine if it's an album and if it should be processed
            is_album = False
            should_process = False
            if len(set(album_names)) == 1:  # All files have the same album name
                is_album = True
                should_process = len(artists) == 1  # Process only if there's a single artist
            elif album_name_consistency >= 0.7:  # At least 70% of files have the same album name
                if len(artists) == 1:
                    is_album = True
                    should_process = False
                elif len(artists) >= 2:
                    is_album  = False
                    should_process = False

            # Log the decision
            if is_album:
                if should_process:
                    self.logger.info(f"Album detected and will be processed: {folder_path}")
                else:
                    self.logger.info(f"Album detected but will be ignored due to inconsistent artists: {folder_path}")
            else:
                self.logger.debug(f"Not considered an album: {folder_path}")

            return is_album, should_process, most_common_album, list(artists)[0] if len(artists) == 1 else None

        except Exception as e:
            self.logger.error(f"Error in analyze_album for {folder_path}: {e}")
            return False, False, None, None

    def handle_album_transfer(self, album_path, album_name, artist_name):
        try:
            if not album_name or not artist_name:
                self.logger.warning(f"Missing album name or artist name for {album_path}")
                return

            # Sanitize the album name for use in file paths
            safe_album_name = self.sanitize_filename(album_name)

            # Determine the target path
            if self.abc_sort:
                target_path = os.path.join(self.target_dir, artist_name[0], artist_name)
            else:
                target_path = os.path.join(self.target_dir, artist_name)
            
            album_target_path = os.path.join(target_path, safe_album_name)

            # Create the target directory if it doesn't exist and we're allowed to
            if not self.exist_only or (self.exist_only and os.path.isdir(target_path)):
                try:
                    os.makedirs(album_target_path, exist_ok=True)
                except Exception as e:
                    self.logger.error(f"Failed in folder creating {album_target_path}: {str(e)}")
                    return

                # Transfer the entire folder
                for item in os.listdir(album_path):
                    source_item = os.path.join(album_path, item)
                    target_item = os.path.join(album_target_path, self.sanitize_filename(item))
                    
                    if self.copy_mode:
                        if os.path.isfile(source_item):
                            try:
                                shutil.copy2(source_item, target_item)
                                self.logger.info(f"Copied {source_item} to {target_item}")
                            except Exception as e:
                                self.logger.error(f"Failed to copy {source_item}: {str(e)}")
                        else:
                            try:
                                shutil.copytree(source_item, target_item)
                                self.logger.info(f"Copied directory {source_item} to {target_item}")
                            except Exception as e:
                                self.logger.error(f"Failed to copy directory {source_item}: {str(e)}")
                    else:
                        try:
                            shutil.move(source_item, target_item)
                            self.logger.info(f"Moved {source_item} to {target_item}")
                        except Exception as e:
                            self.logger.error(f"Failed to move {source_item}: {str(e)}")

                if not self.copy_mode:
                    try:
                        os.rmdir(album_path)
                        self.logger.info(f"Removed original album folder: {album_path}")
                    except Exception as e:
                        self.logger.error(f"Failed to remove original album folder {album_path}: {str(e)}")
            else:
                self.logger.info(f"Skipped album transfer: {album_path} (target folder doesn't exist)")
        except Exception as e:
            self.logger.error(f"Error in handle_album_transfer for {album_path}: {e}")

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

        self.logger.info("Starting directory scan")

        self.check_errors()

        info_list = []
        if not self.main_folder_only:
            for root, _, files in os.walk(self.source_dir):
                try:
                    is_album, should_process, album_name, artist_name = self.analyze_album(root)
                    if is_album:
                        if should_process:
                            self.handle_album_transfer(root, album_name, artist_name)
                        continue  # Skip individual file processing for albums
                    
                    # Process individual files if not an album
                    for my_file in files:
                        file_path = os.path.join(root, my_file)
                        if my_file.lower().endswith((".mp3", ".wma", ".wav")):
                            artists = self.artists_from_song(file_path)
                            if artists:
                                info_list.append((file_path, artists))
                except Exception as e:
                    self.logger.error(f"Error processing directory {root}: {e}")
        else:
            for item in os.listdir(self.source_dir):
                try:
                    item_path = os.path.join(self.source_dir, item)
                    if os.path.isdir(item_path):
                        is_album, should_process, album_name, artist_name = self.analyze_album(item_path)
                        if is_album:
                            if should_process:
                                self.handle_album_transfer(item_path, album_name, artist_name)
                            continue  # Skip individual file processing for albums
                    elif item.lower().endswith((".mp3", ".wma", ".wav")):
                        artists = self.artists_from_song(item_path)
                        if artists:
                            info_list.append((item_path, artists))
                except Exception as e:
                    self.logger.error(f"Error processing item {item}: {e}")
                        
        len_dir = len(info_list)
        progress_generator = self.progress_display(len_dir)

        for file_path, artists in info_list:
            show_len = next(progress_generator)
            self.logger.debug(f"{show_len}% completed")
            if self.progress_callback:
                self.progress_callback(show_len)

            if not self.duet_mode:
                artists = [artists[0]]  # Only use the first artist if duet_mode is False
            
            for artist in artists:
                target_path = self.get_target_path(artist)

                if not self.exist_only or (self.exist_only and os.path.isdir(os.path.dirname(target_path))):
                    try:
                        os.makedirs(target_path, exist_ok=True)
                    except Exception as e:               
                        self.logger.error(f"Failed in folder creating {target_path}: {str(e)}")

                if os.path.isdir(target_path):
                    try:
                        if self.duet_mode and len(artists) > 1:
                            copy(file_path, target_path)
                            self.logger.info(f"Copied {file_path} to {target_path}")
                        elif self.copy_mode:
                            copy(file_path, target_path)
                            self.logger.info(f"Copied {file_path} to {target_path}")
                        else:
                            move(file_path, target_path)
                            self.logger.info(f"Moved {file_path} to {target_path}")
                    except Exception as e:
                        self.logger.error(f"Failed to process {file_path}: {str(e)}")

            # If it's a duet and we've copied to all singers' folders, remove the original
            if self.duet_mode and len(artists) > 1 and not self.copy_mode:
                try:
                    os.remove(file_path)
                    self.logger.info(f"Removed original file: {file_path}")
                except Exception as e:
                    self.logger.error(f"Failed to remove original file {file_path}: {str(e)}")

        self.logger.info("Directory scan completed")

    def get_target_path(self, artist):
        if self.singles_folder and self.abc_sort:
            return os.path.join(self.target_dir, artist[0], artist, "סינגלים")
        elif self.singles_folder:
            return os.path.join(self.target_dir, artist, "סינגלים")
        elif self.abc_sort:
            return os.path.join(self.target_dir, artist[0], artist)
        else:
            return os.path.join(self.target_dir, artist)

    def is_cli_mode(self):
        try:
            return sys.stdin is not None and sys.stdin.isatty()
        except AttributeError:
            return False

    def list_from_csv(self):
        # יבוא רשימת זמרים מקובץ csv
        # Construct the path to the CSV file

        # אם הקוד רץ כקובץ מקומפל
        if getattr(sys, 'frozen', False) and self.is_cli_mode():
            # מצב CLI וגם מקומפל
            csv_path = os.path.join(sys._MEIPASS, 'app', 'singer-list.csv')

        else:
            # כל מקרה אחר (GUI או לא מקומפל)
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


    def artists_from_song(self, my_file):
        split_file = os.path.split(my_file)[1]
        split_file = split_file.replace('_', ' ').replace('-', ' ')

        found_artists = []
        for source_name, target_name in self.singer_list:
            if source_name in split_file:
                exact = check_exact_name(split_file, source_name)
                if exact:
                    found_artists.append(target_name)

        if not found_artists:
            try:
                metadata_file = load_file(my_file)
                artist = metadata_file['artist'].value
                if artist:
                    artist = fix_jibrish(artist, "heb")
                    for source_name, target_name in self.singer_list:
                        if source_name in artist:
                            exact = check_exact_name(artist, source_name)
                            if exact:
                                found_artists.append(target_name)
                    
                    if not found_artists:
                        check_answer = self.check_artist(artist)
                        if check_answer:
                            found_artists.append(artist)
                
                if not found_artists:
                    title = metadata_file['title'].value
                    if title:
                        title = fix_jibrish(title, "heb")
                        for source_name, target_name in self.singer_list:
                            if source_name in title:
                                exact = check_exact_name(title, source_name)
                                if exact:
                                    found_artists.append(target_name)
            except UnicodeDecodeError as e:
                print(f"Error decoding metadata in file {my_file}: {e}")
            except Exception as e:
                print(f"An unexpected error occurred with file {my_file}: {e}")

        return found_artists if found_artists else None
    

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

    def setup_logging(self, log_level):
        self.logger = logging.getLogger('MusicSorter')
        self.logger.setLevel(log_level)

        # Create logs directory if it doesn't exist
        if not os.path.exists('logs'):
            os.makedirs('logs')

        # File handler
        log_filename = f'logs/music_sorter_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log'
        file_handler = logging.FileHandler(log_filename, encoding='utf-8')
        file_handler.setLevel(log_level)

        # Console handler
        console_handler = logging.StreamHandler()
        console_handler.setLevel(log_level)

        # Create a formatter and add it to the handlers
        formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s', datefmt='%Y-%m-%d %H:%M:%S')
        file_handler.setFormatter(formatter)
        console_handler.setFormatter(formatter)

        # Add the handlers to the logger
        self.logger.addHandler(file_handler)
        self.logger.addHandler(console_handler)


def main():
    parser = argparse.ArgumentParser(description=f"Singles Sorter {__VERSION__} - Scan and organize music files into folders by artist using advanced automation.")
    parser.add_argument('source_dir', help="Path to the source directory")
    parser.add_argument('target_dir', help="Path to the target directory")
    parser.add_argument('-c', '--copy_mode', help="Enable copy mode (default is move mode)", action='store_true')
    parser.add_argument('-a', '--abc_sort', help="Sort folders alphabetically (default: False)", action='store_true')
    parser.add_argument('-e', '--exist_only', help="Transfer to existing folders only (default: False)", action='store_true')
    parser.add_argument('-n', '--no_singles_folder', help="Do not create an internal 'singles' folder", action='store_false', dest='singles_folder', default=True)
    parser.add_argument('-m', '--main_folder_only', help="Sort only the main folder (default: False)", action='store_true')
    parser.add_argument('-d', '--duet_mode', help="Copy to all singers' folders for duets (default: False)", action='store_true')
    parser.add_argument('-l', '--log_level', help="Set the logging level", choices=['DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'], default='INFO')

    args = parser.parse_args()

    try:
        log_level = getattr(logging, args.log_level.upper())
        sorter = MusicSorter(args.source_dir, args.target_dir, args.copy_mode, args.abc_sort, args.exist_only, args.singles_folder, args.main_folder_only, args.duet_mode, log_level=log_level)
        sorter.clean_names()
        sorter.scan_dir()
    except Exception as e:
        logging.error(f"An error occurred: {str(e)}")
        sys.exit(1)

if __name__ == '__main__':
    main()