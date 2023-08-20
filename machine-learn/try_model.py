import spacy

# Load your trained model
nlp = spacy.load("custom_ner_model2")

def machine_learn(text):
    # Process the text with the loaded model
    doc = nlp(text)

    # Access the entities recognized by the model
    for entity in doc.ents:
        print(f"{entity.text} \t {entity.label_} ({entity.start_char},{entity.end_char})")

    # Access the tokens in the document
    for token in doc:
        print(token.text, token.pos_, token.tag_, token.dep_, end='')
    print('\n')

text = "מוטי גרינבוים & אוהד מושקוביץ - תינוקות של בית רבן"
machine_learn(text)

text = "יעקב אבינו - ריבונו של עולם"
machine_learn(text)

text = "מאיר כהנוביץ מתוך האלבום החדש: יהושע ליבוביץ, לא ישא גוי"
machine_learn(text)

text = "יונתן שטרן וגלעד שטרן - מי האיש (ווקאלי)"
machine_learn(text)

text = "יוסף לויסון# ויעקב שוואקי - למה תעמוד מרחוק"
machine_learn(text)

text = "להקת שיר ציון - מהרה ה'  ניגון קרלין  נחל נובע  אעופה אשכונה  מי מנוחות"
machine_learn(text)
