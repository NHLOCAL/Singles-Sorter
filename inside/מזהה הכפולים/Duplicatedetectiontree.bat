@echo off
chcp 1255>nul
MODE CON: COLS=70 lines=40
color f1
title מזהה הכפולים 7.0 בטא
echo.
if not [%1]==[] set p=%1 & goto :start

:begin
cls
echo Insert singer folder path to search
echo (on the folder to contain singles folder)
echo.
set /p p=">>>"

:start
if exist "%temp%\list-to-del.csv" del "%temp%\list-to-del.csv"
for %%i in (%p%) do set artist=%%~ni
for %%i in (%p%) do set p=%%~i
if exist "%p%\סינגלים" (
cd /d "%p%\סינגלים"
) else (
echo !םילגניס תיקית הליכמ אל היקיתה
timeout 2
exit /b )

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

dir /b >"%temp%\000.tmp"

for /f "usebackq tokens=1,2,3,4,5,6,7,8,9 delims= " %%i in (%temp%\000.tmp) do (
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
if exist "%temp%\000.tmp" del "%temp%\000.tmp"
if exist %del_file_tmp% del %del_file_tmp%
if not exist %del_file% (
echo not multiply files
timeout 2
exit /b
)


::מחיקת קבצים לפי הרשימה
cls
echo.
set /a num=1
for /f "eol=;tokens=1,1*delims=" %%a in (%temp%\list-to-delete.tmp) do (
set item="%%~a"
set viwe_item="%%~na"
call :choice-delete
)
del %del_file%


::הצגת רשימת השירים למחיקה

echo -------------------
echo Press any key to delete file
echo Press 0 to start again

:choicer
set item_num=
set/p item_num=">>>"
if not defined item_num goto :choicer
if %item_num%==0 exit /b
for /f "tokens=1,2 delims=:"  %%i in ('type "%Temp%\list-to-del.csv"  ^| findstr /l "[%item_num%]"') do (
del %%j && echo the file is deleted!
)
goto :choicer

pause
exit /b

:choice-delete

echo [%num%] %viwe_item% 
echo [%num%]:%item%>>"%Temp%\list-to-del.csv"
set /a num=num+1
exit /b



:func-scan
::מיון לפי המילים הראשונות בשם הקובץ
::וכן בדיקה אם המילים המזוהות הם שם הזמר
::ובמקרה זה לעבור למילים הבאות
if "%i% %j%"=="" exit /b
set "file_tokens=%i% %j% %k%"
if "%i% %j%"=="%artist%" set "file_tokens=%k% %l% %m%"
if "%i% %j% %k%"=="%artist%" set "file_tokens=%l% %m% %n%"
if "%j% %k%"=="%artist%" set "file_tokens=%l% %m% %n%"
if "%j% %k% %l%"=="%artist%" set "file_tokens=%m% %n% %o%"
if "%artist%" == "%file_tokens%" exit /b

::הכנסת מספר תוצאות החיפוש למשתנה
for /f "tokens=1,2* delims=" %%i in ('dir /b ^| find /c "%file_tokens%"') do (set num=%%i)


::בדיקה האם הקובץ הכפול כבר מופיע ברשימת הכפולים
::הבדיקה סבוכה מעט כדי לוודא שלא מדלגים על קבצים שאינם כפולים
set del_file="%temp%\list-to-delete.tmp"
set del_file_tmp="%temp%\list-to-delete-temp.tmp"

if %num% gtr 1 dir /b | find "%file_tokens%">%del_file_tmp%
if exist %del_file_tmp% if %num% gtr 1 for /f "eol=;tokens=1,1*delims=" %%b in (%temp%\list-to-delete-temp.tmp) do (find /c "%%b" %del_file%
if errorlevel 1 dir /b | find "%file_tokens%">>%del_file%
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
exit /b


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