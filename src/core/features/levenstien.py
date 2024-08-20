from fuzzywuzzy import process

def artists_from_song(song_title, singer_list):
    split_file = song_title.replace('_', ' ').replace('-', ' ')

    found_artists = []
    for source_name, target_name in singer_list:
        if source_name in split_file:
            found_artists.append(target_name)

    # Fuzzy Matching in Song Title with Word Extraction and Splitting
    if not found_artists:
        for source_name, target_name in singer_list:
            artist_words = source_name.split()
            num_words = len(artist_words)
            song_words = split_file.split()

            # Split song title into overlapping chunks
            song_chunks = []
            for i in range(len(song_words) - num_words + 1):
                chunk = " ".join(song_words[i:i+num_words])
                song_chunks.append(chunk)

            # Fuzzy matching on each chunk
            for chunk in song_chunks:
                match = process.extractOne(source_name, [chunk])
                if match[1] >= 90:  # Adjust the threshold as needed
                    found_artists.append(target_name)
                    matched_word = match[0]
                    print(f"Matched '{matched_word}' in song title to artist '{target_name}' with score {match[1]}")
                    break  # Stop after finding a match

    return found_artists if found_artists else None


# Test Cases
singer_list = [
    ("אייל גולן", "אייל גולן"),
    ("משה דוד וייסמנדל", "משה דוד וייסמנדל"),
    ("עומר אדם", "עומר אדם"),
    ("עדן בן זקן", "עדן בן זקן"),
]

test_songs = [
    "לב שמח השיר החדש של הזמר משה דוד וויסמנדל",
    "איל גולן - שיר חדש",
    "השיר החדש של עומר אדם",
    "עדן בן זק - סינגל חדש",
    "שיר של זמר לא מוכר",
]

# Run tests and print results
for song in test_songs:
    artists = artists_from_song(song, singer_list)
    print(f"Song: {song}")
    if artists:
        print(f"Artists Found: {artists}")
    else:
        print("No artists found.")
    print("-" * 20)