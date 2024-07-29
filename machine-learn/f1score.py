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
    ("'מרדכי סט' והחברים בקומזיץ 'הבדלה' מושקע", {'entities': [(1, 9, 'SINGER')]}),
    ("רותם נח - אנחנו טובים", {'entities': [(0, 7, 'SINGER')]}),
    ("יהודה וויספיש וטל פוגל מגישים קאבר ''משיח''", {'entities': [(0, 13, 'SINGER'), (15, 22, 'SINGER')]}),
    ("גיל רבין משחרר שיר ''אלוקיי'' מאלבומו החדש ''אז ישיר''", {'entities': [(0, 8, 'SINGER')]}),
    ("רועי גרבר מגיש סינגל חדש בשם ''נופל שדוד''", {'entities': [(0, 9, 'SINGER')]}),
    ("בנימין קניאל במחרוזת משובחת בשם ''השפעות מוצאי שבת''", {'entities': [(0, 12, 'SINGER')]}),
    ("אריה גיגי ויעקב סיבוני במחרוזת קסומה עם מיטב שירי ר''ח", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("אביב רואס בסינגל עוצמתי ומחבר ''אל תוותר''", {'entities': [(0, 9, 'SINGER')]}),
    ("אוהד מוגילבסקי מפתיע ומשחרר סינגל חדש בסטייל השמור רק לו ''מודים''", {'entities': [(0, 14, 'SINGER')]}),
    ("עומר יוסף ורוי ווסה שרים שוובר ועל חסדך", {'entities': [(0, 9, 'SINGER'), (11, 19, 'SINGER')]}),
    ("אלון דגני - מחרוזת לב & נשמה", {'entities': [(0, 9, 'SINGER')]}),
    ("אופק ריינר בסינגל בכורה חדש ''אני פה להישאר''", {'entities': [(0, 10, 'SINGER')]}),
    ("ליפא’ס ערשטע טאנץ - נהוראי שמואלי", {'entities': [(20, 33, 'SINGER')]}),
    ("איתי חיים לא נח לרגע ומגיש מחרוזת חופה מרגשת עם מיטב הלהיטים", {'entities': [(0, 9, 'SINGER')]}),
    ("ניסים ברץ, רונן דולינסקי וגדולי הזמר בסדרת חופה מרהיבה ''ברוך הבא''", {'entities': [(0, 9, 'SINGER'), (11, 24, 'SINGER')]}),
    ("אהרון קפרא ותזמורתו של שי מוגס במחרוזת ''שירי שלמה''", {'entities': [(0, 10, 'SINGER'), (23, 30, 'SINGER')]}),
    ("מרק ויזגן מפתיע בסינגל חדש ''לבוש שחורים''", {'entities': [(0, 9, 'SINGER')]}),
    ("רועי עיני והמכונות - מרבים בשמחה", {'entities': [(0, 9, 'SINGER')]}),
    ("רונן שגב בסינגל חדש ''כבד את אביך''", {'entities': [(0, 8, 'SINGER')]}),
    ("אלירן סבן משיק סינגל חדש מעומק הלב ''נחלי דמעות''", {'entities': [(0, 9, 'SINGER')]}),
    ("אליה מזור ואביו בנימין חי בסינגל חדש ''געבליבן מיט דיר''", {'entities': [(0, 9, 'SINGER'), (16, 25, 'SINGER')]}),
    ("עידן בוחבוט במחרוזת להיטי ניר שטרית", {'entities': [(0, 11, 'SINGER'), (26, 35, 'SINGER')]}),
    ("אבנר גולה והחברים בשירה שכולו ערגה וכיסופין ''שלשידעס''", {'entities': [(0, 9, 'SINGER')]}),
    ("Joey-Newcomb-Kaeileh-יחיאל מינץ-כאלה", {'entities': [(21, 31, 'SINGER')]}),
    ("גל רוט סהר שיבר אלוקי נשמה MP3", {'entities': [(0, 6, 'SINGER'), (7, 15, 'SINGER')]}),
    ("אשריכם תלמידי חכמים - אופק חן - ווקאלי", {'entities': [(22, 29, 'SINGER')]}),
    ("בניה בורובסקי וירחמיאל דאנה - מסלסלים", {'entities': [(0, 13, 'SINGER'), (15, 27, 'SINGER')]}),
    ("אבנר אבו & מיכל טאדסה - מחרוזת ריקודים", {'entities': [(0, 8, 'SINGER'), (11, 21, 'SINGER')]}),
    ("מלך מלכי המלכים עמית מס", {'entities': [(16, 23, 'SINGER')]}),
    ("דודי יוזוק .שדרן שיר", {'entities': [(0, 10, 'SINGER')]}),
    ("עידו דניאלי & ליר אזולוס - כי אנו עמך", {'entities': [(0, 11, 'SINGER'), (14, 24, 'SINGER')]}),
    ("ניגוני יהודה (יהונתן פטרזיל) triste esta el rey david", {'entities': [(14, 27, 'SINGER')]}),
    ("נתנאל & אריאל ביסטרוב והיא שעמדה", {'entities': [(8, 21, 'SINGER')]}),
    ("סט 2019  דיגי אסף אבוטבול (320  kbps) (vipman.xyz)", {'entities': [(14, 25, 'SINGER')]}),
    ("ראם צורף סינגל", {'entities': [(0, 8, 'SINGER')]}),
    ("תום ביטון עם מדרשת תפארת בחורים - אתיתי לחננך", {'entities': [(0, 9, 'SINGER')]}),
    ("יעקב אמסל ויהונתן פליי חזו בני", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("אורי צמח & יועד סויסה - הושיע אותנו", {'entities': [(0, 8, 'SINGER'), (11, 21, 'SINGER')]}),
    ("אברהם קופולוביץ ואיתי גלינסקי מקפיצים", {'entities': [(0, 15, 'SINGER'), (17, 29, 'SINGER')]}),
    ("חיים אלמלך - עננו - anenu - sruli ginsberg", {'entities': [(0, 10, 'SINGER')]}),
    ("יעקב מגידש - והלוואי - Tamir Mizrachi - Vehalevai", {'entities': [(0, 10, 'SINGER')]}),
    ("ברכת המזון המנגינה שהלחין פאר נח", {'entities': [(26, 32, 'SINGER')]}),
    ("האחים שמואלביץ' - במסע החיים", {'entities': [(0, 15, 'SINGER')]}),
    ("ידידיה אבוחצירא שיפקדוני", {'entities': [(0, 15, 'SINGER')]}),
    ("ישראל דרויאן ואריאל טבול ''ידע כל פעול''", {'entities': [(0, 12, 'SINGER'), (14, 24, 'SINGER')]}),
    ("עידו לובינסקי ויהלי קדמן מרגשים במחרוזת ''ימים נוראים''", {'entities': [(0, 13, 'SINGER'), (15, 24, 'SINGER')]}),
    ("נתן שפירו, עידו אלמקיס - אני מאמין", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("נחמן וידידיה טימן - ניגון וזמרה", {'entities': [(6, 17, 'SINGER')]}),
    ("להקת אש ומים בסינגל חדש לימים הנוראים ''אבינו מלכנו''", {'entities': [(0, 12, 'SINGER')]}),
    ("בני ג'וסף, מקהלת בית רבן - האיש מוילנא", {'entities': [(0, 9, 'SINGER'), (11, 24, 'SINGER')]}),
    ("יהלי מעלם, עידו מזוז - כי הנה כחומר", {'entities': [(0, 9, 'SINGER'), (11, 20, 'SINGER')]}),
    ("יובל חבקוק & גבריאל טובול - תשרי מיקס", {'entities': [(0, 10, 'SINGER'), (13, 25, 'SINGER')]}),
    ("דן צפדיה - מצוה טאנץ מושלם", {'entities': [(0, 8, 'SINGER')]}),
    ("אייל בסטקר & יאיר קשמה - בריה אחת", {'entities': [(0, 10, 'SINGER'), (13, 22, 'SINGER')]}),
    ("גיל אביהו & יונתן מויאל - שופט כל הארץ", {'entities': [(0, 9, 'SINGER'), (12, 23, 'SINGER')]}),
    ("יצחק שיינרמן - דאס לעצטע דוכנ'ען", {'entities': [(0, 12, 'SINGER')]}),
    ("Arik Einstein אלחנן מאיר לפעמים(MP3_320K)", {'entities': [(14, 24, 'SINGER')]}),
    ("זוהר קרופרו - עוד דקה את נעלמת(MP3_160K)", {'entities': [(0, 11, 'SINGER')]}),
    ("אברהם טפירו ואליהו מידן - עוד דקה את נעלמת(MP3_320K)", {'entities': [(0, 11, 'SINGER'), (13, 23, 'SINGER')]}),
    ("אברהם קפלון - מיכל(MP3_320K)", {'entities': [(0, 11, 'SINGER')]}),
    ("יוסף צברי - הגולה - Dudu Tassa - Hagole(MP3_320K)", {'entities': [(0, 9, 'SINGER')]}),
    ("מתן שפר - הלא נודע(MP3_320K)", {'entities': [(0, 7, 'SINGER')]}),
    ("יצחק צפרי ואהרון חג'בי - זאת שמעל למצופה(MP3_320K)", {'entities': [(0, 9, 'SINGER'), (11, 22, 'SINGER')]}),
    ("שלום גרמאי - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(0, 10, 'SINGER')]}),
    ("הכוכב הבא 2022 -- מאיר פיינגולד - זאת שמעל לכל המצופה(MP3_320K)", {'entities': [(18, 31, 'SINGER')]}),
    ("יהונתן גובי - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 11, 'SINGER')]}),
    ("אנטוני סלומון ויואב בוני - סהרה (חי בלייב פארק)(MP3_320K)", {'entities': [(0, 13, 'SINGER'), (15, 24, 'SINGER')]}),
    ("אימרי שלוסברג - חופים הם לפעמים(MP3_320K)", {'entities': [(0, 13, 'SINGER')]}),
    ("אברהם מליחי עם אדיר לוי - סתלבט בקיבוץ(MP3_320K)", {'entities': [(0, 11, 'SINGER'), (15, 23, 'SINGER')]}),
    ("מקהלת שישו ושמחו מעלה מעלה Svika Pick(MP3_320K)", {'entities': [(0, 16, 'SINGER')]}),
    ("איתמר סיטבון לא_נרגע_Avihai_Hollender_Lo_Nirga", {'entities': [(0, 12, 'SINGER')]}),
    ("אוהד רחבי, עומר שגב ומתן שמו - רואים רחוק רואים שקוף", {'entities': [(0, 9, 'SINGER'), (11, 19, 'SINGER'), (21, 28, 'SINGER')]}),
    ("יואב סילם...לכה דודי", {'entities': [(0, 9, 'SINGER')]}),
    (" - שי מסגנאו", {'entities': [(3, 12, 'SINGER')]}),
    (" - מאיר פרץ", {'entities': [(3, 11, 'SINGER')]}),
    (" - יוגב אמסלם", {'entities': [(3, 13, 'SINGER')]}),
    (" - אילן אלון", {'entities': [(3, 12, 'SINGER')]}),
    (" - לביא חדד", {'entities': [(3, 11, 'SINGER')]}),
    (" - אורן נעים", {'entities': [(3, 12, 'SINGER')]}),
    (" - אבישי חדד", {'entities': [(3, 12, 'SINGER')]}),
    (" - גיא סוזנה", {'entities': [(3, 12, 'SINGER')]}),
    (" - עירד נגר", {'entities': [(3, 11, 'SINGER')]}),
    (" - אוריה סממה", {'entities': [(3, 13, 'SINGER')]}),
    (" - אמיתי אלבז", {'entities': [(3, 13, 'SINGER')]}),
    (" - דניאל שנטל", {'entities': [(3, 13, 'SINGER')]}),
    (" - גל רובינסקי", {'entities': [(3, 14, 'SINGER')]}),
    (" - עידן לוטן", {'entities': [(3, 12, 'SINGER')]}),
    (" - שחר כהן", {'entities': [(3, 10, 'SINGER')]}),
    (" - חיים זליקוביץ", {'entities': [(3, 16, 'SINGER')]}),
    (" - שגיא פוקס", {'entities': [(3, 12, 'SINGER')]}),
    (" - מרדכי יצחקי", {'entities': [(3, 14, 'SINGER')]}),
    (" - איתי לוי", {'entities': [(3, 11, 'SINGER')]}),
    (" - איתי קרמר", {'entities': [(3, 12, 'SINGER')]}),
    ("12 - אמיר בנימין", {'entities': [(5, 16, 'SINGER')]}),
    (" - שגיא אזולאי", {'entities': [(3, 14, 'SINGER')]}),
    (" - יובל סיביליה", {'entities': [(3, 15, 'SINGER')]}),
    (" - יעקב בטיטו", {'entities': [(3, 13, 'SINGER')]}),
    (" - גיל בן-ארי", {'entities': [(3, 13, 'SINGER')]}),
    (" - אליאב אבינועם", {'entities': [(3, 16, 'SINGER')]}),
    (" - איתמר כהן", {'entities': [(3, 12, 'SINGER')]}),
    (" - שמעון פיק", {'entities': [(3, 12, 'SINGER')]}),
    (" - אייל אניספלד", {'entities': [(3, 15, 'SINGER')]}),
    (" - יוסף ווסקובויניקוב", {'entities': [(3, 21, 'SINGER')]}),
    ("תומר סבח טאנץ..", {'entities': [(0, 8, 'SINGER')]}),
    (" - שאול פנחסוב", {'entities': [(3, 14, 'SINGER')]}),
    (" - עידו פולק", {'entities': [(3, 12, 'SINGER')]}),
    (" - בן זיונס", {'entities': [(3, 11, 'SINGER')]}),
    (" - איתן אילני", {'entities': [(3, 13, 'SINGER')]}),
    (" - ניצן קופף", {'entities': [(3, 12, 'SINGER')]}),
    (" - פנחס בוקובזה", {'entities': [(3, 15, 'SINGER')]}),
    (" - שגיא גרמון", {'entities': [(3, 13, 'SINGER')]}),
    (" - אליעזר שמיר", {'entities': [(3, 14, 'SINGER')]}),
    (" - יהודה ברקו", {'entities': [(3, 13, 'SINGER')]}),
    (" - מור שניידר", {'entities': [(3, 13, 'SINGER')]}),
    (" - אריאל קורלנד", {'entities': [(3, 15, 'SINGER')]}),
    (" - משה אזולאי", {'entities': [(3, 13, 'SINGER')]}),
    (" - שמואל קיפגן", {'entities': [(3, 14, 'SINGER')]}),
    (" - בר פריאנטה", {'entities': [(3, 13, 'SINGER')]}),
    (" - אורן מיזל", {'entities': [(3, 12, 'SINGER')]}),
    (" - חיים כהן", {'entities': [(3, 11, 'SINGER')]}),
    (" - שמשון קדר", {'entities': [(3, 12, 'SINGER')]}),
    (" - נועם שמחון", {'entities': [(3, 13, 'SINGER')]}),
    (" - יונתן מוטיעי", {'entities': [(3, 15, 'SINGER')]}),
    ("12. אושר מנחם, שון שפיצר, א.ח", {'entities': [(4, 13, 'SINGER'), (15, 24, 'SINGER')]}),
    ("תמיר טל פורים - [WinDos", {'entities': [(0, 7, 'SINGER')]}),
    (" - זיו כהן", {'entities': [(3, 10, 'SINGER')]}),
    (" - יובל כהן", {'entities': [(3, 11, 'SINGER')]}),
    ("16 - אשת חיל - רני הלוי - [Windos", {'entities': [(15, 23, 'SINGER')]}),
    (" - אלירן ווקנין", {'entities': [(3, 15, 'SINGER')]}),
    ("שבת וואלץ -נדב רויטמן  2", {'entities': [(11, 21, 'SINGER')]}),
    (" - יעקב איתמר", {'entities': [(3, 13, 'SINGER')]}),
    (" - אסף תמיר", {'entities': [(3, 11, 'SINGER')]}),
    (" - גלעד פרטר", {'entities': [(3, 12, 'SINGER')]}),
    (" - מרדכי פארי", {'entities': [(3, 13, 'SINGER')]}),
    (" - דרור גולן", {'entities': [(3, 12, 'SINGER')]}),
    (" - נוי רזון", {'entities': [(3, 11, 'SINGER')]}),
    (" - בנימין אורלן", {'entities': [(3, 15, 'SINGER')]}),
    (" - יוסף פז", {'entities': [(3, 10, 'SINGER')]}),
    (" - זיו אלמקיאס", {'entities': [(3, 14, 'SINGER')]}),
    (" - אפיק סעד", {'entities': [(3, 11, 'SINGER')]}),
    (" - יהודה טורקניץ", {'entities': [(3, 16, 'SINGER')]}),
    (" - נתן ארזי", {'entities': [(3, 11, 'SINGER')]}),
    (" - זאב גפן", {'entities': [(3, 10, 'SINGER')]}),
    (" - חיים הרפז", {'entities': [(3, 12, 'SINGER')]}),
    (" - אופק מתן", {'entities': [(3, 11, 'SINGER')]}),
    (" - יצחק ביגל", {'entities': [(3, 12, 'SINGER')]}),
    (" - יואל הרטמן", {'entities': [(3, 13, 'SINGER')]}),
    (" - יובל שפיגלמן", {'entities': [(3, 15, 'SINGER')]}),
    (" - אלון שרביט", {'entities': [(3, 13, 'SINGER')]}),
    (" - מנחם אזרן", {'entities': [(3, 12, 'SINGER')]}),
    (" - לביא גוטפריד", {'entities': [(3, 15, 'SINGER')]}),
    (" - אמנון קרן", {'entities': [(3, 12, 'SINGER')]}),
    (" - אלעד ויסמן", {'entities': [(3, 13, 'SINGER')]}),
    (" - נפתלי גרפל", {'entities': [(3, 13, 'SINGER')]}),
    (" - אלי חופי", {'entities': [(3, 11, 'SINGER')]}),
    (" - ליעד בק", {'entities': [(3, 10, 'SINGER')]}),
    (" - עומר ברמי", {'entities': [(3, 12, 'SINGER')]}),
    (" - יהונתן כהן", {'entities': [(3, 13, 'SINGER')]}),
    (" - אילן עני", {'entities': [(3, 11, 'SINGER')]}),
    (" - יקיר אדרי", {'entities': [(3, 12, 'SINGER')]}),
    (" - נחמיה לרנר", {'entities': [(3, 13, 'SINGER')]}),
    (" - עזרא עוזרי", {'entities': [(3, 13, 'SINGER')]}),
    (" - בועז איילה", {'entities': [(3, 13, 'SINGER')]}),
    (" - דוד אליאב", {'entities': [(3, 12, 'SINGER')]}),
    (" - אבנר אפריאט", {'entities': [(3, 14, 'SINGER')]}),
    (" - מאיר אוזן", {'entities': [(3, 12, 'SINGER')]}),
    (" - אופיר שפיגל", {'entities': [(3, 14, 'SINGER')]}),
    (" - אביתר רטנר", {'entities': [(3, 13, 'SINGER')]}),
    (" - ראם צ'קול", {'entities': [(3, 12, 'SINGER')]}),
    (" - אבישי זקצר", {'entities': [(3, 13, 'SINGER')]}),
    (" - משה בוזגלו", {'entities': [(3, 13, 'SINGER')]}),
    (" - רוי ווסה", {'entities': [(3, 11, 'SINGER')]}),
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