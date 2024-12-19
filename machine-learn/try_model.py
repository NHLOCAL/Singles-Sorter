from spacy import load



# read name of model
with open("model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

# Load your trained model
nlp = load(model_name)

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

    text_list = ['אבישי אשל - מרגיש אחר', 'איציק וינגרטן - צועדים לחופה', "אלי הרצליך מחדש את הניגון הוותיק של עולם הישיבות ''מארש חברון''", "אלי פרידמן בשיר קייצי חדש ''זה הים גדול''", "ארי היל בסינגל חדש ''אוי ירושלים''", "בנימין מילר ונכדיו חושפים את הלחן מלפני 30 שנה ''נחמו עמי''", "ברוך לוין דודי קאליש ובנצי קלצקין ''והערב נא''", 'דובי מייזעלס, הרשי ויינברגר - צלך נאה', 'דודי פלדמן - ניצחתי ואנצח  Dudi Feldman - Nitzachti Vaanatzeach', "יודי ביאלוסטוצקי מארח את זמרי החתונות המובילים בניו יורק  ''דער רבי איז געזונט''", "יוני גנוט בביצוע מחודש לסינגל הראשון ''בין השמשות''", 'יוסף נטיב - אוחילה לאל', 'יורד ועולה דוד בן ארזה', 'מהרה', 'מוטי וייס - תחזיר את השנים', 'נותי ליברמן - מילים', 'נתי לוין - לשיר רק לך', 'עקיבא - רוצה להתעורר', 'עקיבא גלב - וטובה היא בעיניו', 'שולי רנד - אחד כמוך', 'שוקי סלומון, קובי ברומר, ישי לפידות - שיר האמונה', 'שלמה קרליבך בבינה מלאכותית - מוצש חי עם מנחם טוקר', 'שמחה יעקבי - מחרוזת עקיבא', 'וינתן לעם שיר חדש מאת הזמר המוביל יצחק שטיין', 'אברהם קופולוביץ וגל רוט ביצעו את הלהיט נפשי', 'עידן בוחבוט שר, רפי הרקדן מדגים ריקוד צמאה תשפב']

    for text in text_list:
        machine_learn(text)
