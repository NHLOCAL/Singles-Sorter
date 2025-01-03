# -*- coding: utf-8 -*-
__VERSION__ = '14.0'

import os
import sys
import re
import argparse
import csv
import logging
import datetime
import traceback
from pathlib import Path
from music_tag import load_file
from jibrish_to_hebrew import fix_jibrish, check_jibrish
from check_name import check_exact_name
import shutil

# פונקציה לבדיקת קיום קבצי מודל AI
def check_model_files():
    model_clf_path = 'models/music_entity_clf/music_entity_clf.pkl'
    meta_json_path = 'models/singer_ner_he/meta.json'
    return os.path.isfile(model_clf_path) and os.path.isfile(meta_json_path)

# קריאה לפונקציה בתחילת הקוד
ai_invalid = False if check_model_files() else True
if not ai_invalid:
    from ai_models import AIModels

# הגדרת רשימות כקבועים גלובליים
UNUSUAL_LIST = [
    "סינגלים",
    "סינגל",
    "אבגדהוזחטיכלמנסעפצקרשתךםןץ",
    "אמן לא ידוע",
    "טוב",
    "לא ידוע",
    "תודה לך ה"
]

SUBSTRINGS_TO_REMOVE = [
    "מייל מיוזיק",
    "ציצו במייל",
    "חדשות המוזיקה",
    "חדשות המוזיקה",
    "ציצו",
    "מוזיקה מכל הלב",
    "מייל מיוזיק",
    "המחדש",
]

SUPPORTED_EXTENSIONS = {'.m4a', '.wma', '.wav', '.aiff', '.flac', '.aac', '.alac', '.wv', '.ogg', '.dsf', '.opus', '.mp3'}

# הוספת רשימת מילות המפתח לזיהוי סינגלים
SINGLE_KEYWORDS = ["סינגל", "סינגלים", "single", "singles"]

