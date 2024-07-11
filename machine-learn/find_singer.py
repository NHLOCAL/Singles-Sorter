from spacy import load
import os
from sys import argv

global nlp

# read name of model
with open("model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

# Load your trained model
nlp = load(model_name)

# בצע חיפוש שם אדם במחרוזת באמצעות מודל NER
def find_name(text):
    # Process the text with the loaded model
    doc = nlp(text)

    # Access the entities recognized by the model
    for entity in doc.ents:
        print(f"{entity.text} \t {entity.label_} ({entity.start_char},{entity.end_char})")

    # Access the tokens in the document
    for token in doc:
        print(token.text, token.pos_, token.tag_, token.dep_, end='')
    print('\n')

# בצע מעבר על רשימת קבצים לסריקה   
def files_list(file_path):


    """
    with open(file_path, mode='r', newline='', encoding='utf-8') as file:
        content = file.readlines()    
        songs_names = [song.strip() for song in content] 
    """
    
    songs_names = os.listdir(file_path)
    
    songs_list = []
    
    for filename in songs_names:
        
        # הרץ זיהוי שם אדם מתוך המחרוזת
        singer_name = find_name(filename)
        
        songs_list.append(singer_name)
    
    return songs_list


def print_list(songs_list):

    num = 0

    for singer_name in songs_list:
        if singer_name:
            num += 1
            print(str(num) + ": " + singer_name[0] + "\n" + singer_name[1])    
    
    

if __name__ == '__main__':
    songs_list = files_list(argv[1])
    print_list(songs_list)

