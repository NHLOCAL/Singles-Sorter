::מטרת הסקריפט היא לסדר שירים בודדים בתיקיות לפי אמנים
::הסקריפט מיועד לציבור החרדי ובשל כך הדאטה שלו מותאמת לציבור זה
::קרדיט: nh.local11@gmail.com

@echo off
::הגדרות של שפה, צבע, כותרת וגודל החלון
::ועוד מספר משתנים חשובים
chcp 1255>nul
set "VER=10.0"
title %VER% מסדר הסינגלים
MODE CON COLS=80 lines=27
::קביעת משתנה למיקום קובץ הדאטה
set "csv-file=%appdata%\singles-sorter\singer-list.csv"

:call-num
::בדיקת מספר הזמרים הקיים כעת בסקריפט
::הדבר נצרך לצורך חישוב הזמן שעבר
::ולצורך הפונקציה של הוספת זמרים עצמאית
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"
set/a abc=%ab%


::מסדר הסינגלים עצמו
:mesader-singels
color f1


:beginning
cls
set/p source_path=<"%temp%\mesader-sourceB.tmp"
del "%temp%\mesader-sourceB.tmp"

:target_folder
cls
set/p h=<"%temp%\mesader-targetB.tmp"
del "%temp%\mesader-targetB.tmp"

::קביעת התיקיה הנוכחית לתיקית המקור
cd /d "%source_path%"

::קביעת משתנים לצורך הגדרות המשתמש

::הגדרת משתנה לניקוי שמות הקבצים
set/p cleaning=<"%tmp%\select7_tmp.tmp"
if "%cleaning%"=="False" (set "clear_heb=ליעפ אל") else (set "clear_heb=ליעפ")

::הגדרת משתנה העברה או העתקה
set/p copy_moving=<"%tmp%\select3_tmp.tmp"
if "%copy_moving%"=="True" (
set c_or_m=xcopy
set par=/y
set "msg=וקתעוהש"
set cm_heb=הקתעה
) else (
set par=
set c_or_m=move
set "msg=ורבעוהש"
set cm_heb=הרבעה
)

::קביעת משתנה ליצירת תיקיות ראשיות בחלוקה לא' ב'
set/p abc_dirs_creating=<"%tmp%\select2_tmp.tmp"
if "%abc_dirs_creating%"=="True" (set "abc_heb=ליעפ") else (set "abc_heb=ליעפ אל")

::הגדרת משתנה ליצירת תיקית סינגלים פנימית
set/p in_folder_creating=<"%tmp%\select1_tmp.tmp"
if "%in_folder_creating%"=="True" (
set sing_heb=ליעפ
set "s=\סינגלים"
)else (
set "sing_heb=ליעפ אל"
set s=
)

::הגדרת משתנה ליצירת תיקיות חדשות או העברה לתיקיות קיימות בלבד
set/p creating_folder=<"%tmp%\select4_tmp.tmp"
if "%creating_folder%"=="True" (set fixed_heb=ליעפ) else (set "fixed_heb=ליעפ אל")

::הגדרת משתנה להפעלת סריקה מתקדמת
set/p pro_scanning=<"%tmp%\select6_tmp.tmp"
if "%pro_scanning%"=="False" (set "artist_heb=ליעפ אל") else (set "artist_heb=ליעפ")

::הגדרת משתנה לסריקת תיקיות משנה
set/p tree_scanning=<"%tmp%\select6_tmp.tmp"
if "%tree_scanning%"=="True" (set "dir_heb=ליעפ") else (set "dir_heb=ליעפ אל")


:intro
::ביצוע ניקוי לשמות הקבצים
::אם הוגדר כך על ידי המשתמש
cls
if "%clear_heb%"=="ליעפ" (
for /r %%i in (*) do (
cls
echo.
echo                           ...םיצבקה תומש לש יוקינ עצבמ
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)
)
goto :preparing

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


:preparing
cls
echo.
echo                                    ...דבוע
::מספר הגדרות משתנים חשובות

::הגדרת סריקת תיקיות משנה לפי העדפות המשתמש
if "%dir_heb%"=="ליעפ" (set tree=/r) else (set tree=)
::איפוס מספרים עבור חישוב אחוזים
set/a d=1
::איפוס משתנה עבור סריקה מתקדמת לפי אמן
set pro_scan=False
:start
::פקודת הפור הראשית שסורקת שמות קבצים
for /f "usebackq tokens=1,2 delims=,"  %%i in (%csv-file%) do (
set a=%%i
set c=%%j
call :sort-func
)
goto :finish

:sort-func
::הגדרת שם הזמר לחיפוש
set a=*%a: =?%*.*

::הגדרת משתנה להצגת מספר אחוזים שהושלמו
set/a en=%d%00/ab
if not "%en%"=="%enb%" cls & echo. & echo                                    ...דבוע & echo. & echo. & echo. & echo                               ...ומלשוה םיזוחא %en%
set/a enb=%d%00/ab

::קובע אם יווצרו תקיות ראשיות לפי א ב
if "%abc_heb%"=="ליעפ" set w=%c:~0,1%\

::קביעת נתיב יעד עם מספר משתנים - בהתאם להגדרות המשתמש
set b="%h%\%w%%c%%s%"

