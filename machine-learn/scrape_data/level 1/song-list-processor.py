import csv
from collections import Counter

def read_songs(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f if line.strip()]

def read_artists(csv_file):
    with open(csv_file, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        return [artist.strip() for row in reader for artist in row if artist.strip()]

def remove_duplicates(songs):
    song_counter = Counter(songs)
    duplicates = [song for song, count in song_counter.items() if count > 1]
    unique_songs = list(dict.fromkeys(songs))  # Preserves order
    return unique_songs, duplicates

def filter_songs_by_artists(songs, artists):
    matched_songs = []
    unmatched_songs = []
    for song in songs:
        if any(artist.lower() in song.lower() for artist in artists):
            matched_songs.append(song)
        else:
            unmatched_songs.append(song)
    return matched_songs, unmatched_songs

def write_songs(file_path, songs):
    with open(file_path, 'w', encoding='utf-8') as f:
        for song in songs:
            f.write(f"{song}\n")

def main():
    input_file = 'list_all_songs.txt'
    output_file = 'list_all_songs_clean.txt'
    artists_csv = 'singers_list.csv' 
    unmatched_output_file = 'list_unknown_songs.txt'

    # Read songs and remove duplicates
    songs = read_songs(input_file)
    unique_songs, duplicates = remove_duplicates(songs)

    # Read artists from CSV
    artists = read_artists(artists_csv)

    # Filter songs by artists
    matched_songs, unmatched_songs = filter_songs_by_artists(unique_songs, artists)

    # Write filtered songs to output files
    write_songs(output_file, matched_songs)
    write_songs(unmatched_output_file, unmatched_songs)

    print(f"\nProcessing complete.")
    print(f"Original song count: {len(songs)}")
    print(f"Unique song count: {len(unique_songs)}")
    print(f"Songs matching artists: {len(matched_songs)}")
    print(f"Songs not matching artists: {len(unmatched_songs)}")
    print(f"Matched song list written to: {output_file}")
    print(f"Unmatched song list written to: {unmatched_output_file}")

if __name__ == "__main__":
    main()