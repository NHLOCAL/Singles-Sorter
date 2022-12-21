import music_tag
import os

class Song:
    def __init__(self, file_path, artist):
        self.file_path = file_path
        self.artist = artist


class SongScanner:
    def __init__(self, root_dir):
        self.root_dir = root_dir
    
    def _load_song_metadata(self, file_path):
        try:
            artist_file = music_tag.load_file(file_path)
            artist = artist_file['artist']

            if artist:
                return Song(file_path, artist)
        except:
            pass
    
    def _scan_dir(self, dir_path):
        songs = []
        for file in os.listdir(dir_path):
            file_path = os.path.join(dir_path, file)
            if os.path.isfile(file_path) and (file_path.endswith(".mp3") or file_path.endswith(".wav") or file_path.endswith(".wma")):
                song = self._load_song_metadata(file_path)
                if song:
                    songs.append(song)
            elif os.path.isdir(file_path):
                songs.extend(self._scan_dir(file_path))
        return songs
    
    def scan(self):
        songs = []
        for song in self._scan_dir(self.root_dir):
            songs.append((song.file_path, song.artist))
        return songs

    

class SongCopier:
    def __init__(self, target_dir):
        self.target_dir = target_dir
    
    def copy(self, songs):
        for song in songs:
            target_path = os.path.join(self.target_dir, song.artist)
            if not os.path.exists(target_path):
                os.makedirs(target_path)
            target_path = os.path.join(target_path, os.path.basename(song.file_path))
            if not os.path.exists(target_path):
                os.link(song.file_path, target_path)

import ftfy

def fix_encoding(text):
    return ftfy.fix_text(text, fix_entities=True)


def main(root_dir, target_dir, copy_mode):
    scanner = SongScanner(root_dir)
    songs = scanner.scan()
    if copy_mode:
        copier = SongCopier(target_dir)
        copier.copy(songs)
    else:
        artists = set([fix_encoding(str(song[1])) for song in songs])
        print(f"Found the following artists: {', '.join([str(artist) for artist in artists])}")


if __name__ == "__main__":
    import sys
    root_dir = sys.argv[1]
    target_dir = sys.argv[2] if len(sys.argv) > 2 else None
    copy_mode = bool(sys.argv[3]) if len(sys.argv) > 3 else False
    main(root_dir, target_dir, copy_mode)
