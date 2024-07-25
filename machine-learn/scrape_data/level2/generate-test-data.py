import random

def read_file(filename):
    with open(filename, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f if line.strip()]

def remove_extension(filename):
    return '.'.join(filename.split('.')[:-1])

def generate_test_data(songs, singers):
    test_data = []
    for song in songs:
        song_without_extension = remove_extension(song)
        singer = random.choice(singers)
        
        if 'SINGER' in song_without_extension:
            new_title = song_without_extension.replace('SINGER', singer)
            start = new_title.index(singer)
            end = start + len(singer)
            entities = [(start, end, 'SINGER')]
        else:
            # If 'SINGER' is not in the title, append the singer to the end
            new_title = f"{song_without_extension} - {singer}"
            start = len(song_without_extension) + 3  # Account for the " - " separator
            end = start + len(singer)
            entities = [(start, end, 'SINGER')]
        
        test_data.append((new_title, {'entities': entities}))
    
    return test_data

def main():
    songs = read_file('songs.txt')
    singers = read_file('singers.txt')
    
    test_data = generate_test_data(songs, singers)
    
    # Print the generated test data
    print("Generated Test Data:")
    for title, annotations in test_data:
        print(f'("{title}", {annotations}),')

if __name__ == "__main__":
    main()