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
    ("'יובל חבקוק' והחברים בקומזיץ 'הבדלה' מושקע", {'entities': [(1, 11, 'SINGER')]}),
    ("פאר נח - אנחנו טובים", {'entities': [(0, 6, 'SINGER')]}),
    ("אוהד רחבי ויובל סיביליה מגישים קאבר ''משיח''", {'entities': [(0, 9, 'SINGER'), (11, 23, 'SINGER')]}),
    ("ליר אזולוס משחרר שיר ''אלוקיי'' מאלבומו החדש ''אז ישיר''", {'entities': [(0, 10, 'SINGER')]}),
    ("בר פריאנטה מגיש סינגל חדש בשם ''נופל שדוד''", {'entities': [(0, 10, 'SINGER')]}),
    ("אוהד מוגילבסקי במחרוזת משובחת בשם ''השפעות מוצאי שבת''", {'entities': [(0, 14, 'SINGER')]}),
    ("יצחק ביגל וזיו אלמקיאס במחרוזת קסומה עם מיטב שירי ר''ח", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("שגיא אזולאי בסינגל עוצמתי ומחבר ''אל תוותר''", {'entities': [(0, 11, 'SINGER')]}),
    ("עידו מזוז מפתיע ומשחרר סינגל חדש בסטייל השמור רק לו ''מודים''", {'entities': [(0, 9, 'SINGER')]}),
    ("אברהם קפלון ומאיר אוזן שרים שוובר ועל חסדך", {'entities': [(0, 11, 'SINGER'), (13, 22, 'SINGER')]}),
    ("בנימין חי - מחרוזת לב & נשמה", {'entities': [(0, 9, 'SINGER')]}),
    ("אברהם מליחי בסינגל בכורה חדש ''אני פה להישאר''", {'entities': [(0, 11, 'SINGER')]}),
    ("ליפא’ס ערשטע טאנץ - גיא סוזנה", {'entities': [(20, 29, 'SINGER')]}),
    ("רני הלוי לא נח לרגע ומגיש מחרוזת חופה מרגשת עם מיטב הלהיטים", {'entities': [(0, 8, 'SINGER')]}),
    ("משה אזולאי, אביתר רטנר וגדולי הזמר בסדרת חופה מרהיבה ''ברוך הבא''", {'entities': [(0, 10, 'SINGER'), (12, 22, 'SINGER')]}),
    ("יהלי מעלם ותזמורתו של אלחנן מאיר במחרוזת ''שירי שלמה''", {'entities': [(0, 9, 'SINGER'), (22, 32, 'SINGER')]}),
    ("יעקב איתמר מפתיע בסינגל חדש ''לבוש שחורים''", {'entities': [(0, 10, 'SINGER')]}),
    ("ראם צורף והמכונות - מרבים בשמחה", {'entities': [(0, 8, 'SINGER')]}),
    ("גיל בן-ארי בסינגל חדש ''כבד את אביך''", {'entities': [(0, 10, 'SINGER')]}),
    ("ליעד בק משיק סינגל חדש מעומק הלב ''נחלי דמעות''", {'entities': [(0, 7, 'SINGER')]}),
    ("אלירן ווקנין ואביו יוסף ווסקובויניקוב בסינגל חדש ''געבליבן מיט דיר''", {'entities': [(0, 12, 'SINGER'), (19, 37, 'SINGER')]}),
    ("חיים כהן במחרוזת להיטי אברהם טפירו", {'entities': [(0, 8, 'SINGER'), (23, 34, 'SINGER')]}),
    ("אליהו מידן והחברים בשירה שכולו ערגה וכיסופין ''שלשידעס''", {'entities': [(0, 10, 'SINGER')]}),
    ("Joey-Newcomb-Kaeileh-שמעון פיק-כאלה", {'entities': [(21, 30, 'SINGER')]}),
    ("אדיר לוי שלום גרמאי אלוקי נשמה MP3", {'entities': [(0, 8, 'SINGER'), (9, 19, 'SINGER')]}),
    ("אשריכם תלמידי חכמים - תמיר טל - ווקאלי", {'entities': [(22, 29, 'SINGER')]}),
    ("אורן נעים ומאיר פרץ - מסלסלים", {'entities': [(0, 9, 'SINGER'), (11, 19, 'SINGER')]}),
    ("שחר כהן & יהונתן כהן - מחרוזת ריקודים", {'entities': [(0, 7, 'SINGER'), (10, 20, 'SINGER')]}),
    ("מלך מלכי המלכים רוי ווסה", {'entities': [(16, 24, 'SINGER')]}),
    ("שי מסגנאו .שדרן שיר", {'entities': [(0, 9, 'SINGER')]}),
    ("אופיר שפיגל & שאול פנחסוב - כי אנו עמך", {'entities': [(0, 11, 'SINGER'), (14, 25, 'SINGER')]}),
    ("ניגוני יהודה (שגיא גרמון) triste esta el rey david", {'entities': [(14, 24, 'SINGER')]}),
    ("נתנאל & אבנר אפריאט והיא שעמדה", {'entities': [(8, 19, 'SINGER')]}),
    ("סט 2019  דיגי מרק ויזגן (320  kbps) (vipman.xyz)", {'entities': [(14, 23, 'SINGER')]}),
    ("יעקב בטיטו סינגל", {'entities': [(0, 10, 'SINGER')]}),
    ("נתן שפירו עם מדרשת תפארת בחורים - אתיתי לחננך", {'entities': [(0, 9, 'SINGER')]}),
    ("זיו כהן ורותם נח חזו בני", {'entities': [(0, 7, 'SINGER'), (9, 16, 'SINGER')]}),
    ("רועי עיני & נהוראי שמואלי - הושיע אותנו", {'entities': [(0, 9, 'SINGER'), (12, 25, 'SINGER')]}),
    ("אפיק סעד ואיתן אילני מקפיצים", {'entities': [(0, 8, 'SINGER'), (10, 20, 'SINGER')]}),
    ("רועי גרבר - עננו - anenu - sruli ginsberg", {'entities': [(0, 9, 'SINGER')]}),
    ("בועז איילה - והלוואי - Tamir Mizrachi - Vehalevai", {'entities': [(0, 10, 'SINGER')]}),
    ("ברכת המזון המנגינה שהלחין תומר סבח", {'entities': [(26, 34, 'SINGER')]}),
    ("בן זיונס - במסע החיים", {'entities': [(0, 8, 'SINGER')]}),
    ("שון שפיצר שיפקדוני", {'entities': [(0, 9, 'SINGER')]}),
    ("אלון דגני וישראל דרויאן ''ידע כל פעול''", {'entities': [(0, 9, 'SINGER'), (11, 23, 'SINGER')]}),
    ("אילן עני ועידו לובינסקי מרגשים במחרוזת ''ימים נוראים''", {'entities': [(0, 8, 'SINGER'), (10, 23, 'SINGER')]}),
    ("זוהר קרופרו, עידו אלמקיס - אני מאמין", {'entities': [(0, 11, 'SINGER'), (13, 24, 'SINGER')]}),
    ("נחמן ועומר יוסף - ניגון וזמרה", {'entities': [(6, 15, 'SINGER')]}),
    ("בנימין קניאל בסינגל חדש לימים הנוראים ''אבינו מלכנו''", {'entities': [(0, 12, 'SINGER')]}),
    ("אסף תמיר, אימרי שלוסברג - האיש מוילנא", {'entities': [(0, 8, 'SINGER'), (10, 23, 'SINGER')]}),
    ("יהלי קדמן, גבריאל טובול - כי הנה כחומר", {'entities': [(0, 9, 'SINGER'), (11, 23, 'SINGER')]}),
    ("אמנון קרן & מרדכי פארי - תשרי מיקס", {'entities': [(0, 9, 'SINGER'), (12, 22, 'SINGER')]}),
    ("האחים שמואלביץ' - מצוה טאנץ מושלם", {'entities': [(0, 15, 'SINGER')]}),
    ("להקת אש ומים & אבישי זקצר - בריה אחת", {'entities': [(0, 12, 'SINGER'), (15, 25, 'SINGER')]}),
    ("אריאל טבול & אילן אלון - שופט כל הארץ", {'entities': [(0, 10, 'SINGER'), (13, 22, 'SINGER')]}),
    ("אלעד ויסמן - דאס לעצטע דוכנ'ען", {'entities': [(0, 10, 'SINGER')]}),
    ("Arik Einstein אבנר אבו לפעמים(MP3_320K)", {'entities': [(14, 22, 'SINGER')]}),
    ("מאיר פיינגולד - עוד דקה את נעלמת(MP3_160K)", {'entities': [(0, 13, 'SINGER')]}),
    ("יעקב אמסל ויקיר אדרי - עוד דקה את נעלמת(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 20, 'SINGER')]}),
    ("אליה מזור - מיכל(MP3_320K)", {'entities': [(0, 9, 'SINGER')]}),
    ("איתי גלינסקי - הגולה - Dudu Tassa - Hagole(MP3_320K)", {'entities': [(0, 12, 'SINGER')]}),
    ("עמית מס - הלא נודע(MP3_320K)", {'entities': [(0, 7, 'SINGER')]}),
    ("אייל אניספלד ורונן שגב - זאת שמעל למצופה(MP3_320K)", {'entities': [(0, 12, 'SINGER'), (14, 22, 'SINGER')]}),
    ("יהודה ברקו - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("הכוכב הבא 2022 -- אליאב אבינועם - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(18, 31, 'SINGER')]}),
    ("גיל רבין - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 8, 'SINGER')]}),
    ("יאיר קשמה ויעקב סיבוני - סהרה (חי בלייב פארק)(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("חיים זליקוביץ - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 13, 'SINGER')]}),
    ("מרדכי סט עם בנימין אורלן - סתלבט בקיבוץ(MP3_320K)", {'entities': [(0, 8, 'SINGER'), (12, 24, 'SINGER')]}),
    ("דוד אליאב מעלה מעלה Svika Pick(MP3_320K)", {'entities': [(0, 9, 'SINGER')]}),
    ("עומר שגב לא_נרגע_Avihai_Hollender_Lo_Nirga", {'entities': [(0, 8, 'SINGER')]}),
    ("אריאל קורלנד, ניסים ברץ ועידו פולק - רואים רחוק רואים שקוף", {'entities': [(0, 12, 'SINGER'), (14, 23, 'SINGER'), (25, 34, 'SINGER')]}),
    ("שמשון קדר...לכה דודי", {'entities': [(0, 9, 'SINGER')]}),
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