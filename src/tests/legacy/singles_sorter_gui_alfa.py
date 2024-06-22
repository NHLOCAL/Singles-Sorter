import sys
from PyQt6.QtWidgets import QApplication, QWidget, QPushButton, QLabel, QRadioButton, QVBoxLayout, QHBoxLayout, QMessageBox, QFileDialog, QButtonGroup, QLineEdit, QProgressBar
from PyQt6.QtCore import Qt
import os
from singles_sorter_func import *


class MainWindow(QWidget):
    def __init__(self):
        super().__init__()
        self.initUI()

    def initUI(self):
        self.setWindowTitle('Music Files Organizer')
        self.setGeometry(100, 100, 400, 300)

        self.input_labels = ['Source Directory:', 'Target Directory:', 'Copy Mode:', 'Scan Tree Folders:', 'Create Singles Folder:', 'Exist Only:', 'Sort by ABC:']
        self.inputs = []
        self.radio_button_groups = []

        vbox = QVBoxLayout()

        for label_text in self.input_labels:
            hbox = QHBoxLayout()
            label = QLabel(label_text)
            hbox.addWidget(label)

            # For folder selection, allow both manual input and selection through file dialog
            if 'Directory' in label_text:
                edit = QLineEdit()
                hbox.addWidget(edit)
                self.inputs.append(edit)

                button = QPushButton('Select Folder')
                button.clicked.connect(lambda checked, edit=edit: self.select_folder(edit))
                hbox.addWidget(button)
            else:
                # For other options, use radio buttons
                true_radio = QRadioButton('True')
                false_radio = QRadioButton('False')
                hbox.addWidget(true_radio)
                hbox.addWidget(false_radio)

                # Group the radio buttons together
                button_group = QButtonGroup(self)
                button_group.addButton(true_radio)
                button_group.addButton(false_radio)

                self.radio_button_groups.append(button_group)

            vbox.addLayout(hbox)

        # Add progress bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setAlignment(Qt.AlignmentFlag.AlignCenter)
        vbox.addWidget(self.progress_bar)

        self.btn_run = QPushButton('Run', self)
        self.btn_run.clicked.connect(self.run_script)
        vbox.addWidget(self.btn_run)

        self.setLayout(vbox)

    def select_folder(self, edit):
        folder_path = QFileDialog.getExistingDirectory(self, 'Select Folder')
        if folder_path:
            edit.setText(folder_path)

    def run_script(self):
        parameters = [str(edit.text()) for edit in self.inputs]

        radio_values = []
        for group in self.radio_button_groups:
            # Get the checked radio button in each group
            selected_button = group.checkedButton()
            # Append the text of the selected radio button to the list
            radio_values.append(selected_button.text())

        try:
            dir_path = os.path.join(parameters[0])
            target_dir = os.path.join(parameters[1])
            copy_mode = True if radio_values[0] == 'True' else False
            tree_folders = True if radio_values[1] == 'True' else False
            singles_folder = True if radio_values[2] == 'True' else False
            exist_only = True if radio_values[3] == 'True' else False
            abc_sort = True if radio_values[4] == 'True' else False

            # Call your function with the provided parameters
            self.scan_dir(dir_path, target_dir, copy_mode, abc_sort, exist_only, singles_folder, tree_folders)

        except Exception as e:
            QMessageBox.critical(self, 'Error', f'An error occurred: {e}')

    # מעבר על עץ התיקיות שהוגדר
    def scan_dir(self, dir_path, target_dir=None, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, tree_folders=False):

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

        len_dir = len(info_list)
        progress_generator = progress_display(len_dir)

        # מעבר על תוצאות הסריקה והדפסתם בכפוף למספר תנאים
        for file_path, artist in info_list:   
            self.progress_bar.setValue(next(progress_generator))
            QApplication.processEvents()  # Ensure the UI updates
                        
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
                try: copy(file_path, target_path)
                except: pass
            elif os.path.isdir(target_path):
                try: move(file_path, target_path)
                except: pass
                
if __name__ == '__main__':
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())
