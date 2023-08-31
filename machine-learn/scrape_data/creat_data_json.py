import csv
import json

# Your list of song names
songs_names_file = "songs_list.txt"

with open(songs_names_file, mode='r', newline='', encoding='utf-8') as file:
    song_names = [row.strip() for row in file]  # Added strip to remove extra whitespace

# List of singer names
singer_names_csv = 'singers_list.csv'

with open(singer_names_csv, mode='r', newline='', encoding='utf-8') as file:
    reader = csv.reader(file)
    singer_names = [row[0].strip() for row in reader]

# Create a list to store examples
examples = []

# Iterate through each song name
for song_name in song_names:
    # Initialize start position for singer's name
    start = 0

    # Create a list to store entities
    entities = []

    # Iterate through singer names and find their positions
    for singer in singer_names:
        # Find the position of the singer's name in the song name
        position = song_name.find(singer, start)

        # If the singer's name is found, add it as an entity
        if position != -1:
            entities.append((position, position + len(singer), "SINGER"))
            start = position + len(singer)  # Update start position

    # Only add to examples if entities are present
    if entities:
        example = (song_name, {"entities": entities})
        examples.append(example)

# Replace 'output.json' with the desired output file name
output_file = 'data.json'

# Write the dataset list to the JSON file
with open(output_file, 'w', encoding='utf-8') as f:
    json.dump(examples, f, ensure_ascii=False, indent=2)  # Added indentation for better readability

print(f'Dataset saved to {output_file}')
print(len(examples))
