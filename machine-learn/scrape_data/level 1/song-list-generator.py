import os
from pathlib import Path

def collect_songs(root_dir):
    songs = set()
    for root, dirs, files in os.walk(root_dir):
        for file in files:
            if file.lower().endswith(('.mp3', '.wav', '.flac', '.m4a')):  # Add or remove file extensions as needed
                song_name = os.path.splitext(file)[0]
                songs.add(song_name)
    return sorted(songs)

def write_songs_to_file(songs, output_file):
    existing_songs = set()
    
    # Read existing content if the file exists
    if os.path.exists(output_file):
        with open(output_file, 'r', encoding='utf-8') as f:
            existing_songs = set(line.strip() for line in f)
    
    # Append new songs to the file
    with open(output_file, 'a', encoding='utf-8') as f:
        for song in songs:
            if song not in existing_songs:
                f.write(f"{song}\n")
                existing_songs.add(song)

def main():
    root_dir = input("Enter the root directory to scan for songs: ")
    output_file = "songs-list1.txt"

    if not os.path.exists(root_dir):
        print("The specified directory does not exist.")
        return

    songs = collect_songs(root_dir)
    write_songs_to_file(songs, output_file)
    print(f"Song list has been updated in {output_file}")

if __name__ == "__main__":
    main()