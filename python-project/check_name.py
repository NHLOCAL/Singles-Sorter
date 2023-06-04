import os
import csv

def artist_from_song(my_file):
    """
    The function checks the artist's name in the file name based on a database and stores the artist's name in a variable.
    If the name does not exist, it scans the metadata of the song and stores the artist's name in the variable.

    Args:
        my_file (str): The file name to be scanned.

    Returns:
        str: The value containing the artist's name from the file.
    """

    # Get the file name without the full path
    split_file = os.path.split(my_file)[1]
    split_file = os.path.splitext(split_file)[0]

    # Remove unwanted characters from the file name
    split_file = split_file.replace('_', ' ')
    split_file = split_file.replace('-', ' ')

    # Import the list of singers from a CSV file
    if 'singer_list' not in globals():
        csv_path = "singer-list.csv"
        global singer_list
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]

        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)

    # Iterate over the list of names and check if any of them exist in the file name
    for source_name, target_name in singer_list:
        if source_name in split_file:
            artist = target_name

            # Check if the singer's name appears at the beginning and end of the file name
            if split_file.startswith(source_name) and split_file.endswith(source_name):
                    return artist

            # Check if the singer's name appears at the beginning of the file name
            elif split_file.startswith(source_name):
                next_char = split_file[len(source_name)]
                if next_char in [" ", ".", ","]:
                    return artist

            # Check if the singer's name appears at the end of the file name
            elif split_file.endswith(source_name):
                previous_char = split_file[split_file.index(source_name) - 1]
                if previous_char in [" ", "ו"]:
                    return artist

            # Check if the singer's name appears in the middle of the file name
            elif source_name in split_file[1:-1]:
                index = split_file.index(source_name)
                previous_char = split_file[index - 1]
                next_char = split_file[index + len(source_name)]
                if previous_char in [" ", "("] and next_char in [" ", ".", ",", ")"]:
                    return artist

    return None

if __name__ == '__main__':
   print(artist_from_song('-אברהם פריד.mp3'))
