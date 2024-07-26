import pandas as pd

# טעינת רשימת השירים
with open('list_all_songs_random.txt', 'r', encoding='utf-8') as file:
    songs = file.read().splitlines()

# טעינת רשימת הזמרים
singers_df = pd.read_csv('singers_list.csv', header=None)

# רשימת הזמרים מהעמודה הראשונה (A)
singers = singers_df[0].tolist()

# בדיקת זמרים שאינם מופיעים ברשימת השירים
missing_singers = [singer for singer in singers if not any(singer in song for song in songs)]

# הדפסת זמרים חסרים עם מספור
print("זמרים שאינם מופיעים ברשימת השירים:")
for idx, singer in enumerate(missing_singers, 1):
    print(singer)
    
print(f'\nסה"כ: - {len(missing_singers)} - זמרים חסרים')