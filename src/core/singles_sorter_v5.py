# -*- coding: utf-8 -*-
__VERSION__ = '14.0'

import os
import re
import sys
import argparse
from shutil import copy, move
import shutil
from music_tag import load_file
from jibrish_to_hebrew import fix_jibrish, check_jibrish
import csv
from check_name import check_exact_name
import logging
import datetime
from ai_models import AIModels  # Import the new AIModels class

class MusicSorter:

    def __init__(self, source_dir, target_dir=None, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, main_folder_only=False, duet_mode=False, progress_callback=None, log_level=logging.INFO):
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
        self.songs_sorted = 0
        self.artist_folders_created = set()
        self.artist_song_count = {}
        self.albums_processed = 0

        # Setup logging
        self.setup_logging(log_level)

        # Initialize AIModels with the same logger
        self.ai_models = AIModels(logger=self.logger)

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
        
        filename = filename.replace("_", " ")
            
        return filename


    def fix_names(self):
        """
        Fix filenames and metadata of audio files in the source directory.
        """
        self.logger.info("Starting to fix filenames and metadata")
        
        files_to_process = []
        
        # Collect all audio files
        if self.main_folder_only:
            files_to_process = [
                os.path.join(self.source_dir, file) 
                for file in os.listdir(self.source_dir) 
                if file.lower().endswith((".mp3", ".wma", ".wav"))
            ]

        else:
            for root, _, files in os.walk(self.source_dir):
                for file in files:
                    if file.lower().endswith((".mp3", ".wma", ".wav")):
                        files_to_process.append(os.path.join(root, file))
        
        
        len_dir = len(files_to_process)
        progress_fix_generator = self.progress_display(len_dir)
        
        for file_path in files_to_process:

            try:

                show_len = next(progress_fix_generator)
                if self.progress_callback:
                    self.progress_callback(show_len)

                # Fix filename
                directory, filename = os.path.split(file_path)
                new_filename = self.clean_filename(filename)
                new_file_path = os.path.join(directory, new_filename)
                
                if file_path != new_file_path:
                    os.rename(file_path, new_file_path)
                    self.logger.info(f"Renamed file: {file_path} -> {new_file_path}")
                
                # Fix metadata
                metadata = load_file(new_file_path)
                
                # Fix artist metadata
                artist = metadata['artist'].value
                if artist and check_jibrish(artist):
                    fixed_artist = fix_jibrish(artist, "heb")
                    metadata['artist'] = fixed_artist
                    self.logger.info(f"Fixed artist metadata for {new_file_path}: {artist} -> {fixed_artist}")

                # Fix album artist metadata
                album_artist = metadata['albumartist'].value
                if album_artist and check_jibrish(album_artist):
                    fixed_album_artist = fix_jibrish(album_artist, "heb")
                    metadata['albumartist'] = fixed_album_artist
                    self.logger.info(f"Fixed album artist metadata for {new_file_path}: {album_artist} -> {fixed_album_artist}")
                
                # Fix title metadata
                title = metadata['title'].value
                if title and check_jibrish(title):
                    fixed_title = fix_jibrish(title, "heb")
                    metadata['title'] = fixed_title
                    self.logger.info(f"Fixed title metadata for {new_file_path}: {title} -> {fixed_title}")
                
                # Fix album metadata
                album = metadata['album'].value
                if album and check_jibrish(album):
                    fixed_album = fix_jibrish(album, "heb")
                    metadata['album'] = fixed_album
                    self.logger.info(f"Fixed album metadata for {new_file_path}: {album} -> {fixed_album}")
                
                # Fix genre metadata
                genre = metadata['genre'].value
                if genre and check_jibrish(genre):
                    fixed_genre = fix_jibrish(genre, "heb")
                    metadata['genre'] = fixed_genre
                    self.logger.info(f"Fixed genre metadata for {new_file_path}: {genre} -> {fixed_genre}")
                
                # Save the changes
                metadata.save()
                
            except Exception as e:
                self.logger.error(f"Error processing file {file_path}: {str(e)}")
        
        self.logger.info("Finished fixing filenames and metadata")




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
            # בדוק אם זו תיקיה
            if not os.path.isdir(folder_path):
                self.logger.debug(f"{folder_path} is not a directory")
                return False, False, None, None

            # קבל את כל קבצי האודיו בתיקייה
            audio_files = [f for f in os.listdir(folder_path) if f.lower().endswith((".mp3", ".wma", ".wav"))]
            
            # בדוק אם יש מספיק קבצים כדי להיחשב כאלבום
            if len(audio_files) < 3:
                return False, False, None, None
            
            # בדוק אם יש ספריות משנה (לאלבומים בדרך כלל אין ספריות משנה)
            if any(os.path.isdir(os.path.join(folder_path, item)) for item in os.listdir(folder_path)):
                return False, False, None, None

            # נתח מטא נתונים של הקבצים
            album_names = []
            artists = {}
            track_numbers = set()
            filename_numbers = set()

            for file in audio_files:
                file_path = os.path.join(folder_path, file)
                try:
                    metadata = load_file(file_path)
                    album = metadata.get('album')
                    artist = metadata.get('artist')
                    track = metadata.get('tracknumber')
                    
                    if album:
                        album_names.append(album.value)
                    if artist:
                        artists[artist.value] = artists.get(artist.value, 0) + 1
                    
                    if track:
                        try:
                            track_number = int(str(track.value).split('/')[0])  # Handle cases like "1/12"
                        except:
                            track_number = int(str(track.value))
                        track_numbers.add(track_number)
                    
                    # בדוק אם יש מספרי רצועות בשם הקובץ
                    filename_match = re.search(r'^(\d+)', file)
                    if filename_match:
                        filename_numbers.add(int(filename_match.group(1)))

                except Exception as e:
                    self.logger.error(f"Error reading metadata from {file_path}: {e}")

            # קבע אם זה אלבום על סמך מספרי הרצועות
            is_album = False
            if track_numbers:
                is_album = len(track_numbers) == len(audio_files) and max(track_numbers) == len(audio_files)
            elif filename_numbers:
                is_album = len(filename_numbers) == len(audio_files) and max(filename_numbers) == len(audio_files)
            
            # אם האלבום מזוהה לפי מספרי הרצועות, אך שמות האלבומים אינם עקביים, דלג עליו
            if is_album and album_names:
                unique_album_names = set(album_names)
                if len(unique_album_names) > 1:
                    self.logger.info(f"Album detected but skipped due to inconsistent album names: {folder_path}")
                    return True, False, None, None

            # אם לא נקבע לפי מספרי הרצועות, בדוק את עקביות שם האלבום
            if not is_album and album_names:
                most_common_album = max(set(album_names), key=album_names.count)
                album_name_consistency = album_names.count(most_common_album) / len(audio_files)
                
                # אם שם האלבום זהה עבור כל הקבצים מוגדר משתנה לניתוח בהמשך
                if len(set(album_names)) == 1:
                    is_album = True

                # אם מעל 60% אך פחות מ-100% מהקבצים מכילים שם אלבום זהה, יתבצע דילוג על התיקיה
                elif 1 > album_name_consistency >= 0.6:
                    self.logger.info(f"An inconsistency was found in the album names, it will be skipped to be safe: {folder_path}")
                    return True, False, None, None
                # אם יש שמות אלבומים מעורבים - התיקיה תזוהה כתיקית סינגלים
                else:
                    return False, False, None, None

            # קבע אם יש לעבד את האלבום או לדלג עליו
            should_process = False
            main_artist = None

            # בדיקה אם שירי האלבום מכילים שם אמן
            if is_album:
                if not artists:
                    # חיפוש שם אמן בשם התיקיה
                    dir_name = os.path.basename(folder_path)
                    for source_name, target_name in self.singer_list:
                        if source_name in dir_name:
                            exact = check_exact_name(dir_name, source_name)
                            if exact:
                                should_process = True
                                main_artist = target_name
                                break

                    # סיום הפעולה אם לא נמצא אמן בשם התיקיה
                    if not main_artist:
                        self.logger.info(f"Album detected but skipped due to lack of artist information: {folder_path}")
                        return True, False, None, None
                
                elif len(artists) == 1:
                    should_process = True
                    main_artist = list(artists.keys())[0]
                    main_artist = fix_jibrish(main_artist, "heb")
                else:
                    # בדוק אם אמן אחד מופיע ב-70% או יותר מהשירים
                    for artist, count in artists.items():
                        if count / audio_files >= 0.7:
                            should_process = True
                            main_artist = fix_jibrish(artist, "heb")
                            break

            # קבע את שם האלבום
            album_name = None
            if album_names:
                album_name = max(set(album_names), key=album_names.count)
                album_name = fix_jibrish(album_name, "heb")
            elif is_album:
                album_name = os.path.basename(folder_path)

            # Log the decision
            if is_album:
                if should_process:
                    self.logger.info(f"Album detected and will be processed: {folder_path}")
                    self.logger.info(f"Main artist: {main_artist}")
                else:
                    self.logger.info(f"Album detected but will be ignored due to inconsistent artists: {folder_path}")
            else:
                self.logger.debug(f"Not considered an album: {folder_path}")

            return is_album, should_process, album_name, main_artist

        except Exception as e:
            # כרגע בעת שגיאה בניתוח תיקיה - היא תזוהה כאלבום ותדולג ליתר ביטחון
            self.logger.error(f"Error in analyze_album for {folder_path}: {e}")
            return True, False, None, None

    def handle_album_transfer(self, album_path, album_name, artist_name):
        try:
            if not album_name or not artist_name:
                self.logger.warning(f"Missing album name or artist name for {album_path}")
                return

            # בדיקה אם שם האמן תקין
            check_answer = self.check_artist(artist_name)
            if check_answer is False:
                self.logger.warning(f"Wrong artist name for {album_path}")
                return
            
            # נקה את שם האלבום לשימוש בנתיבי קבצים
            safe_album_name = self.sanitize_filename(album_name)

            # שמור את מספר הקבצים בתיקיה
            files_num = len(os.listdir(album_path))

            # קבע את שם האמן על סמך רשימת הזמרים
            determined_artist_name = None
            for source_name, target_name in self.singer_list:
                if source_name in artist_name:
                    exact = check_exact_name(artist_name, source_name)
                    if exact:
                        determined_artist_name = target_name
                        break

            # השתמש בשם האמן שנקבע או במקור אם לא נמצא ברשימה
            final_artist_name = determined_artist_name if determined_artist_name else artist_name

            # קבע את נתיב היעד
            if self.abc_sort:
                target_path = os.path.join(self.target_dir, final_artist_name[0], final_artist_name)
            else:
                target_path = os.path.join(self.target_dir, final_artist_name)
            
            album_target_path = os.path.join(target_path, safe_album_name)

            # צור את ספריית היעד אם היא לא קיימת ומותר לנו
            if not self.exist_only or (self.exist_only and os.path.isdir(target_path)):
                try:
                    os.makedirs(album_target_path, exist_ok=True)
                except Exception as e:
                    self.logger.error(f"Failed in folder creating {album_target_path}: {str(e)}")
                    return

                # העבר את כל התיקיה
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
                
            self.albums_processed += 1
            self.artist_song_count[artist_name] = self.artist_song_count.get(artist_name, 0) + files_num

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
                        continue # דלג על עיבוד קבצים בודדים עבור אלבומים
                    
                    # עבד קבצים בודדים אם זה לא אלבום
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
                        if not os.path.exists(target_path):
                            os.makedirs(target_path, exist_ok=True)
                            self.artist_folders_created.add(artist)
                    except Exception as e:               
                        self.logger.error(f"Failed in folder creating {target_path}: {str(e)}")

                if os.path.isdir(target_path):
                    try:
                        if self.duet_mode and len(artists) > 1:
                            copy(file_path, target_path)
                            self.logger.info(f"Copied {file_path} to {target_path}")
                            self.songs_sorted += 1
                            self.artist_song_count[artist] = self.artist_song_count.get(artist, 0) + 1
                        elif self.copy_mode:
                            copy(file_path, target_path)
                            self.logger.info(f"Copied {file_path} to {target_path}")
                            self.songs_sorted += 1
                            self.artist_song_count[artist] = self.artist_song_count.get(artist, 0) + 1
                        else:
                            move(file_path, target_path)
                            self.logger.info(f"Moved {file_path} to {target_path}")
                            self.songs_sorted += 1
                            self.artist_song_count[artist] = self.artist_song_count.get(artist, 0) + 1
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

        return self.generate_summary()

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

        personal_csv_path = os.path.abspath("app/personal-singer-list.csv")
        print(personal_csv_path)
        
        
        with open(csv_path, 'r', encoding='utf-8') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]
        
        if os.path.isfile(personal_csv_path):
            with open(personal_csv_path, 'r', encoding="utf-8") as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)

        return singer_list


    def artists_from_song(self, my_file):
        split_file = os.path.splitext(os.path.basename(my_file))[0]
        split_file = split_file.replace('_', ' ').replace('-', ' ')

        found_artists = []

        # Check filename using original logic
        for source_name, target_name in self.singer_list:
            if source_name in split_file:
                exact = check_exact_name(split_file, source_name)
                if exact:
                    found_artists.append(target_name)

        if not found_artists:
            try:
                """
                metadata_file = load_file(my_file)

                # Check artist metadata
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
                
                # Check title metadata
                if not found_artists:
                    title = metadata_file['title'].value
                    if title:
                        title = fix_jibrish(title, "heb")
                        for source_name, target_name in self.singer_list:
                            if source_name in title:
                                exact = check_exact_name(title, source_name)
                                if exact:
                                    found_artists.append(target_name)
                """
                # If no artists found, use NER on filename
                if self.ai_models.nlp:
                    self.logger.debug(f"Using NER to process filename: {split_file}")
                    found_artists = self.ai_models.process_with_ner(split_file)
                    if found_artists:
                        self.logger.debug(f"NER found artists: {found_artists}")
                    else:
                        self.logger.debug("NER did not find any artists")
                else:
                    self.logger.warning("NLP model not available for processing")

            except UnicodeDecodeError as e:
                self.logger.error(f"Error decoding metadata in file {my_file}: {e}")
            except Exception as e:
                self.logger.error(f"An unexpected error occurred with file {my_file}: {e}")

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


    def generate_summary(self):
        summary = {
            "songs_sorted": self.songs_sorted,
            "artist_folders_created": len(self.artist_folders_created),
            "albums_processed": self.albums_processed,
            "top_artists": sorted(self.artist_song_count.items(), key=lambda x: x[1], reverse=True)[:5]
        }
        
        summary_text = f"""
Summary of Music Sorting:
-------------------------
Total songs sorted: {summary['songs_sorted']}
New artist folders created: {summary['artist_folders_created']}
Albums processed: {summary['albums_processed']}

Top 5 Artists by Song Count:
{self._format_top_artists(summary['top_artists'])}
"""
        
        self.logger.info(summary_text)
        return summary

    def _format_top_artists(self, top_artists):
        return "\n".join([f"{artist}: {count} שירים" for i, (artist, count) in enumerate(top_artists)])



