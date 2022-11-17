@echo off
chcp 1255

::הרצה כמנהל
if not "%1"=="am_admin" (powershell start -verb runas '%0' am_admin & exit /b)


::מחיקת קיצורי דרך
del "%userprofile%\Desktop\מסדר הסינגלים 10.lnk"
del "%AppData%\Microsoft\Windows\Start Menu\Programs\מסדר הסינגלים\מסדר הסינגלים 10.0.lnk"

::מחיקת ערך הרישום האחראי על רשימת התוכנות להסרה
reg delete "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\SinglesSorter" /f

::מחיקת קבצי התוכנה
for %%i in ("%appdata%\singles-sorter\*.*") do (if not "%%~nxi"=="remover_ss.bat" del "%%~i")
del %0%