@echo off
chcp 1255
set "csv-file=%appdata%\singles-sorter\singer-list.csv"
set /p path_cd="PATH:>>>"
cd /d %path_cd%
set "is_file=%0%"
path "C:\Users\משתמש\Desktop\ניסויים 2.5\קבצי באט שיצרתי\תוכנות באט ופייתון\מערכת הקבצים\מסדר הסינגלים\מסדר הסינגלים 2022\גרסאות מתקדמות\MediaInfo_CLI_22.06_Windows_x64";%path%

for /r %%s in (*.mp3) do (
set file=%%~s
call :func
)
del "%Temp%\artist-song.tmp"
del "%Temp%\artist-song-ansi.tmp"
pause
exit

:func
:: שימוש בתוכנה חיצונית להכנסת נתוני השיר לקובץ
::חיפוש שורה המכילה את שם האמן בתוך הקובץ
:: והכנסת השורה לקובץ
mediainfo "%file%" | findstr /b "Performer">"%Temp%\artist-song.tmp"
::המרת קובץ הפלט לפורמט אנסי התואם לבאט
powershell "(Get-Content "%Temp%\artist-song.tmp" -Encoding utf8 | Out-File "%Temp%\artist-song-ansi.tmp" -Encoding default)"

:: העברת תוכן הקובץ למשתנה
set/p artist=<"%Temp%\artist-song-ansi.tmp"
::יציאה מהפונקציה במקרה והמשתנה ריק
if "%artist%"=="" exit /b
::במקרה ולא:
::הסרת תוכן מהקובץ והשארת שם האמן בלבד
set "artist=%artist:~43%"
::חיפוש שם האמן בתוך הקובץ הנוכחי
::אם הוא קיים מתבצעת העברה של השיר
::לתוך תיקיה המוגדרת כשם האמן
find /c "%artist%" "%csv-file%">nul
if %errorlevel%==0 (
md "מסודר\%artist%"
copy "%file%" "מסודר\%artist%"
)
::יציאה מהפונקציה וחזרה לפקודת הפור
exit /b