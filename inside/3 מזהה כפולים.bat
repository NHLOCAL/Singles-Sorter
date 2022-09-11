@echo off
chcp 1255>nul
chcp 1255
echo.
set /p p=">>>"
for %%i in (%p%) do set artist=%%~ni
for %%i in (%p%) do set p=%%~i
cd /d "%p%\סינגלים"
cls
path "C:\Users\משתמש\Desktop\ניסויים 2.5\קבצי באט שיצרתי\תוכנות באט ופייתון\מערכת הקבצים\מסדר הסינגלים\מסדר הסינגלים 2022\גרסאות מתקדמות\MediaInfo_CLI_22.06_Windows_x64";%path%

::ניתן לבצע ארבעה זיהויים שונים
::א. לפי שמות הקבצים
::ב. לפי גודל הקובץ
::ג. לפי אורך הקובץ
::ד. לפי תוכן הקובץ (הקוד הפנימי שלו)

::goto :len-sort


:name-sort
::מיון לפי שמות הקובץ


dir /b >"%temp%\000.txt"
set /p dir=<"%temp%\000.txt"

for /f "usebackq tokens=1,2,3,4,5,6,7,8,9 delims= " %%i in (%temp%\000.txt) do (
set i=%%i
set j=%%j
set k=%%k
set l=%%l
set m=%%m
set n=%%n
set o=%%o
call :func-scan
)
::סיכום הסריקה
cls
del "%temp%\number-find.txt"
del "%temp%\000.txt"
del "%temp%\list-to-delete-temp.tmp"
notepad "%temp%\list-to-delete.tmp"
pause
::מחיקת קבצים לפי הרשימה
cls
echo.
set /a num=1
for /f "eol=;tokens=1,1*delims=" %%a in (%temp%\list-to-delete.tmp) do (
set item=%%a
call :choice-delete
)
echo -------------------
:choicer
choice /c 123456789
if errorlevel 24 echo %item24% & pause & goto :choicer
if errorlevel 23 echo %item23% & pause & goto :choicer
if errorlevel 22 echo %item22% & pause & goto :choicer
if errorlevel 21 echo %item21% & pause & goto :choicer
if errorlevel 20 echo %item20% & pause & goto :choicer
if errorlevel 19 echo %item19% & pause & goto :choicer
if errorlevel 18 echo %item18% & pause & goto :choicer
if errorlevel 17 echo %item17% & pause & goto :choicer
if errorlevel 16 echo %item16% & pause & goto :choicer
if errorlevel 15 echo %item15% & pause & goto :choicer
if errorlevel 14 echo %item14% & pause & goto :choicer
if errorlevel 13 echo %item13% & pause & goto :choicer
if errorlevel 12 echo %item12% & pause & goto :choicer
if errorlevel 11 echo %item11% & pause & goto :choicer
if errorlevel 10 echo %item10% & pause & goto :choicer
if errorlevel 9  echo %item9% & pause & goto :choicer
if errorlevel 8  echo %item8% & pause & goto :choicer
if errorlevel 7  echo %item7% & pause & goto :choicer
if errorlevel 6  echo %item6% & pause & goto :choicer
if errorlevel 5  echo %item5% & pause & goto :choicer
if errorlevel 4  echo %item4% & pause & goto :choicer
if errorlevel 3  echo %item3% & pause & goto :choicer
if errorlevel 2  echo %item2% & pause & goto :choicer
if errorlevel 1  echo %item1% & pause & goto :choicer

pause
del "%temp%\list-to-delete.tmp"
exit

:choice-delete
echo [%num%] %item%
set item%num%=%item%
set /a num=num+1
exit /b



:func-scan
::מיון לפי המילים הראשונות בשם הקובץ
if "%i% %j%"=="" exit /b
if "%i% %j%"=="%artist%" exit /b
if "%i% %j% %k%"=="%artist%" exit /b
if "%j% %k%"=="%artist%" exit /b
if "%j% %k% %l%"=="%artist%" exit /b
dir /b | find /c "%i% %j% %k%">"%temp%\number-find.txt"
::הכנסת מספר תוצאות החיפוש למשתנה
set /p num=<"%temp%\number-find.txt"
::בדיקה האם הקובץ הכפול כבר מופיע ברשימת הכפולים
::הבדיקה סבוכה מעט כדי לוודא שלא מדלגים על קבצים שאינם כפולים
if %num% gtr 1 dir /b | find "%i% %j% %k%">"%temp%\list-to-delete-temp.tmp"
if exist "%temp%\list-to-delete-temp.tmp" if %num% gtr 1 for /f "eol=;tokens=1,1*delims=" %%b in (%temp%\list-to-delete-temp.tmp) do (find /c "%%b" "%temp%\list-to-delete.tmp"
if errorlevel 1 dir /b | find "%i% %j% %k%" >> "%temp%\list-to-delete.tmp"
)
cls
exit /b



:size-sort
::מיון לפי גודל הקובץ



:len-sort
::מיון לפי אורך הקובץ

::סריקת רשימת הקבצים לבדיקת אורכם
for %%i in (*.mp3,*.wma,*.wav) do (
set file=%%~i
call :func-len
)


pause
exit


:func-len
::הכנסת נתוני אורך השיר לקובץ על ידי כלי חיצוני
MediaInfo "%file%" | findstr /b "Duration">"%Temp%\len-song.tmp"
::הכנסת הנתונים מהקובץ לתוך משתנה
set/p len=<"%Temp%\len-song.tmp"
::חיתוך תוכן לא רצוי מהמשתנה
set "len=%len:~43%"
::הכנסת שם השיר + אורך השיר לקובץ
echo %file%,,%len% >>"%temp%\list-len-songs.tmp"
exit /b