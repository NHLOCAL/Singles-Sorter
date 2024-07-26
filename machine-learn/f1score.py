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
    ("'שחר כהן' והחברים בקומזיץ 'הבדלה' מושקע", {'entities': [(1, 8, 'SINGER')]}),
    ("אלון דגני - אנחנו טובים", {'entities': [(0, 9, 'SINGER')]}),
    ("יוגב אמסלם ועירד נגר מגישים קאבר ''משיח''", {'entities': [(0, 10, 'SINGER'), (12, 20, 'SINGER')]}),
    ("רונן דולינסקי משחרר שיר ''אלוקיי'' מאלבומו החדש ''אז ישיר''", {'entities': [(0, 13, 'SINGER')]}),
    ("נתן שפירו מגיש סינגל חדש בשם ''נופל שדוד''", {'entities': [(0, 9, 'SINGER')]}),
    ("בנימין קניאל במחרוזת משובחת בשם ''השפעות מוצאי שבת''", {'entities': [(0, 12, 'SINGER')]}),
    ("אליהו מידן ואביתר רטנר במחרוזת קסומה עם מיטב שירי ר''ח", {'entities': [(0, 10, 'SINGER'), (12, 22, 'SINGER')]}),
    ("חיים כהן בסינגל עוצמתי ומחבר ''אל תוותר''", {'entities': [(0, 8, 'SINGER')]}),
    ("בני ג'וסף מפתיע ומשחרר סינגל חדש בסטייל השמור רק לו ''מודים''", {'entities': [(0, 9, 'SINGER')]}),
    ("יובל כהן ואושר מנחם שרים שוובר ועל חסדך", {'entities': [(0, 8, 'SINGER'), (10, 19, 'SINGER')]}),
    ("אמיתי אלבז - מחרוזת לב & נשמה", {'entities': [(0, 10, 'SINGER')]}),
    ("איתמר כהן בסינגל בכורה חדש ''אני פה להישאר''", {'entities': [(0, 9, 'SINGER')]}),
    ("ליפא’ס ערשטע טאנץ - נפתלי גרפל", {'entities': [(20, 30, 'SINGER')]}),
    ("עידן בוחבוט לא נח לרגע ומגיש מחרוזת חופה מרגשת עם מיטב הלהיטים", {'entities': [(0, 11, 'SINGER')]}),
    ("רועי גרבר, עמית מס וגדולי הזמר בסדרת חופה מרהיבה ''ברוך הבא''", {'entities': [(0, 9, 'SINGER'), (11, 18, 'SINGER')]}),
    ("יעקב איתמר ותזמורתו של אריאל טבול במחרוזת ''שירי שלמה''", {'entities': [(0, 10, 'SINGER'), (23, 33, 'SINGER')]}),
    ("יצחק שיינרמן מפתיע בסינגל חדש ''לבוש שחורים''", {'entities': [(0, 12, 'SINGER')]}),
    ("זיו אלמקיאס והמכונות - מרבים בשמחה", {'entities': [(0, 11, 'SINGER')]}),
    ("יהונתן פטרזיל בסינגל חדש ''כבד את אביך''", {'entities': [(0, 13, 'SINGER')]}),
    ("מתן שמו משיק סינגל חדש מעומק הלב ''נחלי דמעות''", {'entities': [(0, 7, 'SINGER')]}),
    ("זוהר קרופרו ואביו חיים הרפז בסינגל חדש ''געבליבן מיט דיר''", {'entities': [(0, 11, 'SINGER'), (18, 27, 'SINGER')]}),
    ("אימרי שלוסברג במחרוזת להיטי דרור גולן", {'entities': [(0, 13, 'SINGER'), (28, 37, 'SINGER')]}),
    ("איתמר סיטבון והחברים בשירה שכולו ערגה וכיסופין ''שלשידעס''", {'entities': [(0, 12, 'SINGER')]}),
    ("Joey-Newcomb-Kaeileh-נחמיה לרנר-כאלה", {'entities': [(21, 31, 'SINGER')]}),
    ("מאיר אוזן טל פוגל אלוקי נשמה MP3", {'entities': [(0, 9, 'SINGER'), (10, 17, 'SINGER')]}),
    ("אשריכם תלמידי חכמים - בניה בורובסקי - ווקאלי", {'entities': [(22, 35, 'SINGER')]}),
    ("איתן אילני ויצחק ביגל - מסלסלים", {'entities': [(0, 10, 'SINGER'), (12, 21, 'SINGER')]}),
    ("אייל בסטקר & אבנר אבו - מחרוזת ריקודים", {'entities': [(0, 10, 'SINGER'), (13, 21, 'SINGER')]}),
    ("מלך מלכי המלכים מאיר פיינגולד", {'entities': [(16, 29, 'SINGER')]}),
    ("יוסף צברי .שדרן שיר", {'entities': [(0, 9, 'SINGER')]}),
    ("רועי עיני & סהר שיבר - כי אנו עמך", {'entities': [(0, 9, 'SINGER'), (12, 20, 'SINGER')]}),
    ("ניגוני יהודה (אדיר לוי) triste esta el rey david", {'entities': [(14, 22, 'SINGER')]}),
    ("נתנאל & נהוראי שמואלי והיא שעמדה", {'entities': [(8, 21, 'SINGER')]}),
    ("סט 2019  דיגי אריאל קורלנד (320  kbps) (vipman.xyz)", {'entities': [(14, 26, 'SINGER')]}),
    ("ידידיה אבוחצירא סינגל", {'entities': [(0, 15, 'SINGER')]}),
    ("מרדכי פארי עם מדרשת תפארת בחורים - אתיתי לחננך", {'entities': [(0, 10, 'SINGER')]}),
    ("שון שפיצר והאחים שמואלביץ' חזו בני", {'entities': [(0, 9, 'SINGER'), (11, 26, 'SINGER')]}),
    ("יעקב בטיטו & יעקב סיבוני - הושיע אותנו", {'entities': [(0, 10, 'SINGER'), (13, 24, 'SINGER')]}),
    ("גבריאל טובול ובועז איילה מקפיצים", {'entities': [(0, 12, 'SINGER'), (14, 24, 'SINGER')]}),
    ("פאר נח - עננו - anenu - sruli ginsberg", {'entities': [(0, 6, 'SINGER')]}),
    ("עומר יוסף - והלוואי - Tamir Mizrachi - Vehalevai", {'entities': [(0, 9, 'SINGER')]}),
    ("ברכת המזון המנגינה שהלחין מרדכי יצחקי", {'entities': [(26, 37, 'SINGER')]}),
    ("שגיא גרמון - במסע החיים", {'entities': [(0, 10, 'SINGER')]}),
    ("להקת אש ומים שיפקדוני", {'entities': [(0, 12, 'SINGER')]}),
    ("רותם נח ועידו פולק ''ידע כל פעול''", {'entities': [(0, 7, 'SINGER'), (9, 18, 'SINGER')]}),
    ("יהודה וויספיש ואבנר גולה מרגשים במחרוזת ''ימים נוראים''", {'entities': [(0, 13, 'SINGER'), (15, 24, 'SINGER')]}),
    ("משה אזולאי, גיל רבין - אני מאמין", {'entities': [(0, 10, 'SINGER'), (12, 20, 'SINGER')]}),
    ("נחמן וחיים זליקוביץ - ניגון וזמרה", {'entities': [(6, 19, 'SINGER')]}),
    ("אפיק סעד בסינגל חדש לימים הנוראים ''אבינו מלכנו''", {'entities': [(0, 8, 'SINGER')]}),
    ("בנימין אורלן, אברהם טפירו - האיש מוילנא", {'entities': [(0, 12, 'SINGER'), (14, 25, 'SINGER')]}),
    ("אלי חופי, יצחק צפרי - כי הנה כחומר", {'entities': [(0, 8, 'SINGER'), (10, 19, 'SINGER')]}),
    ("עידן לוטן & מרדכי סט - תשרי מיקס", {'entities': [(0, 9, 'SINGER'), (12, 20, 'SINGER')]}),
    ("איתי גלינסקי - מצוה טאנץ מושלם", {'entities': [(0, 12, 'SINGER')]}),
    ("אלעד ויסמן & בנימין חי - בריה אחת", {'entities': [(0, 10, 'SINGER'), (13, 22, 'SINGER')]}),
    ("מקהלת שישו ושמחו & אלירן סבן - שופט כל הארץ", {'entities': [(0, 16, 'SINGER'), (19, 28, 'SINGER')]}),
    ("מתן שפר - דאס לעצטע דוכנ'ען", {'entities': [(0, 7, 'SINGER')]}),
    ("Arik Einstein מיכל טאדסה לפעמים(MP3_320K)", {'entities': [(14, 24, 'SINGER')]}),
    ("שי מסגנאו - עוד דקה את נעלמת(MP3_160K)", {'entities': [(0, 9, 'SINGER')]}),
    ("אמנון קרן ותום ביטון - עוד דקה את נעלמת(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 20, 'SINGER')]}),
    ("אלירן ווקנין - מיכל(MP3_320K)", {'entities': [(0, 12, 'SINGER')]}),
    ("מור שניידר - הגולה - Dudu Tassa - Hagole(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("יובל שפיגלמן - הלא נודע(MP3_320K)", {'entities': [(0, 12, 'SINGER')]}),
    ("אהרון חג'בי ואליה מזור - זאת שמעל למצופה(MP3_320K)", {'entities': [(0, 11, 'SINGER'), (13, 22, 'SINGER')]}),
    ("ירחמיאל דאנה - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(0, 12, 'SINGER')]}),
    ("הכוכב הבא 2022 -- אנטוני סלומון - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(18, 31, 'SINGER')]}),
    ("משה בוזגלו - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("עזרא עוזרי וגיא סוזנה - סהרה (חי בלייב פארק)(MP3_320K)", {'entities': [(0, 10, 'SINGER'), (12, 21, 'SINGER')]}),
    ("מאיר פרץ - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 8, 'SINGER')]}),
    ("פול טראנק עם יהודה טורקניץ - סתלבט בקיבוץ(MP3_320K)", {'entities': [(13, 26, 'SINGER')]}),
    ("דניאל שנטל מעלה מעלה Svika Pick(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("יוסף פז לא_נרגע_Avihai_Hollender_Lo_Nirga", {'entities': [(0, 7, 'SINGER')]}),
    ("אופק ריינר, אלחנן מאיר ומנחם אזרן - רואים רחוק רואים שקוף", {'entities': [(0, 10, 'SINGER'), (12, 22, 'SINGER'), (24, 33, 'SINGER')]}),
    ("שאול פנחסוב...לכה דודי", {'entities': [(0, 11, 'SINGER')]}),
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