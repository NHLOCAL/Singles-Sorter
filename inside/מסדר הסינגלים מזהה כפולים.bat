@echo off
chcp 1255>nul

path "C:\Users\משתמש\Desktop\ניסויים 2.5\קבצי באט שיצרתי\תוכנות באט ופייתון\מערכת הקבצים\מסדר הסינגלים\מסדר הסינגלים 2022\גרסאות מתקדמות\MediaInfo_CLI_22.06_Windows_x64";%path%

::ניתן לבצע שלוש זיהויים שונים
::א. לפי שמות הקבצים
::ב. לפי גודל הקובץ
::ג. לפי אורך הקובץ

::goto :len-sort


:name-sort
::מיון לפי שמות הקובץ


dir /b >000.txt
set /p dir=<000.txt

for /f "usebackq tokens=1,2,3,4,5,6,7,8,9 delims= "  %%i in (000.txt) do (
set i=%%i
set j=%%j
set k=%%k
set l=%%l
set m=%%m
set n=%%n
call :func-scan
)
pause
exit

:func-scan
dir /b | find /c "%i% %j% %k%">"number-find.txt"
set /p num=<"number-find.txt" 
if %num% gtr 1 dir /b | find "%i% %j% %k%"
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