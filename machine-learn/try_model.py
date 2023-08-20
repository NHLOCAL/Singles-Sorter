from spacy import load
import sys

# Load your trained model
nlp = load("custom_ner_model")

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



if __name__ == "__main__":

    text_list = [
"מוטי גרינבוים & אוהד מושקוביץ - תינוקות של בית רבן",
"יעקב אבינו - ריבונו של עולם",
"מאיר כהנוביץ מתוך האלבום החדש: יהושע ליבוביץ, לא ישא גוי",
"יונתן שטרן וגלעד שטרן - מי האיש (ווקאלי)",
"יוסף לויסון# ויעקב שוואקי - למה תעמוד מרחוק",
"להקת שיר ציון - מהרה ה'  ניגון קרלין  נחל נובע  אעופה אשכונה  מי מנוחות"
]

    for text in text_list:
        machine_learn(text)
