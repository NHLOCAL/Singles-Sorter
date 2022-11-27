# -*- coding: utf-8 -*-
import os
import codecs

os.system("MODE CON COLS=80 lines=27")
os.system("title %VER% מסדר הסינגלים")
os.system("color f1")


#קביעת משתנה למיקום קובץ הדאטה
csv_file = r"%appdata%\singles-sorter\singer-list.csv"
personal_csv_file = r"%appdata%\singles-sorter\personal-singer-list.csv"


os.system("pause")

def singles_sorter():
    filename = r"C:\Users\אורי\Videos\פייתון\document.xml"
    reading = codecs.open(filename, "r", "utf-8")
    strfile = r"C:\Users\אורי\Videos\פייתון\past.xml"
    writing = codecs.open(strfile, "a", "utf-8")
    readstr = str(reading.read())
    listlines = readstr.split("<w:")
    for line in listlines:
        writing.write(line + "\n")
    reading.close()
    writing.close()




# <w:t> </w:t>
# 
