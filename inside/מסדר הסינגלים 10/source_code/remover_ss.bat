@echo off


::הרצה כמנהל
if not "%1"=="am_admin" (powershell start -verb runas '%0' am_admin & exit /b)
chcp 1255
mode con cols=16 lines=1

::מחיקת קיצורי דרך
del "%userprofile%\Desktop\מסדר הסינגלים 10.lnk"
rd /s /q "%AppData%\Microsoft\Windows\Start Menu\Programs\מסדר הסינגלים"

::מחיקת ערך הרישום האחראי על רשימת התוכנות להסרה
reg delete "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\SinglesSorter" /f

::מחיקת קבצי התוכנה
for %%i in ("%appdata%\singles-sorter\*.*") do (if not "%%~nxi"=="remover_ss.bat" del "%%~i")
del %0%