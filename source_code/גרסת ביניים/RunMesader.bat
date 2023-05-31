@echo on
CHCP 1255>nul

::פתיחת קובץ ההרצה של מסדר הסינגלים בחלון חדש
start "New Window" cmd /c SinglesSorterC.bat %1 %2 %3 %4 %5 %6 %7
exit