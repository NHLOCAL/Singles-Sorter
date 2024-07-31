import csv
from collections import Counter
import re

def read_songs(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f if line.strip()]

def read_artists(csv_file):
    with open(csv_file, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        return set(artist.strip().lower() for row in reader for artist in row if artist.strip())

def remove_duplicates(songs):
    return list(dict.fromkeys(songs))  # Preserves order and removes duplicates

def create_artist_regex(artists):
    escaped_artists = [re.escape(artist) for artist in artists]
    return re.compile('|'.join(escaped_artists), re.IGNORECASE)

def process_songs(songs, artist_regex):
    matched_songs = []
    unmatched_songs = []
    replaced_songs = []

    for song in songs:
        if artist_regex.search(song):
            matched_songs.append(song)
            replaced_songs.append(artist_regex.sub('SINGER', song))
        else:
            unmatched_songs.append(song)

    return matched_songs, unmatched_songs, replaced_songs

def write_songs(file_path, songs):
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(songs))

def main():
    input_file = 'list_all_songs.txt'
    clean_output_file = 'list_all_songs_clean.txt'
    final_output_file = 'list_all_songs_with_singer.txt'
    artists_csv = 'singers_list.csv' 
    unmatched_output_file = 'list_unknown_songs.txt'

    # Read songs and remove duplicates
    songs = remove_duplicates(read_songs(input_file))

    # Read artists from CSV and create regex
    artists = read_artists(artists_csv)
    artist_regex = create_artist_regex(artists)

    # Process songs
    matched_songs, unmatched_songs, replaced_songs = process_songs(songs, artist_regex)

    # Write output files
    write_songs(clean_output_file, matched_songs)
    write_songs(unmatched_output_file, unmatched_songs)
    write_songs(final_output_file, replaced_songs)

    print(f"\nProcessing complete.")
    print(f"Unique song count: {len(songs)}")
    print(f"Songs matching artists: {len(matched_songs)}")
    print(f"Songs not matching artists: {len(unmatched_songs)}")
    print(f"Matched song list written to: {clean_output_file}")
    print(f"Unmatched song list written to: {unmatched_output_file}")
    print(f"Song list with 'SINGER' replacements written to: {final_output_file}")

if __name__ == "__main__":
    main()