# ניתוח תכונת מיון אלבומים: בעיות ופתרונות

## 1. זיהוי שגוי של תיקיית סינגלים כאלבום

### בעיה:
התוכנית עלולה לזהות בטעות תיקייה המכילה מספר סינגלים כאלבום.

### פתרונות אפשריים:
- [x] א) הגדרת סף מינימלי למספר הקבצים שייחשבו כאלבום (למשל, 4-5 רצועות).
- [x] ב) בדיקת עקביות במטא-נתונים של האלבום בין הקבצים.
- [x] ג) חיפוש אינדיקטורים טיפוסיים לאלבום כמו מספרי רצועות או דפוס שמות עקבי.

### יישום מומלץ:
```python
def is_album(folder_path):
    audio_files = [f for f in os.listdir(folder_path) if f.lower().endswith(('.mp3', '.wma', '.wav'))]
    if len(audio_files) < 4:
        return False
    
    album_names = []
    for file in audio_files:
        metadata = load_file(os.path.join(folder_path, file))
        album_name = metadata.get('album')
        if album_name:
            album_names.append(album_name)
    
    # אם יותר מ-70% מהקבצים יש להם את אותו שם אלבום, נחשיב זאת כאלבום
    if album_names and (album_names.count(max(set(album_names), key=album_names.count)) / len(album_names) > 0.7):
        return True
    return False
```

## 2. מספר אמנים באלבום אחד

### בעיה:
תיקיית אלבום עשויה להכיל רצועות ממספר אמנים (למשל, אוספים, שיתופי פעולה).

### פתרונות אפשריים:
- [ ] א) יצירת קטגוריה "אמנים שונים" עבור אלבומים כאלה.
- [x] ב) שימוש באמן הנפוץ ביותר כאמן הראשי של האלבום.
- [ ] ג) יצירת עותקים כפולים של האלבום בתיקייה של כל אמן.

### יישום מומלץ:
```python
def get_album_artist(folder_path):
    artists = []
    for file in os.listdir(folder_path):
        if file.lower().endswith(('.mp3', '.wma', '.wav')):
            metadata = load_file(os.path.join(folder_path, file))
            artist = metadata.get('artist')
            if artist:
                artists.append(artist)
    
    if not artists:
        return None
    
    most_common_artist = max(set(artists), key=artists.count)
    if artists.count(most_common_artist) / len(artists) > 0.7:
        return most_common_artist
    else:
        return "אמנים שונים"
```

## 3. הבחנה בין סינגלים לרצועות אלבום

### בעיה:
התוכנית עלולה להתקשות להבדיל בין תיקייה של סינגלים לבין רצועות אלבום אמיתיות.

### פתרונות אפשריים:
- [x] א) בדיקת עקביות במטא-נתונים של האלבום בין הקבצים.
- [x] ב) חיפוש מאפיינים ספציפיים לאלבום כמו מספור רצועות רציף.
- [ ] ג) התחשבות במבנה התיקיות ובמוסכמות השמות.

### יישום מומלץ:
```python
def is_album_folder(folder_path):
    audio_files = [f for f in os.listdir(folder_path) if f.lower().endswith(('.mp3', '.wma', '.wav'))]
    if len(audio_files) < 4:
        return False
    
    album_names = []
    track_numbers = set()
    for file in audio_files:
        metadata = load_file(os.path.join(folder_path, file))
        album_name = metadata.get('album')
        track_number = metadata.get('tracknumber')
        if album_name:
            album_names.append(album_name)
        if track_number:
            track_numbers.add(int(track_number))
    
    album_name_consistency = len(set(album_names)) == 1
    track_number_consistency = len(track_numbers) == len(audio_files)
    
    return album_name_consistency and track_number_consistency
```

## 4. טיפול בתתי-תיקיות בתוך תיקיות אלבום

### בעיה:
תיקיות אלבום עשויות להכיל תתי-תיקיות (למשל, עבור דיסקים שונים באלבום מרובה דיסקים).

### פתרונות אפשריים:
א) יישום סריקה רקורסיבית לקבצי אודיו בתוך תיקיות אלבום.
ב) שמירה על מבנה תתי-התיקיות בעת העברה/העתקה של אלבומים.
ג) שטיחת המבנה, מיזוג כל הרצועות לתיקייה אחת.

### יישום מומלץ:
```python
def process_album_folder(album_path, target_path):
    for root, dirs, files in os.walk(album_path):
        for file in files:
            if file.lower().endswith(('.mp3', '.wma', '.wav')):
                source_file = os.path.join(root, file)
                relative_path = os.path.relpath(root, album_path)
                target_file = os.path.join(target_path, relative_path, file)
                os.makedirs(os.path.dirname(target_file), exist_ok=True)
                shutil.copy2(source_file, target_file)
```

## 5. טיפול באלבומים לא שלמים או פגומים

### בעיה:
חלק מתיקיות האלבום עלולות להיות לא שלמות או להכיל קבצים פגומים.

### פתרונות אפשריים:
א) הגדרת סף למספר המינימלי של קבצים תקינים הנדרשים לעיבוד אלבום.
ב) רישום אזהרות עבור אלבומים שעשויים להיות לא שלמים.
ג) יישום תיקיית "הסגר" עבור אלבומים בעייתיים.

### יישום מומלץ:
```python
def process_album(album_path, target_path, min_valid_files=4):
    valid_files = [f for f in os.listdir(album_path) if f.lower().endswith(('.mp3', '.wma', '.wav')) and is_valid_audio_file(os.path.join(album_path, f))]
    
    if len(valid_files) < min_valid_files:
        logger.warning(f"אלבום שעשוי להיות לא שלם: {album_path}")
        quarantine_path = os.path.join(target_path, "הסגר", os.path.basename(album_path))
        shutil.move(album_path, quarantine_path)
        return
    
    # עיבוד האלבום באופן רגיל
    ...

def is_valid_audio_file(file_path):
    try:
        metadata = load_file(file_path)
        # ביצוע בדיקות בסיסיות על המטא-נתונים
        return bool(metadata.get('title') and metadata.get('artist'))
    except Exception:
        return False
```

## אפשרויות תצורה מומלצות

כדי לטפל בבעיות אלה ולספק גמישות, שקול להוסיף את אפשרויות התצורה הבאות לתוכנית שלך:

1. `--album-threshold`: מספר מינימלי של רצועות שייחשבו כתיקייה לאלבום (ברירת מחדל: 4)
2. `--album-consistency`: אחוז מינימלי של קבצים שחייבים להיות בעלי מטא-נתוני אלבום עקביים (ברירת מחדל: 70%)
3. `--handle-multi-artist`: כיצד לטפל באלבומים של מספר אמנים ("שונים", "ראשי", "כפול")
4. `--subfolder-handling`: כיצד לטפל בתתי-תיקיות באלבומים ("שמור", "שטח")
5. `--incomplete-album-action`: פעולה עבור אלבומים שעשויים להיות לא שלמים ("הסגר", "התעלם", "עבד")

אפשרויות אלה יאפשרו למשתמשים לכוונן את התנהגות מיון האלבומים בהתאם לצרכים הספציפיים שלהם ולמאפייני ספריית המוזיקה שלהם.

