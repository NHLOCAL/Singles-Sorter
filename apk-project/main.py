import csv
import os
import shutil
# יבוא פונקציה להמרת ג'יבריש לעברית תקינה
from jibrish_to_hebrew import jibrish_to_hebrew
# יבוא פונקציה לזיהוי דמיון בין מחרוזות
from identify_similarities import similarity_sure

from kivy.app import App
from kivy.uix.boxlayout import BoxLayout
from kivy.uix.button import Button
from kivy.uix.label import Label
from kivy.uix.popup import Popup
from kivy.uix.filechooser import FileChooserListView, FileChooserIconView
from kivy.properties import StringProperty

class MusicOrganizerApp(App):
    source_path = StringProperty('')
    target_path = StringProperty('')

    def build(self):
        layout = BoxLayout(orientation='vertical')

        source_label = Label(text='תיקית מקור', size_hint_y=0.1)
        layout.add_widget(source_label)

        if self.is_android():  # Use FileChooserIconView for Android
            self.source_chooser = FileChooserIconView()
        else:
            self.source_chooser = FileChooserListView()

        self.source_chooser.bind(path=self.on_source_selection)
        layout.add_widget(self.source_chooser)

        target_label = Label(text='תיקית יעד', size_hint_y=0.1)
        layout.add_widget(target_label)

        if self.is_android():  # Use FileChooserIconView for Android
            self.target_chooser = FileChooserIconView()
        else:
            self.target_chooser = FileChooserListView()

        self.target_chooser.bind(path=self.on_target_selection)
        layout.add_widget(self.target_chooser)

        run_button = Button(text='מסדר הסינגלים', size_hint_y=0.1)
        run_button.bind(on_press=self.organize_music)
        layout.add_widget(run_button)

        self.result_label = Label(text='', size_hint_y=0.1)
        layout.add_widget(self.result_label)

        return layout

    def is_android(self):
        return 'ANDROID_ARGUMENT' in os.environ

    def on_source_selection(self, instance, path):
        self.source_path = path

    def on_target_selection(self, instance, path):
        self.target_path = path

    def organize_music(self, instance):
        if not self.source_path or not self.target_path:
            self.show_error_popup("Please select source and target directories.")
            return

        try:
            scan_dir(self.source_path, self.target_path)
            self.result_label.text = "מיון הקבצים הסתיים!"
        except Exception as e:
            self.show_error_popup("Error: {}".format(e))

    def show_error_popup(self, message):
        popup = Popup(title='Error', content=Label(text=message), size_hint=(None, None), size=(400, 200))
        popup.open()


def artist_from_song(my_file):
    """
הפונקציה בודקת את שם אמן הקובץ בשם הקובץ לפי מסד נתונים ומכניסה את שם האמן למשתנה
אם השם לא קיים היא סורקת את המטאדאטה של השיר ומכניסה את שם האמן למשתנה
    
תנאים:
    my_file (str) - שם הקובץ שנסרק
    
תוצאה:
    ערך המכיל את שם אמן הקובץ
    """
 
    # מעבר על רשימת הזמרים בדאטה וחיפוש שלהם בשם הקובץ
    
    # קבלת שם הקובץ ללא נתיב מלא
    split_file = os.path.split(my_file)[1]
    
    # הסרת תווים מטעים בשם הקובץ
    split_file = split_file.replace('_', ' ')
    split_file = split_file.replace('-', ' ')
    
    # יבוא רשימת זמרים מקובץ csv
    if not 'singer_list' in globals():
        csv_path = "singer-list.csv"
        global singer_list
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]
        
        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)
    
    # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בשם השיר
    for source_name, target_name in singer_list:
        if source_name in split_file:
            artist = target_name
            return artist

    # אם שם הקובץ לא נמצא יתבצע חיפוש במטאדאטה של הקובץ
    try:
        # בדיקה האם הקובץ הוא קובץ שמע
        if not my_file.lower().endswith((".mp3",".wma", ".wav")):
            return
        # טעינת מטאדאטה של השיר
        artist_file = load_file(my_file)
        # קבלת אמן מטאדאטה של השיר
        artist = artist_file['artist']
        artist = artist.value
        # הכנסת נתוני האמן למשתנה הגלובלי
        if artist:
            # המרת שם האמן אם הוא פגום
            if any(c in "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ" for c in artist):
                artist = jibrish_to_hebrew(artist)
            
            # מעבר על רשימת השמות ובדיקה אם אחד מהם קיים בתגית האמן
            for source_name, target_name in singer_list:
                if source_name in artist:
                    artist = target_name
                    return artist 
                
            # הפעלת פונקציה המבצעת בדיקות על שם האמן
            check_answer = check_artist(artist)
            if check_answer == False:
                return
                
            return artist
    except:
        return


