import random

# רשימה מורחבת של שמות זמרים פוטנציאליים
singers = [
    "אורי גלר", "דנה אינטרנשיונל", "עידן רייכל", "מיכל רגב",
    "קובי אפללו", "נועה קירל", "מרגלית צנעני", "אריק איינשטיין",
    "אייל גולן", "סטטיק ובן אל תבורי", "דודו טסה", "גל גדות",
    "נועה מורקם", "אתי אנקרי", "נועה בורווזילי", "מירי מסיקה",
    "שלמה ארצי", "שירי מימון", "הדג נחש", "נינט טייב",
    "ריטה", "כינור אורן", "איילת שמוליק", "טלי פורטמן",
    "יובל דיין", "דוד לוי", "אפרת גבאי", "תומר בר יחיאל",
    "לירז קורן", "עומר אדם", "השרון צוקר", "עופרה חזה",
    "גליה בן-דרור", "קובי פרץ", "שקד", "הדס שפירא",
    "אהוד בנאי", "שלום חנוך", "דן אנרג'י", "מיכל קליאב",
    "שלי אשכול", "מרקוס גרינברג", "ריטה דל ריו", "שירי מימון"
]

# רשימה מורחבת של חלקי שירים פוטנציאליים
song_phrases = [
    "אהבה בלי גבולות", "שירי הלב", "רגע של שלום", "חלום בהקיץ",
    "בלב הרחב", "לילה של כוכבים", "שוב אני חוזרת", "הדרך הביתה",
    "צעד אחר צעד", "שמיים פתוחים", "לב אחד", "כל יום מחדש",
    "נשימה עמוקה", "קול הלב", "שקט פנימי", "שיר של תקווה",
    "ליבי פועם", "רוחות של שינוי", "מנגינה של חיים", "פזמון האהבה",
    "שוב ניפגש", "מסע בזמן", "אהבה במלוא הדרה", "רוח סערה",
    "שביל האור", "לב סדוק", "צלילי הלב", "תמונות מחיי",
    "שירי הלילה", "פלא של רגשות", "נשיקות בחושך", "שירה תחת כיפת השמיים",
    "הזמן עובר", "רגעי השיא", "קצה התהילה", "אהבה מתוקה",
    "לילות של געגועים", "מסע הלב", "שירים בלי סוף", "חלון לעולם",
    "ציפורים בשמיים", "חלום מתוק", "שיר התודה", "נתיב האהבה",
    "תווים של אהבה", "ריקוד הלב", "אור בחושך", "שירה בחצות",
    "שיר הפרידה", "שמחת החיים", "לב פתוח", "שירי התקווה",
    "רגע של אמת", "קול היקום", "מנגינה נשכחת", "תזמורת הלב",
    "רוחניות בשיר", "שירה אינסופית", "הפסקת זמן", "נשמות מתנגנות",
    "שמיים כחולים", "לב נרגש", "שיר הנשמה", "המסע המוזיקלי",
    "פזמון מתוק", "מנגינות של אהבה", "שיר של אור", "קצב הלב",
    "חלום מוזיקלי", "שירת החופש", "לב בריא", "שמיים בהירים",
    "מנגינות השמחה", "שירה בשקט", "אהבה עד אין קץ", "ריקוד של שמחה",
    "קסם הרגע", "צלילי געגוע", "מסע אל העבר", "חיבוק של מילים",
    "דמעות של אושר", "מנגינת הגשם", "רסיסי אור", "מחול הרוח",
    "צבעים של רגש", "לחישות האופק", "שירת הים", "צעדים בחול",
    "חיוך של שחר", "דרך ארוכה הביתה", "פריחה באביב", "קרן שמש אחרונה",
    "ניגון השקט", "רעם בקיץ", "עלי שלכת", "אבק כוכבים",
    "נוצות ברוח", "להבה כחולה", "צליל השקיעה",
    "ריקוד הפרפרים", "שביל החלומות", "קצב הגלים", "ירח מלא",
    "שירת הציפורים", "עננים נודדים", "אש ומים", "צל ואור",
    "מנגינת היער", "קול הדממה", "פעמוני רוח", "שלג ראשון",
    "ניחוח הדרים", "קשת בענן", "אבני חן", "מפל הזהב",
    "נתיב החלב", "רוח מדברית", "גשר מעל התהום", "מגדלור בערפל",
    "צלצול פעמונים", "שדה פרגים", "מעיין נסתר", "שער הזמן",
    "מרבד קסמים", "צוק איתן", "נוף ילדות", "מחול הלהבות",
    "קצה העולם", "נחשול געגועים", "פסגת ההר", "שביל הכסף",
    "מנגינת הנחל", "סימפוניית האור", "ערפילי בוקר", "קרחון נמס",
    "שירת המדבר", "גן עדן אבוד", "מפתח הזהב", "חומות של תקווה",
    "נשימת הסתיו", "קרני ירח", "צליל השופר", "מחול הרוח",
    "שער הניצחון", "גלי אושר", "מגדל השן", "צופן החיים",
    "נתיב המשי", "קולות האוקיינוס", "שפת הפרחים", "מסתרי היקום",
    "רחש העלים", "מנגינת הקרח", "צבעי הקשת", "שירת האדמה",
    "מעגל הזמן", "ניצוץ של קסם", "מסע הנשמה", "צלילי המרחב",
    "שפת הכוכבים", "מחול הצללים", "קצב החיים", "נגיעה של אור",
    "שירת הרקיע", "מסתרי הלב", "צעדי הזמן", "קול ההד",
    "נוף פראי", "מנגינת הרוח", "צבעי השקיעה", "שער הדמיון",
    "רשרוש הגלים", "מסע אל הבלתי נודע", "צליל הדממה", "קסם הטבע",
    "שירת הנצח", "מחול העונות", "צלילי היקום", "נתיב האור",
    "קולות העיר", "מסתרי העבר", "צבעי הסתיו", "שער ההזדמנויות",
    "רחש המים", "מנגינת החלל", "צעדים בערפל", "קצב הלילה"
]