::יצירת תיקית זמר בכפוף לכמה תנאים
set xx=v
set ss=z
for %tree% %%c in ("%a%") do if exist %%c set ss=ss
if "%fixed_heb%"=="ליעפ" if not exist "%h%\%w%%c%" set ss==z
if %ss%==ss md %b%

::הגדרת מחיקת קבצי מקור לאחר העתקה אם הוגדר כך
if %c_or_m%==del set b= & set par=/q

::העתקת הסינגלים בכפוף לכמה תנאים
for %tree% %%d in ("%a%") do if exist %%d set xx=xx
if exist %b% if %xx%==xx for %tree% %%e in (%a%) do %c_or_m% %par% "%%e" %b%>>םוכיס

::מעבר למספר הבא לצורך חישוב ההתקדמות
set/a d=d+1

::יציאה מהפונקציה וחזרה לפקודת הפור
exit /b


:finish
cls
echo.
echo.
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !וקחמנ םיצבקה & echo. & del םוכיס & goto :intro_pro
if exist םוכיס (echo                                 :%msg% םיצבקה רפסמ & find /c "1" םוכיס
) else (
echo                                   !רבד אצמנ אל
)
if exist םוכיס (del םוכיס) else (goto :intro_pro)
echo.
::אם התבצעה העתקה ניתנת אפשרות למחוק את קבצי המקור
if not "%pro_scan%"=="True" if %c_or_m%==xcopy echo. & echo                 [2] תעכ שקה םיירוקמה םיצבקה תא קוחמל ןיינועמ התא םא & echo                         [1] שקה םתוא רומשל ןיינועמ התא םא & echo. & echo               !הקיחמב רוחבל רוסא היקית התוא םה דעיהו ביתנה םא !תוריהז & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto preparing & if errorlevel 1 goto :intro_pro

:intro_pro
::אם הוגדרה סריקה לפי אמן
::יתבצע מעבר לפונקציה זו
if "%artist_heb%"=="ליעפ" goto :pro_scanner

:pause
echo.
echo.
echo.
echo ================================================================================
echo                               !החלצהב םייתסה ךילהתה
pause>nul
exit


:pro_scanner
::הוספת מיקום התוכנה החיצונית למשתנה הסביבה
path "%AppData%\singles-sorter";%path%
echo.
timeout 10 | echo            ךלש תורדגהה יפ לע ןמא יפל תמדקתמ הקירס לחת תוינש רפסמ דועב
for %tree% %%s in (*.mp3,*.wma,*.wav) do (
set file=%%~s
call :scanner_func
set/a d=d+1
)
::מחיקת קבצים זמניים והגדרת משתנים חשובים
::עבור תצוגת הסיכום
del "%Temp%\artist-song.tmp"
del "%Temp%\artist-song-ansi.tmp"
set "artist_heb=ליעפ אל"
set pro_scan=True
goto :finish

:scanner_func
::תצוגה למשתמש
cls
echo.
echo                                    ...דבוע

:: שימוש בתוכנה חיצונית להכנסת נתוני השיר לקובץ
::חיפוש שורה המכילה את שם האמן בתוך הקובץ
:: והכנסת השורה לקובץ
for /f "tokens=1,2 delims=:" %%i in ('mediainfo "%file%" ^| findstr /b "Performer"') do echo %%j>"%Temp%\artist-song.tmp"

::בדיקה אם הקובץ ריק לפי גודל הקובץ
::ויציאה מהפונקציה אם התשובה חיובית
for %%h in ("%Temp%\artist-song-ansi.tmp") do (if %%~zh==0 exit /b)

::המרת קובץ הפלט לפורמט אנסי התואם לבאט
powershell "(Get-Content "%Temp%\artist-song.tmp" -Encoding utf8 | Out-File "%Temp%\artist-song-ansi.tmp" -Encoding default)"

::בדיקה אם קיימים תוים בעייתים בקובץ
::ויציאה מהפונקציה אם התשובה חיובית
find /c """" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b
find /c "?" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b

:: העברת תוכן הקובץ למשתנה
set/p artist=<"%Temp%\artist-song-ansi.tmp"


::הסרת תוכן מהקובץ והשארת שם האמן בלבד
set "artist=%artist:~1%"

::במקרה ולא:
::חיפוש שם האמן בתוך הקובץ הנוכחי
::אם הוא קיים מתבצעת העברה של השיר
::לתוך תיקיה המוגדרת כשם האמן
if "%abc_heb%"=="ליעפ" set w=%artist:~0,1%\
find /c "%artist%" "%csv-file%">nul
if %errorlevel%==0 (
if "%fixed_heb%"=="ליעפ אל" (if not exist "%h%\%w%%artist%%s%" md "%h%\%w%%artist%%s%"
) else (
if exist "%h%\%w%%artist%" if not exist "%h%\%w%%artist%%s%" md "%h%\%w%%artist%%s%")
if exist "%h%\%w%%artist%%s%" %c_or_m% %par% "%file%" "%h%\%w%%artist%%s%">>םוכיס
)
::יציאה מהפונקציה וחזרה לפקודת הפור
exit /b




::קרדיט: nh.local11@gmail.com