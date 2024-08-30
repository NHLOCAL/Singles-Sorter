::מטרת הסקריפט היא לסדר שירים בודדים בתיקיות לפי אמנים
::הסקריפט מיועד לציבור החרדי ובשל כך הדאטה שלו מותאמת לציבור זה
::קרדיט: nh.local11@gmail.com

@echo off
::הגדרות של שפה, צבע, כותרת וגודל החלון
::ועוד מספר משתנים חשובים
chcp 1255>nul
set "VER=12.8"
title %VER% מסדר הסינגלים
MODE CON COLS=80 lines=18
color f1


:beginning
cls
set source_path=%1
set "h=%~2"

::במידה והוכנס פרמטר "clean"
::יבוצע ניקוי קבצים בלבד
if [%2]==[-clean] goto :intro


:preparing
echo.
echo.
echo.

singles_sorter_func.exe %source_path% "%h%" %3 %4 %5 %6 %7


echo.
echo.
echo.
echo                             !בוט לזמ !לכה ונמייס 
echo.
echo                      ןולחה תא רוגסל ידכ והשלכ שקמ לע שקה
pause>nul

::לבינתיים יציאה מידית
exit



:intro
::ביצוע ניקוי לשמות הקבצים
::אם הוגדר כך על ידי המשתמש
cls
cd /d %source_path%
for /r %%i in (*) do (
cls
echo.
echo                           ...םיצבקה תומש לש יוקינ עצבמ
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)

::יציאה במקרה והוגדר ניקוי קבצים בלבד
if [%2]==[-clean] pause & exit

:clear-func
::הפונקציה מבצעת ניקוי של שמות הקבצים
set "new_filename=%file:_= %"
set "new_filename=%new_filename: -מייל מיוזיק=%"
set "new_filename=%new_filename: - ציצו במייל=%"
set "new_filename=%new_filename: -מייל מיוזיק=%"
set "new_filename=%new_filename:-חדשות המוזיקה=%"
set "new_filename=%new_filename: - חדשות המוזיקה=%"
set "new_filename=%new_filename: - ציצו=%"
set "new_filename=%new_filename: מוזיקה מכל הלב=%"
set "new_filename=%new_filename: - מייל מיוזיק=%"
ren "%file%%ext%" "%new_filename%%ext%"
exit /b