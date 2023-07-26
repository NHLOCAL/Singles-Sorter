from spacy import load
import os


# בצע חיפוש שם אדם במחרוזת באמצעות מודל NER
def find_name(string):
    nlp = load("he_ner_news_trf")
    text = string

    doc = nlp(text)
    
    '''
    for entity in doc.ents:
        print(f"{entity.text} \t {entity.label_}: {entity._.confidence_score:.4f} ({entity.start_char},{entity.end_char})")
    '''
    
    for entity in doc.ents:
        if entity.label_ == 'PERS':
            return entity.text
    return

# בצע מעבר על רשימת קבצים לסריקה   
def files_list(dir_path):
    
    for filename in os.listdir(dir_path):
        
        # הסרת סיומת שם הקובץ
        my_str = os.path.splitext(filename)[0]
        
        # הרץ זיהוי שם אדם מתוך המחרוזת
        singer_name = find_name(my_str)
        print(singer_name)


if __name__ == '__main__':
    files_list(r'C:\Users\משתמש\Music\סינגלים')