# רשימה מורחבת של תיאורי ביצועים או נוספות
performance_descriptors = [
    "בקולו של", "בביצוע של", "עם", "&", "רמיקס של",
    "בקולו המיוחד של", "בהשתתפות", "בגירסה", "באולפן של",
    "בליווי של", "בסטייל", "עם נגיעות של", "באירוח של",
    "בפורמט של", "בגרסה מיוחדת של", "עם שיתוף פעולה של",
    "באווירה של", "במנגינה של", "במילים של", "ביצוע מיוחד של"
]

# רשימה מורחבת של סוגי ביצועים או קישורים
performance_types = [
    "ווקאלית", "אינסטרומנטלית", "קאבר", "דיג'יי", "מזרחי", "דתי",
    "רוק", "פופ", "ג'אז", "קלאסי", "היפ הופ", "טכנו", "בלוז",
    "בלוזי", "אלקטרונית", "אקוסטית", "סול", "פאנק", "דאנס",
    "מיינסטרים", "אורקטרלית", "בלוז רוק", "אנדרוגינית", "פסיכדלית",
    "טאנק", "רפ", "טרנס", "דאבסטפ", "אימפרוביזציה"
]

# רשימה מורחבת של תאריכים או שנים
years = [
    "2021", "2022", "2023", "2024", "2025", "2026", "2027", "2028",
    "2029", "2030", "2019", "2018", "2017", "2016", "2015", "2014",
    "2013", "2012", "2011", "2010", "2009", "2008", "2007", "2006",
    "2005", "2004", "2003", "2002", "2001", "2000"
]

