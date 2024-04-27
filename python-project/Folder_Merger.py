'''
1. קבלת תוכן קובץ CSV והכנסתו לרשימה.
2. מיזוג תיקיות זמרים לפי הרשימה.
3. מעבר על התיקיות שנותרו עם פונקציית זיהוי דמיונות.
4. לתת רשימה של כל השמות הדומים ולאפשר למשתמש לבחור מה באמת דומה ומה לא.
(יש לשים לב שאין את אותם השמות פעמיים)
5. לקבץ שמות דומים לטאפלים של שתיים או יותר,
ולשאול את המשתמש באיזה שם ברירת מחדל לבחור.
6. ליצור רשימה עם על אחד מהשמות שבטאפלים עם השם שנבחר.
7. להכניס את התוכן הזה לקובץ CSV חדש (בתנאי ששם היעד מכיל 2 מילים או יותר).
8. לבצע מיזוג תיקיות לפי הרשימה.
'''

import csv
import os
import sys
import shutil
from identify_similarities import similarity_sure


class SingerMerger:
    def __init__(self, dir_path, file_path):
        self.dir_path = dir_path
        self.file_path = file_path
        self.dir_listing = os.listdir(dir_path)
        self.singer_list = self.read_csv()
        self.similarity_set = set()

    def read_csv(self):
        with open(self.file_path, 'r') as file:
            csv_reader = csv.reader(file)
            return [tuple(row) for row in csv_reader]

    def merge_folders_by_csv(self):
        os.chdir(self.dir_path)
        for source_name, target_name in self.singer_list:
            if source_name == target_name:
                continue
            if source_name not in self.dir_listing or target_name not in self.dir_listing:
                continue

            old_path = os.path.join(os.getcwd(), source_name)
            new_path = os.path.join(os.getcwd(), target_name)

            if os.path.exists(old_path) and not os.path.exists(new_path):
                os.rename(old_path, new_path)
                print(f"{old_path} -->\n{new_path}")
            elif os.path.exists(old_path):
                for filename in os.listdir(old_path):
                    source_path = os.path.join(old_path, filename)
                    destination_path = os.path.join(new_path, filename)
                    shutil.move(source_path, destination_path)
                print(f"{old_path} -->\n{new_path}")
                shutil.rmtree(old_path)

    def check_similarity(self, artist):
        list_dirs = self.dir_listing
        if not list_dirs:
            return None

        answer, similarity_str = similarity_sure(artist, list_dirs, False)
        if answer:
            return similarity_str
        else:
            return None

    def create_similarity_list(self):
        folders_list = [item for item in self.dir_listing if os.path.isdir(os.path.join(self.dir_path, item))]

        for artist in folders_list:
            similarity_str = self.check_similarity(artist)

            # Check if the similarity pair has been processed
            set_item = (artist, similarity_str)
            if similarity_str and set_item not in self.similarity_set:
                print(f'Found similar names - merge into folder "{artist}" or "{similarity_str}"?')
                print('Press 1 to merge into first folder, 2 to merge into second folder, or Enter to skip.')
                answer = input(">>> ")

                try:
                    if int(answer) == 1:
                        self.similarity_set.add(set_item)
                        self.merge_folders(similarity_str, artist)
                    elif int(answer) == 2:
                        self.similarity_set.add(set_item)
                        self.merge_folders(artist, similarity_str)

                except ValueError:
                    pass

        self.merge_folders_by_csv()



    def merge_folders(self, source_name, target_name):
        if source_name == target_name:
            return
        if source_name not in self.dir_listing or target_name not in self.dir_listing:
            return

        old_path = os.path.join(self.dir_path, source_name)
        new_path = os.path.join(self.dir_path, target_name)

        if os.path.exists(old_path) and not os.path.exists(new_path):
            os.rename(old_path, new_path)
            print(f"{old_path} -->\n{new_path}")
        elif os.path.exists(old_path):
            for filename in os.listdir(old_path):
                source_path = os.path.join(old_path, filename)
                destination_path = os.path.join(new_path, filename)
                shutil.move(source_path, destination_path)
            print(f"{old_path} -->\n{new_path}")

            try:
                shutil.rmtree(old_path)
            except PermissionError as e:
                print(f"Permission error: {e}. Skipping directory deletion.")



def main():
    dir_path = r'D:\שמע\מסודר מחדש\זמרי שירים בודדים'
    file_path = r"C:\Users\משתמש\AppData\Roaming\singles-sorter\singer-list.csv"
    singer_merger = SingerMerger(dir_path, file_path)
    singer_merger.create_similarity_list()


if __name__ == '__main__':
    main()
