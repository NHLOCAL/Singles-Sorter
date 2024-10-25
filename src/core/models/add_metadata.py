import spacy

# טען את המודל הקיים או צור חדש
nlp = spacy.load("singer_ner_he")

# עדכון המטא נתונים
nlp.meta['name'] = 'singer_ner_he'
nlp.meta['version'] = '0.23.0'
nlp.meta['description'] = 'Model for recognizing singer names in Hebrew song titles'
nlp.meta['author'] = 'nhlocal'
nlp.meta['email'] = 'nh.local11@gmail.com'
nlp.meta['license'] = 'MIT'
nlp.meta['tags'] = ['NER', 'Hebrew', 'Singer', 'Named Entity Recognition', 'Text Classification']

# שמור את המודל מחדש
nlp.to_disk("singer_ner_he")
print("the model with metadata saving to disk!")