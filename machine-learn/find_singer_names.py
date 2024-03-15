from spacy import load
import os
from sys import argv

global nlp

# improved_model / custom_ner_model / he_ner_news_trf
nlp = load("custom_ner_model2")

# בצע חיפוש שם אדם במחרוזת באמצעות מודל NER
def find_name(string):

    doc = nlp(string)
    
    '''
    for entity in doc.ents:
        print(f"{entity.text} \t {entity.label_}: {entity._.confidence_score:.4f} ({entity.start_char},{entity.end_char})")
    '''
    
    singers_list = []
    
    # מעבר על רשימת אובייקטים המכילים מידע על המחרוזת
    for entity in doc.ents:
        if entity.label_ == 'SINGER':
            singers_list.append(entity.text)
    
    if singers_list:
        singers_list.append(text)
        return singers_list
    else:
        return

# בצע מעבר על רשימת קבצים לסריקה   
def files_list(file_path):

    with open(file_path, mode='r', newline='', encoding='utf-8') as file:
        content = file.readlines()    
        songs_names = [song.strip() for song in content]    
    
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

