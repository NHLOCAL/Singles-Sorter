import os

def get_album_folders(root_folder):
  """
  מחזירה רשימה של שמות תיקיות אלבומים, למעט תיקיות "סינגלים".

  Args:
    root_folder: נתיב לתיקיה הראשית.

  Returns:
    רשימה של שמות תיקיות אלבומים.
  """
  album_folders = []
  for singer_folder in os.listdir(root_folder):
    singer_path = os.path.join(root_folder, singer_folder)
    if os.path.isdir(singer_path):
      for album_folder in os.listdir(singer_path):
        album_path = os.path.join(singer_path, album_folder)
        if os.path.isdir(album_path) and album_folder != "סינגלים":  # בדיקה שהפריט הוא תיקיה ושאינו "סינגלים"
          album_folders.append(album_folder)
  return album_folders

# נתיב לתיקיה הראשית
root_folder = r"D:\שמע\כל המוזיקה"

# קריאה לפונקציה וקבלת רשימת האלבומים
album_list = get_album_folders(root_folder)

# יצירת קובץ טקסט וכתיבת הרשימה אליו
with open("album_list.txt", "a", encoding="utf-8") as f:
  for album in album_list:
    f.write(album + "\n")

print(f"רשימת האלבומים נשמרה בקובץ album_list.txt")