# בדיקות שונות על שם האמן
def check_artist(artist):
    # הגדרת רשימת מחרוזות יוצאות דופן, עליהם המערכת מוגדרת לדלג
    unusual_list = ["סינגלים", "סינגל", "אבגדהוזחטיכלמנסעפצקרשתךםןץ", "אמן לא ידוע", "טוב", "לא ידוע", "תודה לך ה"]
    # החזרת שקר אם שם האמן קיים ברשימת יוצאי הדופן
    # או אם הוא דומה לפריט כלשהו ברשימת יוצאי הדופן       
    unusual_str = similarity_sure(artist, unusual_list, True)        
    if artist in unusual_list or unusual_str[0]:
        return False
       
    # בדיקה אם המחרוזת אינה ארוכה מידי
    if len(artist.split()) >= 4 or len(artist.split()) <= 0:
        return False
    
    # בדיקה אם המחרוזת מכילה תוים תקינים בלבד
    if all(c in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ'׳ " for c in artist):
        return True
    else:
        return False


# מעבר על עץ התיקיות שהוגדר
def scan_dir(dir_path, target_dir=None, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, tree_folders=False):
    """
הפונקציה המרכיבת את הפקודה הראשית של התכנית. היא סורקת את התיקיות והקבצים תחת נתיב שצוין ויוצרת רשימה של קבצים להעתקה.
בסוף התהליך היא מעתיקה אותם אם הוכנס פרמטר של תיקית יעד.
    
תנאים:
    פרמטר 1 = נתיב תיקיה לסריקה
    פרמטר 2 = נתיב תיקית יעד להעברה אליה (אופציונלי)
    פרמטר 3 = הפעלת מצב העתקה (ברירת המחדל היא העברה)
    פרמטר 4 = מיון בתיקיות לפי א' ב'
    פרמטר 5 = העברה לתיקיות קיימות בלבד
    פרמטר 6 = יצירת תיקית "סינגלים" פנימית
    פרמטר 7 = מיון תיקיה ראשית בלבד/עץ תיקיות
מוגדר על ידי True או False.
    
תוצאה:
    מדפיס את רשימת האמנים שמופיעים במטאדאטה של השירים, ומעתיק אותם ליעד.
    """
    # סריקת עץ התיקיות או התיקיה הראשית בהתאם לבחירת המשתמש והכנסת שם הקבצים ושם האמן שלהם לרשימה
    info_list = []  
    if tree_folders is False:
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                file_path = os.path.join(root, my_file)
                if my_file.lower().endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

    # סריקת התיקיה הראשית בלבד ללא תיקיות פנימיות
    elif tree_folders:
        for my_file in os.listdir(dir_path):
            file_path = os.path.join(dir_path, my_file)
            if os.path.isfile(file_path):
                if my_file.lower().endswith((".mp3",".wma", ".wav")):
                    artist = artist_from_song(file_path)
                    if artist: info_list.append((file_path, artist))

        
    # הגדרות עבור תצוגת אחוזים
    len_dir = len(info_list)
    len_item = 0
    
    # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
    for file_path, artist in info_list:   
        # תצוגת אחוזים מתחלפת
        len_item += 1
        show_len = len_item * 100 // len_dir
        print(" " * 30, str(show_len), "% ", "הושלמו",end='\r')
                       
        # הגדרת משתנה עבור תיקית יעד בהתאם להתאמות האישיות של המשתמש
        if singles_folder and abc_sort:
            main_target_path = os.path.join(target_dir, artist[0], artist)
            target_path = os.path.join(target_dir, artist[0], artist, "סינגלים")
        elif singles_folder:
            main_target_path = os.path.join(target_dir, artist)
            target_path = os.path.join(target_dir, artist, "סינגלים")
        elif abc_sort:
            main_target_path = os.path.join(target_dir, artist[0], artist)
            target_path = os.path.join(target_dir, artist[0], artist)
        else:
            main_target_path = os.path.join(target_dir, artist)
            target_path = os.path.join(target_dir, artist)
        
        # יצירת תיקית יעד בתנאים מסויימים
        if exist_only is False:
            if not os.path.isdir(target_path):
                try: os.makedirs(target_path)
                except: pass
                
        elif exist_only and singles_folder:
            if os.path.isdir(main_target_path) and not os.path.isdir(target_path):
                try: os.makedirs(target_path)
                except: pass
        else:
            pass #לא תיווצר תיקיה חדשה


        # העברה או העתקה בהתאם להגדרות המשתמש
        if copy_mode and os.path.isdir(target_path):
            try:
                shutil.copy(file_path, target_path)
            except Exception as e:
                print("Error copying file:", e)
        elif os.path.isdir(target_path):
            try:
                shutil.move(file_path, target_path)
            except Exception as e:
                print("Error moving file:", e)





if __name__ == '__main__':
    MusicOrganizerApp().run()
