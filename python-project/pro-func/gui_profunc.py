import tkinter as tk

from tkinter import filedialog

def get_directory():
  # קבלת נתיב תיקיה מהמשתמש
  directory = tk.filedialog.askdirectory()
  return directory

# יצירת חלון גרפי
window = tk.Tk()

# יצירת כפתור לקבלת נתיב תיקיה
button = tk.Button(window, text="בחר תיקיה", command=get_directory)
button.pack()

# רצת התוכנה
window.mainloop()

# יצירת מלבן להחזרת פלט
output = tk.Text(window)
output.pack()

# יצירת 3 מקומות לסימון וי
checkbox1 = tk.Checkbutton(window, text="אפשרות 1")
checkbox1.pack()
checkbox2 = tk.Checkbutton(window, text="אפשרות 2")
checkbox2.pack()
checkbox3 = tk.Checkbutton(window, text="אפשרות 3")
checkbox3.pack()

# הגדרת פונקציה לעדכון המלבן
def update_output():
  # קבלת ערכי המקומות לסימון וי
  checkbox1_value = checkbox1.get()
  checkbox2_value = checkbox2.get()
  checkbox3_value = checkbox3.get()

  # בדיקת הסימונים והחזרת הפלט למלבן
  output_text = "ערכי הסימונים:\n"
  if checkbox1_value:
    output_text += "אפשרות 1\n"
  if checkbox2_value:
    output_text += "אפשרות 2\n"
  if checkbox3_value:
    output_text += "אפשרות 3\n"
  output.delete(1.0, tk.END)  # ניקוי המלבן
  output.insert(tk.END, output_text)  # הכנסת הפלט למלבן

# יצירת כפתור לעדכון המלבן
update_button = tk.Button(window, text="עדכן", command=update_output)
update_button.pack()

# רצת התוכנה
window.mainloop()
