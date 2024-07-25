import random

def read_file(filename):
    with open(filename, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f if line.strip()]

def remove_extension(filename):
    return '.'.join(filename.split('.')[:-1])

class SingerSelector:
    def __init__(self, singers):
        self.original_singers = singers
        self.singers = singers.copy()
        random.shuffle(self.singers)
        self.index = 0

    def get_singer(self):
        if self.index >= len(self.singers):
            self.singers = self.original_singers.copy()
            random.shuffle(self.singers)
            self.index = 0
        singer = self.singers[self.index]
        self.index += 1
        return singer

def generate_test_data(songs, singers):
    test_data = []
    singer_selector = SingerSelector(singers)
    
    for song in songs:
        song_without_extension = remove_extension(song)
        
        if 'SINGER' in song_without_extension:
            new_title = song_without_extension
            entities = []
            while 'SINGER' in new_title:
                singer = singer_selector.get_singer()
                start = new_title.index('SINGER')
                new_title = new_title.replace('SINGER', singer, 1)
                end = start + len(singer)
                entities.append((start, end, 'SINGER'))
                
                # Adjust start positions for subsequent entities
                for i in range(len(entities)):
                    if entities[i][0] > start:
                        entities[i] = (entities[i][0] + len(singer) - 6, entities[i][1] + len(singer) - 6, entities[i][2])
        else:
            # If 'SINGER' is not in the title, append the singer to the end
            singer = singer_selector.get_singer()
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