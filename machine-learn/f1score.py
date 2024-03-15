import spacy
from spacy.training.example import Example

nlp = spacy.load("custom_ner_model1")
examples = []
data = [('אבישי אשל - מרגיש אחר', {'entities': [(0, 9, 'SINGER')]}), ('איציק וינגרטן - צועדים לחופה', {'entities': [(0, 13, 'SINGER')]}), ("אלי הרצליך מחדש את הניגון הוותיק של עולם הישיבות ''מארש חברון''", {'entities': [(0, 10, 'SINGER')]}), ("אלי פרידמן בשיר קייצי חדש ''זה הים גדול''", {'entities': [(0, 10, 'SINGER')]}), ("ארי היל בסינגל חדש ''אוי ירושלים''", {'entities': [(0, 7, 'SINGER')]}), ("בנימין מילר ונכדיו חושפים את הלחן מלפני 30 שנה ''נחמו עמי''", {'entities': [(0, 11, 'SINGER')]}), ("ברוך לוין דודי קאליש ובנצי קלצקין ''והערב נא''", {'entities': [(0, 9, 'SINGER'), (10, 20, 'SINGER'), (22, 33, 'SINGER')]}), ('דובי מייזעלס, הרשי ויינברגר - צלך נאה', {'entities': [(0, 12, 'SINGER'), (14, 27, 'SINGER')]}), ('דודי פלדמן - ניצחתי ואנצח  Dudi Feldman - Nitzachti Vaanatzeach', {'entities': [(0, 10, 'SINGER')]}), ("יודי ביאלוסטוצקי מארח את זמרי החתונות המובילים בניו יורק  ''דער רבי איז געזונט''", {'entities': [(0, 16, 'SINGER')]}), ("יוני גנוט בביצוע מחודש לסינגל הראשון ''בין השמשות''", {'entities': [(0, 9, 'SINGER')]}), ('יוסף נטיב - אוחילה לאל', {'entities': [(0, 9, 'SINGER')]}), ('יורד ועולה דוד בן ארזה', {'entities': []}), ('מהרה', {'entities': []}), ('מוטי וייס - תחזיר את השנים', {'entities': [(0, 9, 'SINGER')]}), ('נותי ליברמן - מילים', {'entities': [(0, 11, 'SINGER')]}), ('נתי לוין - לשיר רק לך', {'entities': [(0, 8, 'SINGER')]}), ('עקיבא - רוצה להתעורר', {'entities': [(0, 5, 'SINGER')]}), ('עקיבא גלב - וטובה היא בעיניו', {'entities': [(0, 5, 'SINGER')]}), ('שולי רנד - אחד כמוך', {'entities': [(0, 8, 'SINGER')]}), ('שוקי סלומון, קובי ברומר, ישי לפידות - שיר האמונה', {'entities': [(0, 11, 'SINGER')]}), ('שלמה קרליבך בבינה מלאכותית - מוצש חי עם מנחם טוקר', {'entities': [(0, 11, 'SINGER')]}), ('שמחה יעקבי - מחרוזת עקיבא', {'entities': [(20, 25, 'SINGER')]})]



for text, annots in data:
    doc = nlp.make_doc(text)
    examples.append(Example.from_dict(doc, annots))
print(nlp.evaluate(examples))

"""
for key, value in results.items():
    if not value is None and not value == 0.0 and not value == {}:
        print(f"{key:<5}: {value}")
"""
        