import os
from shutil import copy, move
from music_tag import load_file
from os.path import join
import csv
from jibrish_to_hebrew import jibrish_to_hebrew
from identify_similarities import similarity_sure
from check_name import check_exact_name


class MusicFile:
    """Represents a music file and handles its metadata."""

    def __init__(self, file_path):
        """Initialize a MusicFile instance.

        Args:
            file_path (str): The path to the music file.
        """
        self.file_path = file_path
        self.metadata = self.load_metadata()

    def load_metadata(self):
        """Load metadata from the music file.

        Returns:
            dict: Metadata dictionary.
        """
        try:
            return load_file(self.file_path)
        except Exception as e:
            print(f"Error loading metadata for {self.file_path}: {e}")
            return None

    def get_artist(self):
        """Get the artist from the metadata.

        Returns:
            str: The artist's name.
        """
        if self.metadata and 'artist' in self.metadata:
            artist = self.metadata['artist'].value
            return jibrish_to_hebrew(artist, "heb") if artist else None
            
        elif self.metadata and 'title' in self.metadata:
            title = metadata_file['title'].value
            title = jibrish_to_hebrew(title, "heb")
            for source_name, target_name in self.singer_list:
                if source_name in title:
                    exact = check_exact_name(title, source_name)
                    if exact:
                        artist = target_name
                        return artist
        return None

        for source_name, target_name in self.singer_list:
            if source_name in artist:
                artist = target_name
                return artist

        check_answer = self.check_artist(artist)
        if check_answer == False: return

        return artist



class ArtistFile:

    def __init__(self, file_path):
        self.file_path = file_path
        self.load_singer_list()

    def get_artist(self):
        split_file = self._process_file_name()

        for source_name, target_name in self.singer_list:
            if source_name in split_file:
                exact = check_exact_name(split_file, source_name)
                if exact:
                    artist = target_name
                    return artist



    def load_singer_list(self):
        csv_path = "singer-list.csv"
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            self.singer_list = [tuple(row) for row in csv_reader]

        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            self.singer_list.extend(personal_list)



    def _process_file_name(self):
        split_file = os.path.split(self.file_path)[1]
        split_file = split_file.replace('_', ' ')
        split_file = split_file.replace('-', ' ')
        return split_file

   

    def check_artist(artist):
        """Check if the artist name is valid.

        Args:
            artist (str): The artist's name.

        Returns:
            bool: True if the artist name is valid, False otherwise.
        """
        unusual_list = ["סינגלים", "סינגל", "אבגדהוזחטיכלמנסעפצקרשתךםןץ", "אמן לא ידוע", "טוב", "לא ידוע", "תודה לך ה"]
        unusual_str = similarity_sure(artist, unusual_list, True)
        if artist in unusual_list or unusual_str[0] or len(artist.split()) >= 4 or len(artist.split()) <= 0 \
                or not all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳ " for c in artist):
            return False
        return True




class MusicOrganizer:
    """Organizes music files based on metadata."""

    def __init__(self, dir_path, target_dir, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, tree_folders=False):
        """Initialize a MusicOrganizer instance.

        Args:
            dir_path (str): The path to the source directory.
            target_dir (str): The path to the target directory.
            copy_mode (bool): True for copying, False for moving.
            abc_sort (bool): True for sorting by artist name, False for default sorting.
            exist_only (bool): True to organize existing directories only.
            singles_folder (bool): True to create a 'Singles' folder for each artist.
            tree_folders (bool): True to scan the directory tree, False for only the immediate directory.
        """
        self.dir_path = dir_path
        self.target_dir = target_dir
        self.copy_mode = copy_mode
        self.abc_sort = abc_sort
        self.exist_only = exist_only
        self.singles_folder = singles_folder
        self.tree_folders = tree_folders

    def scan_directory(self):
        """Scan the directory for music files and retrieve artist information."""
        info_list = []
        if self.tree_folders:
            for root, dirs, files in os.walk(self.dir_path):
                for my_file in files:
                    file_path = os.path.join(root, my_file)
                    if my_file.lower().endswith((".mp3", ".wma", ".wav")):
                        music_file = MusicFile(file_path)
                        artist = music_file.get_artist()
                        if artist:
                            info_list.append((file_path, artist))
        else:
            for my_file in os.listdir(self.dir_path):
                file_path = os.path.join(self.dir_path, my_file)
                if os.path.isfile(file_path) and my_file.lower().endswith((".mp3", ".wma", ".wav")):
                    music_file = MusicFile(file_path)
                    artist = music_file.get_artist()
                    if artist:
                        info_list.append((file_path, artist))
        return info_list


    def organize_music(self, info_list):
        """Organize music files based on metadata.

        Args:
            info_list (list): List of tuples containing file paths and artists.
        """
        len_dir = len(info_list)
        len_item = 0

        for file_path, artist in info_list:
            len_item += 1
            show_len = len_item * 100 // len_dir
            print(" " * 30, str(show_len), "% ", "Completed", end='\r')

            target_path = self.get_target_path(artist)
            if target_path and not os.path.isdir(target_path) and not self.exist_only:
                try:
                    os.makedirs(target_path)
                except Exception as e:
                    print(f"Error creating directory {target_path}: {e}")

            if target_path:
                self.move_or_copy_file(file_path, target_path)

    def get_target_path(self, artist):
        """Get the target path based on artist and organization options.

        Args:
            artist (str): The artist's name.

        Returns:
            str: The target path.
        """
        if self.singles_folder and self.abc_sort:
            return os.path.join(self.target_dir, artist[0], artist, "סינגלים")
        elif self.singles_folder:
            return os.path.join(self.target_dir, artist, "סינגלים")
        elif self.abc_sort:
            return os.path.join(self.target_dir, artist[0], artist)
        else:
            return os.path.join(self.target_dir, artist)

    def move_or_copy_file(self, file_path, target_path):
        """Move or copy a file to the target path.

        Args:
            file_path (str): The path to the source file.
            target_path (str): The path to the target directory.
        """
        try:
            if self.copy_mode and os.path.isdir(target_path):
                copy(file_path, target_path)
            elif os.path.isdir(target_path):
                move(file_path, target_path)
        except Exception as e:
            print(f"Error moving/copying file {file_path} to {target_path}: {e}")



def main():
    try:
        dir_path = os.path.join(argv[1])
        target_dir = os.path.join(argv[2])
        copy_mode = True if eval(argv[3]) else False
        tree_folders = True if eval(argv[4]) else False
        singles_folder = True if eval(argv[5]) else False
        exist_only = True if eval(argv[6]) else False
        abc_sort = True if eval(argv[7]) else False

        music_organizer = MusicOrganizer(dir_path, target_dir, copy_mode, abc_sort, exist_only, singles_folder, tree_folders)
        info_list = music_organizer.scan_directory()
        music_organizer.organize_music(info_list)

    except Exception as e:
        print("Error: {}".format(e))


if __name__ == '__main__':
    main()