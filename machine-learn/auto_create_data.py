import os
import pandas as pd
import re
import spacy
from spacy.tokens import Doc

# Replace 'singers_list.csv' with the actual path to your CSV file with the list of singers
singers_csv = 'singers_list.csv'
song_folder = r"C:\Users\משתמש\Documents\song_list"
output_csv = 'training_data.csv'

# Read the CSV file with the list of singers into a pandas DataFrame
singers_df = pd.read_csv(singers_csv)

# Load the spaCy model for tokenization
nlp = spacy.blank("he")

# Regular expression pattern to find the singer's name in the song filename
# Modify the pattern as needed based on the structure of your song filenames
singer_pattern = re.compile(r'(' + '|'.join(map(re.escape, singers_df['Singer'].tolist())) + r')', re.IGNORECASE)

# Create an empty list to store the training data
training_data = []

# Iterate over files in the song folder
for root, dirs, files in os.walk(song_folder):
    for filename in files:
        if filename.lower().endswith('.mp3'):
            full_song_name = os.path.splitext(filename)[0]
            matches = singer_pattern.finditer(full_song_name)
            for match in matches:
                singer_name = match.group()
                start_position = match.start()
                end_position = match.end()

                # Use spaCy tokenizer to get the token-based positions
                doc = nlp(full_song_name)
                start_token = None
                end_token = None
                for token in doc:
                    if token.idx == start_position:
                        start_token = token.i
                    if token.idx + len(token.text) == end_position:
                        end_token = token.i
                if start_token is not None and end_token is not None:
                    # Append the data to the training_data list
                    training_data.append([full_song_name, singer_name, start_token, end_token])

# Create a pandas DataFrame from the training_data list
columns = ['Column A', 'Column B', 'Start Position', 'End Position']
df = pd.DataFrame(training_data, columns=columns)

# Save the DataFrame to a CSV file
df.to_csv(output_csv, index=False)
