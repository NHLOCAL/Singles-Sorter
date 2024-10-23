import os
import librosa

# הגדרת קטגוריות קצב וטווחי BPM
tempo_categories = {
    'רגוע': (0, 60),
    'מתון': (61, 100),
    'מהיר': (101, 140),
    'אנרגטי': (141, 180),
    'סוער': (181, 300)  # בהנחה ש-300 הוא ה-BPM המקסימלי
}

# ספרייה המכילה את קבצי השירים
directory = input("add path for dir\n>>>")

# יצירת מילון לאחסון השירים המסווגים
categorized_songs = {category: [] for category in tempo_categories}

# עיבוד כל קובץ שיר בספרייה
for filename in os.listdir(directory):
    if filename.endswith('.mp3') or filename.endswith('.wav'):
        file_path = os.path.join(directory, filename)
        # טעינת קובץ האודיו
        y, sr = librosa.load(file_path)
        # הערכת הקצב (BPM)
        tempo, _ = librosa.beat.beat_track(y=y, sr=sr)
        bpm = int(tempo)
        # סיווג השיר לפי הקצב
        for category, (min_bpm, max_bpm) in tempo_categories.items():
            if min_bpm <= bpm <= max_bpm:
                categorized_songs[category].append(filename)
                break

# הדפסת השירים המסווגים
for category, song_list in categorized_songs.items():
    print(f"שירים בקטגוריית {category}:")
    for title in song_list:
        print(f" - {title}")
    print()
