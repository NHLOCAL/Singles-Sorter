import sys
import spacy
from spacy.training.example import Example

# read name of model
if len(sys.argv) > 1:
    model_name = f"custom_ner_model{sys.argv[1]}git"
    print(f"# {model_name}")
else:
    with open("model_name.txt", 'r', encoding='utf-8') as f:
        model_name = f.read()
        print(f'# {model_name}')


# Load your trained model
nlp = spacy.load(model_name)


examples = []
data = [
    ("'יצחק ביגל' והחברים בקומזיץ 'הבדלה' מושקע", {'entities': [(1, 10, 'SINGER')]}),
    ("שגיא גרמון - אנחנו טובים", {'entities': [(0, 10, 'SINGER')]}),
    ("עידו מזוז ויובל כהן מגישים קאבר ''משיח''", {'entities': [(0, 9, 'SINGER'), (11, 19, 'SINGER')]}),
    ("אלירן ווקנין משחרר שיר ''אלוקיי'' מאלבומו החדש ''אז ישיר''", {'entities': [(0, 12, 'SINGER')]}),
    ("מאיר פרץ מגיש סינגל חדש בשם ''נופל שדוד''", {'entities': [(0, 8, 'SINGER')]}),
    ("גבריאל טובול במחרוזת משובחת בשם ''השפעות מוצאי שבת''", {'entities': [(0, 12, 'SINGER')]}),
    ("טל פוגל ושחר כהן במחרוזת קסומה עם מיטב שירי ר''ח", {'entities': [(0, 7, 'SINGER'), (9, 16, 'SINGER')]}),
    ("אופק מתן בסינגל עוצמתי ומחבר ''אל תוותר''", {'entities': [(0, 8, 'SINGER')]}),
    ("אריאל ביסטרוב מפתיע ומשחרר סינגל חדש בסטייל השמור רק לו ''מודים''", {'entities': [(0, 13, 'SINGER')]}),
    ("אברהם קפלון ועידו דניאלי שרים שוובר ועל חסדך", {'entities': [(0, 11, 'SINGER'), (13, 24, 'SINGER')]}),
    ("עידו פולק - מחרוזת לב & נשמה", {'entities': [(0, 9, 'SINGER')]}),
    ("יהודה ברקו בסינגל בכורה חדש ''אני פה להישאר''", {'entities': [(0, 10, 'SINGER')]}),
    ("ליפא’ס ערשטע טאנץ - אנטוני סלומון", {'entities': [(20, 33, 'SINGER')]}),
    ("שגיא אזולאי לא נח לרגע ומגיש מחרוזת חופה מרגשת עם מיטב הלהיטים", {'entities': [(0, 11, 'SINGER')]}),
    ("עידן בוחבוט, פנחס בוקובזה וגדולי הזמר בסדרת חופה מרהיבה ''ברוך הבא''", {'entities': [(0, 11, 'SINGER'), (13, 25, 'SINGER')]}),
    ("בנימין קניאל ותזמורתו של ניר שטרית במחרוזת ''שירי שלמה''", {'entities': [(0, 12, 'SINGER'), (25, 34, 'SINGER')]}),
    ("אביב רואס מפתיע בסינגל חדש ''לבוש שחורים''", {'entities': [(0, 9, 'SINGER')]}),
    ("יהונתן גובי והמכונות - מרבים בשמחה", {'entities': [(0, 11, 'SINGER')]}),
    ("רועי גרבר בסינגל חדש ''כבד את אביך''", {'entities': [(0, 9, 'SINGER')]}),
    ("נוי רזון משיק סינגל חדש מעומק הלב ''נחלי דמעות''", {'entities': [(0, 8, 'SINGER')]}),
    ("בנימין חי ואביו רותם נח בסינגל חדש ''געבליבן מיט דיר''", {'entities': [(0, 9, 'SINGER'), (16, 23, 'SINGER')]}),
    ("ליר אזולוס במחרוזת להיטי בנימין אורלן", {'entities': [(0, 10, 'SINGER'), (25, 37, 'SINGER')]}),
    ("עומר יוסף והחברים בשירה שכולו ערגה וכיסופין ''שלשידעס''", {'entities': [(0, 9, 'SINGER')]}),
    ("Joey-Newcomb-Kaeileh-יקיר אדרי-כאלה", {'entities': [(21, 30, 'SINGER')]}),
    ("יואל הרטמן אושר מנחם אלוקי נשמה MP3", {'entities': [(0, 10, 'SINGER'), (11, 20, 'SINGER')]}),
    ("אשריכם תלמידי חכמים - שי מסגנאו - ווקאלי", {'entities': [(22, 31, 'SINGER')]}),
    ("יעקב איתמר ובני ג'וסף - מסלסלים", {'entities': [(0, 10, 'SINGER'), (12, 21, 'SINGER')]}),
    ("אליאב אבינועם & זאב גפן - מחרוזת ריקודים", {'entities': [(0, 13, 'SINGER'), (16, 23, 'SINGER')]}),
    ("מלך מלכי המלכים ראם צ'קול", {'entities': [(16, 25, 'SINGER')]}),
    ("אייל אניספלד .שדרן שיר", {'entities': [(0, 12, 'SINGER')]}),
    ("אילן עני & אוהד מוגילבסקי - כי אנו עמך", {'entities': [(0, 8, 'SINGER'), (11, 25, 'SINGER')]}),
    ("ניגוני יהודה (נפתלי גרפל) triste esta el rey david", {'entities': [(14, 24, 'SINGER')]}),
    ("נתנאל & גיל רבין והיא שעמדה", {'entities': [(8, 16, 'SINGER')]}),
    ("סט 2019  דיגי אליעזר שמיר (320  kbps) (vipman.xyz)", {'entities': [(14, 25, 'SINGER')]}),
    ("ליעד בק סינגל", {'entities': [(0, 7, 'SINGER')]}),
    ("נתן ארזי עם מדרשת תפארת בחורים - אתיתי לחננך", {'entities': [(0, 8, 'SINGER')]}),
    ("ישראל דרויאן ובן זיונס חזו בני", {'entities': [(0, 12, 'SINGER'), (14, 22, 'SINGER')]}),
    ("גיל אביהו & שגיא פוקס - הושיע אותנו", {'entities': [(0, 9, 'SINGER'), (12, 21, 'SINGER')]}),
    ("גיל בן-ארי ונהוראי שמואלי מקפיצים", {'entities': [(0, 10, 'SINGER'), (12, 25, 'SINGER')]}),
    ("עידן לוטן - עננו - anenu - sruli ginsberg", {'entities': [(0, 9, 'SINGER')]}),
    ("אבנר אבו - והלוואי - Tamir Mizrachi - Vehalevai", {'entities': [(0, 8, 'SINGER')]}),
    ("ברכת המזון המנגינה שהלחין משה אזולאי", {'entities': [(26, 36, 'SINGER')]}),
    ("יוסף ווסקובויניקוב - במסע החיים", {'entities': [(0, 18, 'SINGER')]}),
    ("חיים זליקוביץ שיפקדוני", {'entities': [(0, 13, 'SINGER')]}),
    ("יהונתן פטרזיל ודן צפדיה ''ידע כל פעול''", {'entities': [(0, 13, 'SINGER'), (15, 23, 'SINGER')]}),
    ("יעקב בטיטו ומרדכי סט מרגשים במחרוזת ''ימים נוראים''", {'entities': [(0, 10, 'SINGER'), (12, 20, 'SINGER')]}),
    ("יחיאל מינץ, יועד סויסה - אני מאמין", {'entities': [(0, 10, 'SINGER'), (12, 22, 'SINGER')]}),
    ("נחמן ונדב רויטמן - ניגון וזמרה", {'entities': [(6, 16, 'SINGER')]}),
    ("נתן שפירו בסינגל חדש לימים הנוראים ''אבינו מלכנו''", {'entities': [(0, 9, 'SINGER')]}),
    ("דרור גולן, מקהלת שישו ושמחו - האיש מוילנא", {'entities': [(0, 9, 'SINGER'), (11, 27, 'SINGER')]}),
    ("אריה גיגי, דוד אליאב - כי הנה כחומר", {'entities': [(0, 9, 'SINGER'), (11, 20, 'SINGER')]}),
    ("אימרי שלוסברג & יעקב אמסל - תשרי מיקס", {'entities': [(0, 13, 'SINGER'), (16, 25, 'SINGER')]}),
    ("יהלי מעלם - מצוה טאנץ מושלם", {'entities': [(0, 9, 'SINGER')]}),
    ("בר פריאנטה & אדיר לוי - בריה אחת", {'entities': [(0, 10, 'SINGER'), (13, 21, 'SINGER')]}),
    ("אליה מזור & אבנר אפריאט - שופט כל הארץ", {'entities': [(0, 9, 'SINGER'), (12, 23, 'SINGER')]}),
    ("מרק ויזגן - דאס לעצטע דוכנ'ען", {'entities': [(0, 9, 'SINGER')]}),
    ("Arik Einstein מתן שפר לפעמים(MP3_320K)", {'entities': [(14, 21, 'SINGER')]}),
    ("ירחמיאל דאנה - עוד דקה את נעלמת(MP3_160K)", {'entities': [(0, 12, 'SINGER')]}),
    ("יוסף צברי ויונתן מוטיעי - עוד דקה את נעלמת(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 23, 'SINGER')]}),
    ("לביא גוטפריד - מיכל(MP3_320K)", {'entities': [(0, 12, 'SINGER')]}),
    ("מנחם אזרן - הגולה - Dudu Tassa - Hagole(MP3_320K)", {'entities': [(0, 9, 'SINGER')]}),
    ("עירד נגר - הלא נודע(MP3_320K)", {'entities': [(0, 8, 'SINGER')]}),
    ("איתי חיים ואלירן סבן - זאת שמעל למצופה(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 20, 'SINGER')]}),
    ("בועז איילה - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("הכוכב הבא 2022 -- פאר נח - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(18, 24, 'SINGER')]}),
    ("ידידיה טימן - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 11, 'SINGER')]}),
    ("נחמיה לרנר ומאיר אוזן - סהרה (חי בלייב פארק)(MP3_320K)", {'entities': [(0, 10, 'SINGER'), (12, 21, 'SINGER')]}),
    ("שי מוגס - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 7, 'SINGER')]}),
    ("יהונתן פליי עם דודי יוזוק - סתלבט בקיבוץ(MP3_320K)", {'entities': [(0, 11, 'SINGER'), (15, 25, 'SINGER')]}),
    ("יעקב מגידש מעלה מעלה Svika Pick(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("עידו אלמקיס לא_נרגע_Avihai_Hollender_Lo_Nirga", {'entities': [(0, 11, 'SINGER')]}),
    ("ניצן קופף, בניה בורובסקי ויצחק שיינרמן - רואים רחוק רואים שקוף", {'entities': [(0, 9, 'SINGER'), (11, 24, 'SINGER'), (26, 38, 'SINGER')]}),
    ("אמיתי אלבז...לכה דודי", {'entities': [(0, 10, 'SINGER')]}),
]


for text, annots in data:
    doc = nlp.make_doc(text)
    examples.append(Example.from_dict(doc, annots))
print(nlp.evaluate(examples))



def analyze_errors(nlp, data):
    errors = []
    for text, true_entities in data:
        doc = nlp(text)
        pred_entities = [(ent.start_char, ent.end_char, ent.label_) for ent in doc.ents]
        
        # Check for false positives and false negatives
        for start, end, label in set(pred_entities + true_entities['entities']):
            pred = (start, end, label) in pred_entities
            true = (start, end, label) in true_entities['entities']
            
            if pred != true:
                error_type = "False Positive" if pred else "False Negative"
                errors.append({
                    "text": text,
                    "error_type": error_type,
                    "entity": text[start:end],
                    "start": start,
                    "end": end,
                    "label": label
                })
    
    return errors

errors = analyze_errors(nlp, data)

print("\nIncorrect Identifications:")
for error in errors:
    print(f"Text: {error['text']}")
    print(f"Error Type: {error['error_type']}")
    print(f"Entity: {error['entity']}")
    print(f"Position: {error['start']}:{error['end']}")
    #print(f"Label: {error['label']}")
    print()

print(f"Total errors: {len(errors)}")