class MusicSorter:

    def __init__(
        self,
        source_dir,
        target_dir=None,
        copy_mode=False,
        abc_sort=False,
        exist_only=False,
        singles_folder=True,
        main_folder_only=False,
        duet_mode=False,
        progress_callback=None,
        log_level=logging.INFO,
        logger=None
    ):
        self.source_dir = Path(source_dir)
        self.target_dir = Path(target_dir) if target_dir else None
        self.copy_mode = copy_mode
        self.abc_sort = abc_sort
        self.exist_only = exist_only
        self.singles_folder = singles_folder
        self.main_folder_only = main_folder_only
        self.duet_mode = duet_mode
        self.progress_callback = progress_callback
        self.operating_details = [
            source_dir,
            target_dir,
            copy_mode,
            abc_sort,
            exist_only,
            singles_folder,
            main_folder_only,
            duet_mode
        ]
        self.singer_list = self.list_from_csv()
        self.songs_sorted = 0
        self.artist_folders_created = set()
        self.artist_song_count = {}
        self.albums_processed = 0

        # Use the provided logger or create a new one
        self.logger = logger or logging.getLogger('MusicSorter')
        self.logger.setLevel(log_level)

        # יצירת מופע של AIModels אם לא באנדרואיד
        if not ai_invalid:
            self.ai_models = AIModels(logger=self.logger)
        else:
            self.ai_models = None  # אם באנדרואיד, לא משתמשים ב-AIModels

    def progress_display(self, total_amount):
        for current_item in range(1, total_amount + 1):
            progress = (current_item / total_amount) * 100
            yield progress

    def check_errors(self):
        """
        Checks for potential errors related to source and target directories.

        Raises:
            FileNotFoundError: If the source or target directory does not exist.
            PermissionError: If the script does not have write access to the target directory.
            ValueError: If the source and target directories are the same or source directory is empty.
        """
        if not self.source_dir.exists():
            raise FileNotFoundError("תיקיית המקור לא נמצאה")

        if not self.target_dir.exists():
            raise FileNotFoundError("תיקיית היעד לא נמצאה")

        if not os.access(self.target_dir, os.W_OK):
            raise PermissionError("אין הרשאת כתיבה לתיקיית היעד")

        if self.source_dir.samefile(self.target_dir):
            raise ValueError("תיקיית המקור ותיקיית היעד לא יכולות להיות זהות")

        if not any(self.source_dir.iterdir()):
            raise ValueError("תיקיית המקור ריקה")


    def clean_filename(self, filename):
        # טיפול בקווים תחתונים
        if "_" in filename:
            if " " not in filename:
                # אם אין רווחים, החלף את כל הקווים התחתונים ברווחים
                filename = filename.replace("_", " ")
            else:
                # הסר קווים תחתונים שמופיעים בין אותיות ללא רווחים
                filename = re.sub(r'(?<=\w)_(?=\w)', '', filename)

        # הסרת מקפים שמחוברים לאותיות ללא רווח
        filename = re.sub(r'(?<=\w)-(?=\w)', ' ', filename)

        # הסרת תתי-מחרוזות מוגדרות מראש, מבלי להסיר רווחים
        for substring in SUBSTRINGS_TO_REMOVE:
            if substring.strip():  # מוודא שהמחרוזת אינה רווח או מחרוזת ריקה
                filename = filename.replace(substring, "")

        # החלפת רווחים מרובים ברווח בודד
        filename = re.sub(r'\s+', ' ', filename).strip()

        return filename




    def fix_metadata_field(self, metadata, field_name, file_path):
        value = metadata[field_name].value
        if value and check_jibrish(value):
            fixed_value = fix_jibrish(value, "heb")
            metadata[field_name] = fixed_value
            self.logger.info(f"Fixed {field_name} for {file_path}: {value} -> {fixed_value}")

    def fix_names(self):
        """
        Fix filenames and metadata of audio files in the source directory.
        """
        self.logger.info("Starting to fix filenames and metadata")

        # Collect all audio files
        if self.main_folder_only:
            files_to_process = [f for f in self.source_dir.glob('*') if f.suffix.lower() in SUPPORTED_EXTENSIONS]
        else:
            files_to_process = [f for f in self.source_dir.rglob('*') if f.suffix.lower() in SUPPORTED_EXTENSIONS]

        total_files = len(files_to_process)
        progress_fix_generator = self.progress_display(total_files)

        for file_path in files_to_process:

            try:
                progress = next(progress_fix_generator)
                if self.progress_callback:
                    self.progress_callback(progress)

                # Fix filename
                new_filename = self.clean_filename(file_path.name)
                new_filename = self.sanitize_filename(new_filename)
                if not new_filename:
                    self.logger.warning(f"Filename is empty after sanitization for {file_path}, skipping")
                    continue

                new_file_path = file_path.with_name(new_filename)

                if file_path != new_file_path:
                    if new_file_path.exists():
                        new_file_path = self.generate_unique_filename(new_file_path)
                    try:
                        file_path.rename(new_file_path)
                        self.logger.info(f"Renamed file: {file_path} -> {new_file_path}")
                    except Exception as e:
                        self.logger.error(f"Failed to rename {file_path} to {new_file_path}: {str(e)}")
                        self.logger.debug(traceback.format_exc())

                # Fix metadata
                metadata = load_file(new_file_path)

                for field in ['artist', 'albumartist', 'title', 'album', 'genre']:
                    self.fix_metadata_field(metadata, field, new_file_path)

                # Save the changes
                metadata.save()

            except Exception as e:
                self.logger.error(f"Error processing file {file_path}: {str(e)}")
                self.logger.debug(traceback.format_exc())

        self.logger.info("Finished fixing filenames and metadata")

    def sanitize_filename(self, filename):
        # Remove invalid characters for Windows filenames
        filename = re.sub(r'[<>:"/\\|?*]', '', filename)

        # Replace multiple spaces with a single space
        filename = re.sub(r'\s+', ' ', filename)

        # Trim leading and trailing whitespace
        filename = filename.strip()

        # Truncate to 255 characters (limit for filenames)
        filename = filename[:255]

        # If filename is empty after sanitization, return None
        if not filename:
            return None

        return filename

    def generate_unique_filename(self, target_path):
        """
        If target_path exists, append a number to make it unique.
        """
        counter = 1
        original_stem = target_path.stem
        original_suffix = target_path.suffix
        parent = target_path.parent
        while target_path.exists():
            target_path = parent / f"{original_stem}_{counter}{original_suffix}"
            counter += 1
        return target_path

    def move_file(self, source, target):
        try:
            source.rename(target)
            return True
        except Exception as e:
            self.logger.debug(f"rename failed for {source} to {target}: {e}, trying shutil.move")
            try:
                shutil.move(str(source), str(target))
                return True
            except Exception as e2:
                self.logger.error(f"Failed to move {source} to {target}: {str(e2)}")
                self.logger.debug(traceback.format_exc())
                # Do not raise exception; just log the error
                return False  # Indicate failure

    def analyze_album(self, folder_path):
        """
        Analyzes a folder to determine if it's an album and if it should be processed or ignored.

        Args:
            folder_path (Path): Path to the folder to analyze

        Returns:
            tuple: (is_album, should_process, album_name, artist_name)
                is_album (bool): True if the folder is considered an album
                should_process (bool): True if the album should be processed (not ignored)
                album_name (str): Name of the album (if applicable)
                artist_name (str): Name of the artist (if applicable)
        """
        try:
            # Check if it's a directory
            if not folder_path.is_dir():
                self.logger.debug(f"{folder_path} is not a directory")
                return False, False, None, None

            # Get all audio files in the folder
            audio_files = [f for f in folder_path.glob('*') if f.suffix.lower() in SUPPORTED_EXTENSIONS]

            # Check if there are enough files to be considered an album
            if len(audio_files) < 3:
                return False, False, None, None

            # Check for subdirectories (albums usually don't have subdirectories)
            folder_count = sum(1 for item in folder_path.iterdir() if item.is_dir())

            if folder_count == 1:
                contain_folder = True
            elif folder_count > 1:
                return False, False, None, None
            else:
                contain_folder = False

            # Analyze metadata of the files
            album_names = []
            artists = {}
            track_numbers = set()
            filename_numbers = set()

            for file in audio_files:
                file_path = file
                try:
                    metadata = load_file(file_path)
                    album = metadata.get('album')
                    artist = metadata.get('artist')
                    album_artist = metadata.get('albumartist')  # קריאת אמן אלבום
                    track = metadata.get('tracknumber')

                    if album:
                        album_value = album.value
                        album_names.append(album_value)

                    # שינוי: בדיקת אמן אלבום תחילה
                    if album_artist:
                        artist_name = fix_jibrish(album_artist.value, "heb")
                        if self.check_artist(artist_name):  # בדיקה אם אמן אלבום תקין
                            artists[artist_name] = artists.get(artist_name, 0) + 1
                        elif artist:  # אם לא תקין, נסה את אמן רגיל
                            artist_name = fix_jibrish(artist.value, "heb")
                            artists[artist_name] = artists.get(artist_name, 0) + 1
                    elif artist:  # אם אין אמן אלבום, נסה את אמן רגיל
                        artist_name = fix_jibrish(artist.value, "heb")
                        artists[artist_name] = artists.get(artist_name, 0) + 1

                    if track:
                        try:
                            track_number = int(str(track.value).split('/')[0])  # Handle "1/12"
                        except ValueError:
                            track_number = int(str(track.value))
                        track_numbers.add(track_number)

                    # Check for track numbers in filename
                    filename_match = re.search(r'^(\d+)', file.name)
                    if filename_match:
                        filename_numbers.add(int(filename_match.group(1)))

                except Exception as e:
                    self.logger.error(f"Error reading metadata from {file_path}: {e}")
                    self.logger.debug(traceback.format_exc())

            # הוספת בדיקה למילות מפתח של סינגלים
            contains_single_keyword = False
            for name in album_names + list(artists.keys()):
                if name:
                    name_lower = name.lower()
                    for keyword in SINGLE_KEYWORDS:
                        if keyword.lower() in name_lower:
                            contains_single_keyword = True
                            break
                    if contains_single_keyword:
                        break

            if contains_single_keyword:
                self.logger.info(f"Album or artist name contains single keyword, treating files as singles: {folder_path}")
                return False, False, None, None  # Treat as not an album, so files will be processed individually

            # Determine if it's an album based on track numbers
            is_album = False
            if track_numbers:
                is_album = len(track_numbers) == len(audio_files) and max(track_numbers) == len(
                    audio_files
                )
            elif filename_numbers:
                is_album = len(filename_numbers) == len(audio_files) and max(
                    filename_numbers
                ) == len(audio_files)

            # Skip if album names are inconsistent
            if is_album and album_names:
                unique_album_names = set(album_names)
                if len(unique_album_names) > 1:
                    self.logger.info(
                        f"Album detected but skipped due to inconsistent album names: {folder_path}"
                    )
                    return True, False, None, None

            # Check consistency of album name
            if not is_album and album_names:
                most_common_album = max(set(album_names), key=album_names.count)
                album_name_consistency = album_names.count(most_common_album) / len(audio_files)

                if len(set(album_names)) == 1:
                    is_album = True
                elif 1 > album_name_consistency >= 0.6 and not contain_folder:
                    self.logger.info(
                        f"Inconsistency found in album names, skipping: {folder_path}"
                    )
                    return True, False, None, None
                else:
                    return False, False, None, None

            # Determine whether to process the album
            should_process = False
            main_artist = None

            if is_album and contain_folder:
                self.logger.info(
                    f"Album skipped due to containing internal folder: {folder_path}"
                )
                return True, False, None, None

            if is_album:
                if not artists:
                    # Search for artist name in folder name
                    dir_name = folder_path.name
                    for source_name, target_name in self.singer_list:
                        if source_name in dir_name:
                            exact = check_exact_name(dir_name, source_name)
                            if exact:
                                should_process = True
                                main_artist = target_name
                                break

                    if not main_artist:
                        self.logger.info(
                            f"Album detected but skipped due to lack of artist info: {folder_path}"
                        )
                        return True, False, None, None

                elif len(artists) == 1:
                    should_process = True
                    main_artist = list(artists.keys())[0]
                else:
                    # Check if one artist appears in 70% or more of the songs
                    total_songs = sum(artists.values())
                    for artist, count in artists.items():
                        if count / total_songs >= 0.7:
                            should_process = True
                            main_artist = artist
                            break

            # Determine album name
            album_name = None
            if album_names:
                album_name = max(set(album_names), key=album_names.count)
                album_name = fix_jibrish(album_name, "heb")
            elif is_album:
                album_name = folder_path.name

            # Log the decision
            if is_album:
                if should_process:
                    self.logger.info(f"Album detected and will be processed: {folder_path}")
                    self.logger.info(f"Main artist: {main_artist}")
                else:
                    self.logger.info(
                        f"Album detected but will be ignored due to inconsistent artists: {folder_path}"
                    )
            else:
                self.logger.debug(f"Not considered an album: {folder_path}")

            return is_album, should_process, album_name, main_artist

        except Exception as e:
            # In case of an error, identify as an album and skip to be safe
            self.logger.error(f"Error in analyze_album for {folder_path}: {e}")
            self.logger.debug(traceback.format_exc())
            return True, False, None, None

    def handle_album_transfer(self, album_path, album_name, artist_name):
        try:
            if not album_name or not artist_name:
                self.logger.warning(f"Missing album name or artist name for {album_path}")
                return

            # Check if artist name is valid
            if not self.check_artist(artist_name):
                self.logger.warning(f"Invalid artist name for {album_path}")
                return

            # Sanitize album name
            safe_album_name = self.sanitize_filename(album_name)
            if not safe_album_name:
                self.logger.warning(f"Album name is empty after sanitization for {album_path}")
                return

            # Determine artist name from the singer list
            determined_artist_name = None
            for source_name, target_name in self.singer_list:
                if source_name in artist_name:
                    exact = check_exact_name(artist_name, source_name)
                    if exact:
                        determined_artist_name = target_name
                        break

            # Use the determined artist name or the original if not found in the list
            final_artist_name = determined_artist_name if determined_artist_name else artist_name

            if not final_artist_name:
                self.logger.warning(f"Empty artist name for album {album_path}, skipping")
                return

            # Determine target path
            if self.abc_sort:
                if not final_artist_name:
                    self.logger.warning(f"Empty artist name for album {album_path}, skipping")
                    return
                target_path = self.target_dir / final_artist_name[0] / final_artist_name
            else:
                target_path = self.target_dir / final_artist_name

            album_target_path = target_path / safe_album_name

            # Check if album already exists
            if album_target_path.exists():
                self.logger.info(f"Album {album_name} already exists at {album_target_path}, skipping")
                return

            # Check if artist folder exists when exist_only is True
            if self.exist_only and not target_path.exists():
                self.logger.info(f"Skipped album transfer: {album_path} (artist folder does not exist)")
                return

            # Create album directory if it doesn't exist
            try:
                album_target_path.mkdir(parents=True, exist_ok=True)
            except Exception as e:
                self.logger.error(f"Failed to create folder {album_target_path}: {str(e)}")
                self.logger.debug(traceback.format_exc())
                return

            # Transfer the entire folder
            for item in album_path.iterdir():
                source_item = item
                safe_item_name = self.sanitize_filename(item.name)
                if not safe_item_name:
                    self.logger.warning(f"Item name is empty after sanitization for {item}, skipping")
                    continue  # Skip this item
                target_item = album_target_path / safe_item_name

                if self.copy_mode:
                    if source_item.is_file():
                        try:
                            shutil.copy2(source_item, target_item)
                            self.logger.info(f"Copied {source_item} to {target_item}")
                        except Exception as e:
                            self.logger.error(f"Failed to copy {source_item}: {str(e)}")
                            self.logger.debug(traceback.format_exc())
                    else:
                        try:
                            shutil.copytree(source_item, target_item)
                            self.logger.info(f"Copied directory {source_item} to {target_item}")
                        except Exception as e:
                            self.logger.error(f"Failed to copy directory {source_item}: {str(e)}")
                            self.logger.debug(traceback.format_exc())
                else:
                    success = self.move_file(source_item, target_item)
                    if success:
                        self.logger.info(f"Moved {source_item} to {target_item}")
                    else:
                        self.logger.error(f"Failed to move {source_item} to {target_item}")

            if not self.copy_mode:
                try:
                    album_path.rmdir()
                    self.logger.info(f"Removed original album folder: {album_path}")
                except Exception as e:
                    self.logger.error(
                        f"Failed to remove original album folder {album_path}: {str(e)}"
                    )
                    self.logger.debug(traceback.format_exc())

            # Update counters
            files_num = sum(1 for f in album_target_path.iterdir() if f.is_file())
            self.albums_processed += 1
            self.artist_song_count[artist_name] = self.artist_song_count.get(artist_name, 0) + files_num

        except Exception as e:
            self.logger.error(f"Error in handle_album_transfer for {album_path}: {e}")
            self.logger.debug(traceback.format_exc())

    def scan_dir(self):
        """
        Scans the specified directory and organizes music files into artist folders.
        """
        self.logger.info("Starting directory scan")

        self.check_errors()

        info_list = []
        items_to_process = []
        if not self.main_folder_only:
            for root, dirs, files in os.walk(self.source_dir):
                root_path = Path(root)
                items_to_process.append(root_path)
        else:
            items_to_process = [item for item in self.source_dir.iterdir()]

        for item in items_to_process:
            try:
                if item.is_dir():
                    is_album, should_process, album_name, artist_name = self.analyze_album(item)
                    if is_album:
                        if should_process:
                            self.handle_album_transfer(item, album_name, artist_name)
                        continue  # Skip processing individual files for albums

                audio_files = [item] if item.is_file() else [f for f in item.glob('*') if f.is_file()]
                for my_file in audio_files:
                    if my_file.suffix.lower() in SUPPORTED_EXTENSIONS:
                        artists = self.artists_from_song(my_file)
                        if artists:
                            info_list.append((my_file, artists))
            except Exception as e:
                self.logger.error(f"Error processing item {item}: {e}")
                self.logger.debug(traceback.format_exc())

        total_files = len(info_list)
        progress_generator = self.progress_display(total_files)

        for file_path, artists in info_list:
            try:
                progress = next(progress_generator)
                self.logger.debug(f"{progress:.2f}% completed")
                if self.progress_callback:
                    self.progress_callback(progress)

                if not self.duet_mode:
                    artists = [artists[0]]  # Only use the first artist if duet_mode is False

                for artist in artists:
                    target_path = self.get_target_path(artist)

                    if not self.exist_only or (self.exist_only and target_path.is_dir()):
                        try:
                            if not target_path.exists() and not self.exist_only:
                                target_path.mkdir(parents=True, exist_ok=True)
                                self.artist_folders_created.add(artist)
                        except Exception as e:
                            self.logger.error(f"Failed to create folder {target_path}: {str(e)}")
                            self.logger.debug(traceback.format_exc())

                        if target_path.is_dir():
                            try:
                                destination_file_name = self.sanitize_filename(file_path.name)
                                if not destination_file_name:
                                    self.logger.warning(f"Filename is empty after sanitization for {file_path}, skipping")
                                    continue
                                destination_file = target_path / destination_file_name

                                if destination_file.exists():
                                    destination_file = self.generate_unique_filename(destination_file)

                                if self.duet_mode and len(artists) > 1:
                                    shutil.copy2(file_path, destination_file)
                                    self.logger.info(f"Copied {file_path} to {destination_file}")
                                elif self.copy_mode:
                                    shutil.copy2(file_path, destination_file)
                                    self.logger.info(f"Copied {file_path} to {destination_file}")
                                else:
                                    success = self.move_file(file_path, destination_file)
                                    if success:
                                        self.logger.info(f"Moved {file_path} to {destination_file}")
                                    else:
                                        self.logger.error(f"Failed to move {file_path} to {destination_file}")

                                self.songs_sorted += 1
                                self.artist_song_count[artist] = self.artist_song_count.get(artist, 0) + 1
                            except Exception as e:
                                self.logger.error(f"Failed to process {file_path}: {str(e)}")
                                self.logger.debug(traceback.format_exc())

                # If it's a duet and we've copied to all singers' folders, remove the original
                if self.duet_mode and len(artists) > 1 and not self.copy_mode:
                    try:
                        if file_path.exists():
                            file_path.unlink()
                            self.logger.info(f"Removed original file: {file_path}")
                    except Exception as e:
                        self.logger.error(f"Failed to remove original file {file_path}: {str(e)}")
                        self.logger.debug(traceback.format_exc())
            except Exception as e:
                self.logger.error(f"Error processing file {file_path}: {e}")
                self.logger.debug(traceback.format_exc())

        self.logger.info("Directory scan completed")

        return self.generate_summary()


    def get_target_path(self, artist):
        if self.singles_folder and self.abc_sort:
            return self.target_dir / artist[0] / artist / "סינגלים"
        elif self.singles_folder:
            return self.target_dir / artist / "סינגלים"
        elif self.abc_sort:
            return self.target_dir / artist[0] / artist
        else:
            return self.target_dir / artist

    def load_csv(self, path):
        try:
            with path.open('r', encoding='utf-8') as file:
                return [tuple(row) for row in csv.reader(file)]
        except FileNotFoundError as e:
            self.logger.error(f"CSV file not found: {path}")
            raise e
        except Exception as e:
            self.logger.error(f"Error reading CSV file {path}: {e}")
            self.logger.debug(traceback.format_exc())
            return []

    def is_cli_mode(self):
        try:
            return sys.stdin is not None and sys.stdin.isatty()
        except AttributeError:
            return False

    def list_from_csv(self):
        # Import singer list from CSV file
        if getattr(sys, 'frozen', False) and self.is_cli_mode():
            # Running in a bundle (e.g., PyInstaller)
            main_csv_path = Path(sys._MEIPASS) / 'app' / 'singer-list.csv'
        else:
            main_csv_path = Path("app/singer-list.csv").resolve()

        personal_csv_path = Path("app/personal-singer-list.csv").resolve()

        csv_paths = [main_csv_path, personal_csv_path]

        singer_list = []
        for csv_path in csv_paths:
            if csv_path.is_file():
                singer_list.extend(self.load_csv(csv_path))

        if not singer_list:
            raise FileNotFoundError("No singer lists found.")

        return singer_list

    def artists_from_song(self, my_file):
        # ניקוי ושינוי שם הקובץ לפני ניתוח
        original_filename = my_file.name
        cleaned_filename = self.clean_filename(original_filename)
        sanitized_filename = self.sanitize_filename(cleaned_filename) if cleaned_filename else original_filename

        # שימוש בשם הקובץ הנקי להמשך העיבוד בלבד
        split_file = Path(sanitized_filename).stem  # קבלת שם הקובץ ללא הסיומת

        found_artists = []

        # שלב ראשון: בדיקת שם הקובץ באמצעות רשימת הזמרים
        for source_name, target_name in self.singer_list:
            if source_name in split_file:
                exact = check_exact_name(split_file, source_name)
                if exact:
                    found_artists.append(target_name)
                    break  # מצאנו אמן, אין צורך להמשיך

        try:
            metadata_file = load_file(my_file)
        except Exception as e:
            self.logger.error(f"Error loading metadata for file {my_file}: {e}")
            self.logger.debug(traceback.format_exc())
            metadata_file = None

        if metadata_file:
            # ניקוי ושינוי שם הכותרת במטאדאטה בלבד פנימית
            original_title = metadata_file['title'].value
            if original_title:
                cleaned_title = self.clean_filename(original_title)
                sanitized_title = self.sanitize_filename(cleaned_title) if cleaned_title else original_title
                # שימוש בכותרת המסוננת להמשך העיבוד בלבד
            else:
                sanitized_title = original_title  # אם אין כותרת, נשאר עם הערך המקורי

        if not found_artists and metadata_file:
            # שלב שני: בדיקת שם האמן במטאדאטה
            artist = metadata_file['artist'].value
            if artist:
                artist = fix_jibrish(artist, "heb")
                # בדיקת אם האמן נמצא ברשימת הזמרים
                for source_name, target_name in self.singer_list:
                    if source_name in artist:
                        exact = check_exact_name(artist, source_name)
                        if exact:
                            found_artists.append(target_name)
                            break

                if not found_artists and self.check_artist(artist):
                    # אם האמן לא נמצא ברשימה, וב-AIModels זמין
                    if self.ai_models and not ai_invalid:
                        # אימות באמצעות מודל SKLEARN
                        if self.ai_models.verify_artist_with_sklearn(artist):
                            found_artists.append(artist)
                        else:
                            self.logger.debug(f"Artist '{artist}' not verified by SKLEARN model")
                    else:
                        self.logger.debug("AI-based checks are disabled on Android.")

        if not found_artists and metadata_file:
            # שלב שלישי: בדיקת שם הזמר בכותרת השיר במטאדאטה
            if original_title:
                # שימוש בכותרת המסוננת
                title = sanitized_title
                title = fix_jibrish(title, "heb")
                for source_name, target_name in self.singer_list:
                    if source_name in title:
                        exact = check_exact_name(title, source_name)
                        if exact:
                            found_artists.append(target_name)
                            break

        if not found_artists and not ai_invalid:
            # שלב רביעי: שימוש ב-NER על שם הקובץ
            if self.ai_models:
                self.logger.debug(f"Using NER to process filename: {split_file}")
                found_artists = self.ai_models.process_with_ner(split_file)
                if found_artists:
                    self.logger.debug(f"NER found artists in filename: {found_artists}")
                else:
                    self.logger.debug("NER did not find any artists in filename")
                    # שלב חמישי: שימוש ב-NER על כותרת השיר במטאדאטה
                    if metadata_file and original_title:
                        title = sanitized_title
                        title = fix_jibrish(title, "heb")
                        self.logger.debug(f"Using NER to process title: {title}")
                        found_artists = self.ai_models.process_with_ner(title)
                        if found_artists:
                            self.logger.debug(f"NER found artists in title: {found_artists}")
                        else:
                            self.logger.debug("NER did not find any artists in title")
            else:
                self.logger.debug("AI-based checks are disabled on Android.")

        return found_artists if found_artists else None


    def check_artist(self, artist):
        if not artist or artist in UNUSUAL_LIST:
            return False

        words = artist.split()
        if not (1 <= len(words) < 4):
            return False

        return all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳- " for c in artist)

    def generate_summary(self):
        summary = {
            "songs_sorted": self.songs_sorted,
            "artist_folders_created": len(self.artist_folders_created),
            "albums_processed": self.albums_processed,
            "top_artists": sorted(
                self.artist_song_count.items(), key=lambda x: x[1], reverse=True
            )[:5]
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
        return "\n".join(
            [f"{artist}: {count} שירים" for i, (artist, count) in enumerate(top_artists)]
        )


def main():
    parser = argparse.ArgumentParser(
        description=f"Singles Sorter {__VERSION__} - Scan and organize music files into folders by artist using advanced automation."
    )
    parser.add_argument('source_dir', help="Path to the source directory")
    parser.add_argument('target_dir', nargs="?", help="Path to the target directory")
    parser.add_argument(
        '-c', '--copy_mode', help="Enable copy mode (default is move mode)", action='store_true'
    )
    parser.add_argument(
        '-a', '--abc_sort', help="Sort folders alphabetically (default: False)", action='store_true'
    )
    parser.add_argument(
        '-e',
        '--exist_only',
        help="Transfer to existing folders only (default: False)",
        action='store_true'
    )
    parser.add_argument(
        '-n',
        '--no_singles_dir',
        help="Do not create an internal 'singles' folder",
        action='store_false',
        dest='singles_folder',
        default=True
    )
    parser.add_argument(
        '-m',
        '--main_dir_only',
        help="Sort only the main folder (default: False)",
        action='store_true',
        dest='main_folder_only'
    )
    parser.add_argument(
        '-d',
        '--duet_mode',
        help="Copy to all singers' folders for duets (default: False)",
        action='store_true'
    )
    parser.add_argument(
        "-f",
        "--fix_names",
        action="store_true",
        help="Fix file names only without sorting files"
    )
    parser.add_argument(
        '-l',
        '--log_level',
        help="Set the logging level",
        choices=['DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'],
        default='INFO'
    )

    args = parser.parse_args()

    # Set up logging
    logger = logging.getLogger('MusicSorter')
    log_level = getattr(logging, args.log_level.upper())
    logger.setLevel(log_level)

    # Create logs directory if it doesn't exist
    logs_dir = Path('logs')
    logs_dir.mkdir(exist_ok=True)

    # File handler with unique identifier
    log_filename = logs_dir / f'music_sorter_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}_{os.getpid()}.log'
    file_handler = logging.FileHandler(log_filename, encoding='utf-8')
    file_handler.setLevel(log_level)

    # Console handler
    console_handler = logging.StreamHandler()
    console_handler.setLevel(log_level)

    # Create a formatter and add it to the handlers
    formatter = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(message)s', datefmt='%Y-%m-%d %H:%M:%S'
    )
    file_handler.setFormatter(formatter)
    console_handler.setFormatter(formatter)

    # Add the handlers to the logger
    logger.addHandler(file_handler)
    logger.addHandler(console_handler)

    try:
        sorter = MusicSorter(
            args.source_dir,
            args.target_dir,
            args.copy_mode,
            args.abc_sort,
            args.exist_only,
            args.singles_folder,
            args.main_folder_only,
            args.duet_mode,
            log_level=log_level,
            logger=logger
        )

        if args.fix_names:
            sorter.fix_names()
        else:
            sorter.scan_dir()
    except FileNotFoundError as e:
        logger.error(f"File not found: {str(e)}")
        logger.debug(traceback.format_exc())
        sys.exit(1)
    except PermissionError as e:
        logger.error(f"Permission error: {str(e)}")
        logger.debug(traceback.format_exc())
        sys.exit(1)
    except ValueError as e:
        logger.error(f"Value error: {str(e)}")
        logger.debug(traceback.format_exc())
        sys.exit(1)
    except Exception as e:
        logger.error(f"An unexpected error occurred: {str(e)}")
        logger.debug(traceback.format_exc())
        sys.exit(1)


if __name__ == '__main__':
    main()
