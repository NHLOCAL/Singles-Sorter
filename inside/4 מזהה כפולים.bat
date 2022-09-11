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

::הצגת רשימת השירים למחיקה
echo -------------------
:choicer
choice /c ABCDEFGHIJKLMNOPQRSTUVWXYZ
if errorlevel 26 echo %itemZ% |rev & pause & goto :choicer
if errorlevel 25 echo %itemY% |rev & pause & goto :choicer
if errorlevel 24 echo %itemX% |rev & pause & goto :choicer
if errorlevel 23 echo %itemW% |rev & pause & goto :choicer
if errorlevel 22 echo %itemV% |rev & pause & goto :choicer
if errorlevel 21 echo %itemU% |rev & pause & goto :choicer
if errorlevel 20 echo %itemT% |rev & pause & goto :choicer
if errorlevel 19 echo %itemS% |rev & pause & goto :choicer
if errorlevel 18 echo %itemR% |rev & pause & goto :choicer
if errorlevel 17 echo %itemQ% |rev & pause & goto :choicer
if errorlevel 16 echo %itemP% |rev & pause & goto :choicer
if errorlevel 15 echo %itemO% |rev & pause & goto :choicer
if errorlevel 14 echo %itemN% |rev & pause & goto :choicer
if errorlevel 13 echo %itemM% |rev & pause & goto :choicer
if errorlevel 12 echo %itemL% |rev & pause & goto :choicer
if errorlevel 11 echo %itemK% |rev & pause & goto :choicer
if errorlevel 10 echo %itemJ% |rev & pause & goto :choicer
if errorlevel 9 echo %itemI% |rev & pause & goto :choicer
if errorlevel 8 echo %itemH% |rev & pause & goto :choicer
if errorlevel 7 echo %itemG% |rev & pause & goto :choicer
if errorlevel 6 echo %itemF% |rev & pause & goto :choicer
if errorlevel 5 echo %itemE% |rev & pause & goto :choicer
if errorlevel 4 echo %itemD% |rev & pause & goto :choicer
if errorlevel 3 echo %itemC% |rev & pause & goto :choicer
if errorlevel 2 echo %itemB% |rev & pause & goto :choicer
if errorlevel 1 echo %itemA% |rev & pause & goto :choicer


pause
del "%temp%\list-to-delete.tmp"
exit

:choice-delete
if %num%==1 set let=A
if %num%==2 set let=B
if %num%==3 set let=C
if %num%==4 set let=D
if %num%==5 set let=E
if %num%==6 set let=F
if %num%==7 set let=G
if %num%==8 set let=H
if %num%==9 set let=I
if %num%==10 set let=J
if %num%==11 set let=K
if %num%==12 set let=L
if %num%==13 set let=M
if %num%==14 set let=N
if %num%==14 set let=O
if %num%==15 set let=P
if %num%==16 set let=Q
if %num%==17 set let=R
if %num%==18 set let=S
if %num%==19 set let=T
if %num%==20 set let=U
if %num%==21 set let=V
if %num%==22 set let=W
if %num%==23 set let=X
if %num%==24 set let=Y
if %num%==25 set let=Z
echo [%let%] %item%
set item%let%=%item%
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