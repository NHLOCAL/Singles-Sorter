import csv

def reduce_song_list(songs_file, singer_csv, output_file, max_singer_count=5):
    # Read the list of singers from the CSV file
    singer_names = []
    with open(singer_csv, mode='r', newline='', encoding='utf-8') as file:
        reader = csv.reader(file)
        for row in reader:
            singer_names.append(row[0].strip())

    # Read the list of songs from the text file
    with open(songs_file, mode='r', encoding='utf-8') as file:
        song_names = [line.strip() for line in file]

    # Count occurrences of each singer in song names
    singer_counts = {singer: 0 for singer in singer_names}
    for song in song_names:
        for singer in singer_names:
            if singer in song:
                singer_counts[singer] += 1

    # Identify singers to remove
    singers_to_remove = set()
    for singer, count in singer_counts.items():
        if count > max_singer_count:
            singers_to_remove.add(singer)

    # Create a reduced song list
    reduced_songs = []
    for song in song_names:
        has_other_singer = any(singer in song for singer in singer_names if singer not in singers_to_remove)
        if not any(singer in song for singer in singers_to_remove) or has_other_singer:
            reduced_songs.append(song)

    # Write the reduced song list to the output file
    with open(output_file, mode='w', encoding='utf-8') as file:
        for song in reduced_songs:
            file.write(song + '\n')

# Example usage
songs_file = 'songs_list.txt'
singer_csv_file = 'singers_list.csv'
output_file = 'reduced_songs.txt'

reduce_song_list(songs_file, singer_csv_file, output_file, max_singer_count=5)
print(f"Reduced song list saved to {output_file}")
