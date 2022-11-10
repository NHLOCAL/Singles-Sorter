@echo on
CHCP 1255>nul

::תיקון קידוד קובץ נתיב מקור
powershell "(Get-Content "%temp%\mesader-source.tmp" -Encoding utf8 | Out-File "%temp%\mesader-sourceB.tmp" -Encoding default)"

::תיקון קידוד קובץ נתיב יעד
powershell "(Get-Content "%temp%\mesader-target.tmp" -Encoding utf8 | Out-File "%temp%\mesader-targetB.tmp" -Encoding default)"

::פתיחת קובץ ההרצה של מסדר הסינגלים בחלון חדש
start "New Window" cmd /c "%appdata%\singles-sorter\SinglesSorterC.bat"
exit