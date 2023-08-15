from spacy import load
import os
from sys import argv

global nlp

nlp = load("custom_ner_model")

# בצע חיפוש שם אדם במחרוזת באמצעות מודל NER
def find_name(string):
    # improved_model / custom_ner_model / he_ner_news_trf
    
    # הסרת תווים המפריעים לניתוח תקין
    text = string.replace('-', ' ')

    doc = nlp(text)
    
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
def files_list(dir_path):

    songs_list = []
    
    for filename in os.listdir(dir_path):
        
        # הסרת סיומת שם הקובץ
        my_str = os.path.splitext(filename)[0]
        
        # הרץ זיהוי שם אדם מתוך המחרוזת
        singer_name = find_name(my_str)
        
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

