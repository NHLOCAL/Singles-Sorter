import os
import random

def get_song_list(root_folder):
  """
  מחזירה רשימה של שמות שירים מכל עץ התיקיות, ללא סיומות.

  Args:
    root_folder: נתיב לתיקיה הראשית.

  Returns:
    רשימה של שמות שירים.
  """
  song_list = []
  for _, _, filenames in os.walk(root_folder):
    for filename in filenames:
      if filename.endswith((".mp3", ".wav", ".flac")):  # הוסף כאן סיומות נוספות לפי הצורך
        song_name = os.path.splitext(filename)[0]  # הסרת סיומת
        song_list.append(song_name)
  return song_list

# נתיב לתיקיה הראשית
root_folder = r"D:\שמע\כל המוזיקה"

# קריאה לפונקציה וקבלת רשימת השירים
song_list = get_song_list(root_folder)

# ערבוב הרשימה בצורה רנדומלית
random.shuffle(song_list)

# יצירת קובץ טקסט וכתיבת הרשימה אליו
with open("song_list.txt", "w", encoding="utf-8") as f:
  for song in song_list:
    f.write(song + "\n")

print(f"רשימת השירים (בצורה רנדומלית) נשמרה בקובץ song_list.txt")