def main():
    parser = argparse.ArgumentParser(description=f"Singles Sorter {__VERSION__} - Scan and organize music files into folders by artist using advanced automation.")
    parser.add_argument('source_dir', help="Path to the source directory")
    parser.add_argument('target_dir', nargs="?", help="Path to the target directory")
    parser.add_argument('-c', '--copy_mode', help="Enable copy mode (default is move mode)", action='store_true')
    parser.add_argument('-a', '--abc_sort', help="Sort folders alphabetically (default: False)", action='store_true')
    parser.add_argument('-e', '--exist_only', help="Transfer to existing folders only (default: False)", action='store_true')
    parser.add_argument('-n', '--no_singles_dir', help="Do not create an internal 'singles' folder", action='store_false', dest='singles_folder', default=True)
    parser.add_argument('-m', '--main_dir_only', help="Sort only the main folder (default: False)", action='store_true', dest='main_folder_only')
    parser.add_argument('-d', '--duet_mode', help="Copy to all singers' folders for duets (default: False)", action='store_true')
    parser.add_argument("-f", "--fix_names", action="store_true", help="Fix file names only without sorting files")
    parser.add_argument('-l', '--log_level', help="Set the logging level", choices=['DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'], default='INFO')

    args = parser.parse_args()

    try:
        log_level = getattr(logging, args.log_level.upper())
        sorter = MusicSorter(
            args.source_dir,
            args.target_dir,
            args.copy_mode,
            args.abc_sort,
            args.exist_only,
            args.singles_folder,
            args.main_folder_only,
            args.duet_mode,
            log_level=log_level
        )

        if args.fix_names:
            sorter.fix_names()
        else:
            sorter.scan_dir()
    except Exception as e:
        logging.error(f"An error occurred: {str(e)}")
        sys.exit(1)

if __name__ == '__main__':
    main()