# רשימה מורחבת של תבניות שונות ליצירת שמות שירים
templates = [
    "{singer} - {song_phrase}",
    "{singer}, {song_phrase} {performance_type}",
    "{singer} {performance_descriptor} {another_singer} - {song_phrase}",
    "{song_phrase} - {performance_descriptor} {singer}",
    "{singer} {performance_type} {performance_descriptor} {song_phrase}",
    "{singer} & {another_singer} - {song_phrase}",
    "{song_phrase} {performance_descriptor} {singer} & {another_singer}",
    "{performance_type} {singer} - {song_phrase} {year}",
    "{singer} {performance_type} - {song_phrase} {performance_descriptor} {another_singer}",
    "{singer} {performance_descriptor} {singer} - {song_phrase}",
    "{singer} {song_phrase}, {performance_type}",
    "{song_phrase} {singer}",
    "{singer} - {song_phrase} {performance_type} {year}",
    "{singer} {performance_type} {song_phrase}",
    "{performance_descriptor} {singer} - {song_phrase}",
    "{singer} ft. {another_singer} - {song_phrase}",
    "{singer} {song_phrase} ({performance_type} version)",
    "{song_phrase} by {singer} {performance_type}",
    "{singer} presents {song_phrase}",
    "{singer} - {song_phrase} ({year})",
    "{song_phrase} / {singer}",
    "האלבום החדש של {singer}: {song_phrase}",
    "{singer} {performance_type} {song_phrase} feat. {another_singer}",
    "סינגל חדש: {singer} - {song_phrase}",
    "{singer} & {another_singer}, {song_phrase}",
    "{song_phrase} - {singer} {performance_type}",
    "{singer} - {song_phrase} remastered",
    "פרשנות חדשה: {singer} - {song_phrase}",
    "{singer} - {song_phrase} [Live]",
    "{singer} {performance_descriptor} {song_phrase}",
    "{singer} {song_phrase} - {performance_type}",
    "בלייב: {singer} - {song_phrase}",
    "{singer} {performance_type} {song_phrase} {year}",
    "{song_phrase} {year} {singer}",
    "{singer} {song_phrase} {performance_descriptor}",
    "{singer} - {song_phrase} {performance_type} feat. {another_singer}",
    "{song_phrase} {performance_type} by {singer}",
    "{singer} {song_phrase} - {performance_type}",
    "{singer} {song_phrase} in {performance_type}",
    "{song_phrase} - {singer} {year}",
    "{singer} {song_phrase} (feat. {another_singer})",
    "הגרסה החדשה של {singer} - {song_phrase}",
    "{singer} - {song_phrase} (Acoustic)",
    "{singer} {performance_type} של {song_phrase}",
    "{singer} {song_phrase} (Remix)",
    "{singer} {song_phrase} {performance_type}",
    "החזרה של {singer} עם {song_phrase}",
    "{singer} - {song_phrase} [Remix]",
    "{song_phrase} {performance_type} עם {singer}",
    "{singer} {performance_type} {song_phrase} {performance_descriptor}",
    "{singer} {song_phrase} {year}",
    "{singer} - {song_phrase} (Live)",
    "{singer} {song_phrase} {performance_type} {year}",
    "{singer} - {song_phrase} feat. {another_singer}",
    "{singer} {performance_descriptor} - {song_phrase}",
    "{singer} {performance_type} - {song_phrase} feat. {another_singer}",
    "{song_phrase} - {singer} {year}",
    "{singer} {song_phrase} (feat. {another_singer}) {performance_type}",
    "{singer} {song_phrase} {performance_type} feat. {another_singer}",
    "{singer} {song_phrase} {performance_descriptor} feat. {another_singer}",
    "{singer} {song_phrase} (feat. {another_singer})",
    "{song_phrase} by {singer} {performance_type}",
    "שיר חדש: {singer} - {song_phrase}",
    "{singer} {song_phrase} {performance_type} version",
    "{singer} - {song_phrase} (feat. {another_singer})",
    "{singer} {song_phrase} - {another_singer} {performance_type}"
]

def generate_song_title():
    template = random.choice(templates)
    singer = random.choice(singers)
    another_singer = random.choice([s for s in singers if s != singer]) if "{another_singer}" in template else ""
    song_phrase = random.choice(song_phrases)
    performance_descriptor = random.choice(performance_descriptors) if "{performance_descriptor}" in template else ""
    performance_type = random.choice(performance_types) if "{performance_type}" in template else ""
    year = random.choice(years) if "{year}" in template else ""
    
    # Handle optional parts
    try:
        title = template.format(
            singer=singer,
            another_singer=another_singer,
            song_phrase=song_phrase,
            performance_descriptor=performance_descriptor,
            performance_type=performance_type,
            year=year
        )
    except KeyError as e:
        # אם חסר מפתח כלשהו בתבנית, נחליף אותו במחרוזת ריקה
        key = e.args[0]
        title = template.replace(f"{{{key}}}", "")
        title = title.format(
            singer=singer,
            another_singer=another_singer,
            song_phrase=song_phrase,
            performance_descriptor=performance_descriptor,
            performance_type=performance_type,
            year=year
        )
    
    return title.strip()

def generate_song_titles(n):
    titles = set()
    attempts = 0
    max_attempts = n * 10  # הגבלת מספר הניסיונות למניעת לולאה אינסופית
    
    while len(titles) < n and attempts < max_attempts:
        title = generate_song_title()
        titles.add(title)
        attempts += 1
    
    if len(titles) < n:
        print(f"הושגו רק {len(titles)} כותרות מתוך {n} המבוקשות.")
    
    return list(titles)

# פונקציה להדפסת דוגמאות
def print_sample_titles(titles, sample_size=20):
    print(f"הצגת {sample_size} דוגמאות מתוך {len(titles)}:")
    for title in random.sample(titles, min(sample_size, len(titles))):
        print(title)

# יצירת מאות דוגמאות של שמות שירים
if __name__ == "__main__":
    number_of_samples = 10000  # ניתן לשנות את המספר לפי הצורך
    song_titles = generate_song_titles(number_of_samples)
    print_sample_titles(song_titles)
    
    # שמירת הדוגמאות לקובץ (לא חובה)
    with open("hebrew_song_titles.txt", "w", encoding="utf-8") as f:
        for title in song_titles:
            f.write(title + "\n")
