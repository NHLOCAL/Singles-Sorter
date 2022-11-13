::מטרת הסקריפט היא לסדר שירים בודדים בתיקיות לפי אמנים
::הסקריפט מיועד לציבור החרדי ובשל כך הדאטה שלו מותאמת לציבור זה
::קרדיט: nh.local11@gmail.com

@echo off
::הגדרות של שפה, צבע, כותרת וגודל החלון
::ועוד מספר משתנים חשובים
chcp 1255>nul
set "VER=9.1"
title %VER% מסדר הסינגלים
MODE CON COLS=80 lines=27
if [%1]==[] call :logo_show

::בדיקה אם קיים קובץ דאטה ב-אפפדאטה או בתיקית הסקריפט
::וקביעת משתנה למיקום קובץ הדאטה
::יצירת קובץ דאטה זמרים במקרה והוא לא קיים
if exist "singer-list.csv" (
set csv-file=singer-list.csv
) else (
if exist "%appdata%\singles-sorter\singer-list.csv" (
set "csv-file=%appdata%\singles-sorter\singer-list.csv"
) else (
call :creat-cvs
set csv-file="%appdata%\singles-sorter\singer-list.csv"
)
)

:call-num
::בדיקת מספר הזמרים הקיים כעת בסקריפט
::הדבר נצרך לצורך חישוב הזמן שעבר
::ולצורך הפונקציה של הוספת זמרים עצמאית
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"
set/a abc=%ab%

::הגדרת משתנה לתיקית המקור ודילוג לשלב 2
::במקרה שהמשתמש גרר תיקיה על גבי הסקריפט
if not [%1]==[] (
set "source_path=%1"
color f1
call :drag_func
)
goto :down-mediainfo

:drag_func
for %%i in (%source_path%) do set source_path=%%~i
if exist "%source_path%\" goto :target_folder
exit /b


:down-mediainfo
::פונקציה מתקדמת המאפשרת חיפוש לפי שם האמן המופיע בקובץ
::העקת קובץ התוכנה לתיקית העבודה או הורדה שלו
::הורדת תוכנת עזר לפונצית מיון לפי אמן
set mi="%appdata%\singles-sorter\MediaInfo.exe"
if not exist %mi% if exist "%~dp0MediaInfo.exe" (
copy "%~dp0MediaInfo.exe" "%AppData%\singles-sorter"
) else (
curl -LJO https://github.com/NHLOCAL/Singles-Sorter/raw/main/data/MediaInfo.exe
move MediaInfo.exe "%AppData%\singles-sorter"
)

::בדיקה אם גרסה חדשה זמינה להורדה
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :mesader-singels else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)

goto :mesader-singels

::קוד להוספת זמרים על ידי המשתמש
:singer-list-new
cls
echo.
echo.
echo.
echo.[30m 
echo                             םירמז תפסוה - םילגניסה רדסמ
echo                      ========================================= [34m 
echo.
echo.
echo                           [1] ץבוק ךותמ םירמז תומכ תפסוהל
echo                        [2] הנכותה תיקיתל םירמזה תמישר תקתעהל
echo                               [3] ישארה טירפתל הרזחל
echo.
echo                                 תויורשפאהמ תחאב רחב
choice /c 123>nul
if errorlevel 3 goto :mesader-singels
if errorlevel 2 (@copy "%csv-file%" "%~dp0"
echo.
timeout 2 | echo                                    !קתעוה ץבוקה
goto :mesader-singels
)
if errorlevel 1 "%csv-file%" & call :call-num


::מסדר הסינגלים עצמו
:mesader-singels
color f1
cls
echo. [30m
echo       _             _                            _              _____  _____ 
echo      (_)           ^| ^|                          ^| ^|            ^|  _  ^|^|  _  ^|
echo   ___ _ _ __   __ _^| ^| ___  ___   ___  ___  _ __^| ^|_ ___ _ __  ^| ^|_^| ^|^| ^|/' ^|
echo  / __^| ^| '_ \ / _` ^| ^|/ _ \/ __^| / __^|/ _ \^| '__^| __/ _ \ '__^| \____ ^|^|  /^| ^|
echo  \__ \ ^| ^| ^| ^| (_^| ^| ^|  __/\__ \ \__ \ (_) ^| ^|  ^| ^|^|  __/ ^|    .___/ /\ ^|_/ /
echo  ^|___/_^|_^| ^|_^|\__, ^|_^|\___^|^|___/ ^|___/\___/^|_^|   \__\___^|_^|    \____(_)\___/ 
echo               ^|___/                                                          
echo ================================================================================
echo                                 %VER% םילגניסה רדסמ
echo                                       ***** [34m 
echo.
echo.
echo.                           [0] שקה יטמוטוא ןוכדעל !שדח
echo                                 -----------------
echo                               [1] שקה ליחתהל תנמ לע
echo                          [2] שקה (ןיילנואו ןיילפוא) הרזעל
echo                             [3] שקה ךלשמ םירמז תפסוהל
echo                                  [4] שקה ?שדח המ
echo.
echo                       !תויורשפאהמ תחאב הריחבל תדלקמב רפסמ שקה
choice /c 01234>nul
if errorlevel 5 (
cls
echo. [30m
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================
echo                                  ?%VER% הסרגב שדח המ
echo                                        ***** [34m
echo.
echo                        תויקיתה תריחב בלשב םיגאב ןוקית *
echo                      דבלב תישאר היקית ןוימ תורשפא תפסוה *
echo                      !ץבוקה ןמא יפל ןוימ :השדח היצקנופ *
echo                        השדחה היקצנופה לש םיגאב ינוקית *
echo                        הרזע ץבוק לש רתוי הטושפ הדרוה *
echo.                          
echo                   םיצבק ןכות לש יטמוטוא יוקינל תורשפא הפסוותה
echo                חוור יוותב ףלחוי _ וותה תא ליכמה ץבוק םש :אמגודל
echo              וקחמי המודכו "ליימ קיזוימ" ,"ליימב וציצ" ומכ תומודיק
echo.
echo                            :טידרק nh.local11@gmail.com
echo.
echo                        ישארה טירפתל הרזחל והשלכ שקמ לע ץחל
pause>nul
goto :mesader-singels
)

if errorlevel 4 goto :singer-list-new
if errorlevel 3 goto :help
if errorlevel 2 goto :beginning
if errorlevel 1 goto :updating

:updating
cls
color f1
echo. [30m
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================[34m
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/%VER%%%2Bversion
echo.
echo.                                 1 שקה תעכ ןוכדעל
echo                              2 שקה ישארה טירפתל הרזחל
echo.
echo                              -----------------------
echo.                              %VER% איה תיחכונה הסרגה
echo.
choice /c 12
if errorlevel 2 goto :mesader-singels
if errorlevel 1 (
curl -LJ https://github.com/NHLOCAL/Singles-Sorter/releases/download/v%update%/Singles-Sorter-%update%.bat -o "%~dp0\מסדר הסינגלים %update%.bat"
cls
echo.[30m
echo                                        ___
echo                                       ^|   ^|
echo                                       \   /
echo                                        \_/
echo                                        ^(_^)
echo ================================================================================
echo.[34m
echo.
echo.                           !ךלצא רבכ %update% הסרג !בוט לזמ
timeout 7 | echo               ...עגר דועב חתפתש היקיתב השדחה הסרגה תא אוצמל לכות 
explorer "%~dp0"
cls & "%~dp0\מסדר הסינגלים %update%.bat"

)

goto :mesader-singels

:help
cls
echo.[30m
echo                                הרזע - םילגניסה רדסמ
echo ================================================================================
echo.[34m
echo         .םינמא יפל תרדוסמ הרוצב םכלש םילגניסה תא רדסל איה הנכותה תרטמ
echo             :םיטושפ םידעצ השולשב תאז עצבל ןתינ ?הב םישמתשמ ךיא זא
echo                    ----------------------------------------
echo   .רטנא לע ץוחללו הנכתה ןולח ךותל הייוצרה םילגניסה תייקית תא רורגל שי .1 בלש
echo.
echo            .רטנא שיקהלו ןולחה ךותל םתרציש דעי תיקית רורגל שי .2 בלש
echo.
echo            תישיא תומאתומ תורדגהל תויורשפא רפסמ םנשי הז בלשב .3 בלש
echo         .םירפסמה ישקמ לע הציחל ידי לע תונושה תויורשפאה תא תוסנל ולכות
echo                       !האלה וכישמהו 7 וצחל טושפ ?םיכבתסמ
echo.
echo       !הצר הנכותה !והז - םתרדגהש תורדגהה תא רשאל אלא רתונ אל תעכ .4 בלש
echo                    ----------------------------------------
echo           ידיסחה רנא'זב רקיעב םירמז 350-ל לעמ הנכותב םימייק הז בלשב
echo             ישארה טירפתב 3 לע השקהב יאמצע ןפואב םירמז ףיסוהל ןתינ
echo.
echo                          !!!החמשב ץיפהלו קיתעהל ןתינ
echo                        mesader.singelim@gmail.com :ליימ
echo.
echo                              1 שקה תשרב תפסונ הרזעל
echo                           2 שקה בשחמל הרזע ץבוק תדרוהל
echo                           3 שקה ליימב הרזע תשקב תחילשל
echo                             4 שקה ישארה טירפתל הרזחל
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start https://mail.google.com/mail/u/0/?fs=1^&tf=cm^&source=mailto^&to=mesader.singelim@gmail.com & goto :mesader-singels
if errorlevel 2 (
cls
echo.
echo.
echo.
curl -LJ https://github.com/NHLOCAL/Singles-Sorter/releases/download/v8.2/help-singles-sorter.pdf -o "%userprofile%\downloads\עזרה - מסדר הסינגלים.pdf"
cls
echo.
echo.
echo                         !ךלש תודרוהה תיקיתל דרי ץבוקה
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:wrong_path
echo.
echo                      !בוש ביתנה תא סנכה אנא !םייק וניא ביתנה
timeout 2 >nul
exit /b

:beginning
cls
echo.[30m
echo                                     _  __   __
echo                                    / ^| \ \  \ \
echo                                    ^| ^|  \ \  \ \
echo                                    ^| ^|  / /  / /
echo                                    ^|_^| /_/  /_/
echo ================================================================================
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo                             רטנא + 0 ושיקה לוטיבו האיציל[34m
echo.
echo.
echo                הנכותה ןולח ךותל םילגניס שפחנש הצור ךנה הב היקית רורג
echo                       ינדי ןפואב היקית ביתנ ןזה - ןיפוליחל
echo.
echo                         !רטנא שיקהל שי ביתנה תסנכה רחאל
echo.
set/p source_path=
::בדיקה אם הוקש 0 תתבצע חזרה לתפריט הראשי
if [%source_path%] == [0] goto :mesader-singels
::הסרת מרכאות מהמשתנה
for %%i in (%source_path%) do set source_path=%%~i
::בדיקה אם מדובר בנתיב שגוי או נתיב של קובץ
if not exist "%source_path%\" call :wrong_path & goto :beginning
::בדיקה אם מדובר בנתיב ריק
if "%source_path%"=="" call :wrong_path & goto :beginning


:target_folder
cls
echo.[30m
echo                                  __    ____   __
echo                                  \ \  ^|___ \  \ \
echo                                   \ \   __) ^|  \ \
echo                                   / /  / __/   / /
echo                                  /_/  ^|_____^| /_/
echo ================================================================================
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo                             רטנא + 0 ושיקה לוטיבו האיציל[34m
echo.
echo.
echo                   םיצבקה וקתעוי הילא היקיתה תא הנכותה ןולחל רורג
echo                       ינדי ןפואב היקית ביתנ ןזה - ןיפוליחל
echo.
echo                         !רטנא שיקהל שי ביתנה תסנכה רחאל
echo.
echo               רטנא+1 ושיקה הנכותה תיקיתב יטמוטוא ןפואב היקית תריציל
echo.
set/p h=
::בדיקה אם הוקש 1 תווצר תיקית יעד באופן אוטומטי
if [%h%] == [1] md "סינגלים מסודרים" & set h="%~dp0סינגלים מסודרים"
::בדיקה אם הוקש 0 תתבצע חזרה לתפריט הראשי
if [%h%] == [0] goto :mesader-singels
::הסרת מרכאות מהמשתנה
for %%i in (%h%) do set h=%%~i
::בדיקה אם מדובר בנתיב שגוי או נתיב של קובץ
if not exist "%h%\" call :wrong_Path & goto :target_folder
::בדיקה אם מדובר בנתיב ריק
if "%h%"=="" call :wrong_path & goto :target_folder

::הודעת אזהרה אם תיקיות המקור והיעד זהות
if "%source_path%"=="%h%" (
cls
echo.
echo.
echo                     תוהז תנזהש דעיהו רוקמה תויקיתש בל ונמש
echo                                ץלמומ וניא רבדה
echo                      1 שקה תאז לכב ךישמהל ןיינועמ התא םא
echo                            2 שקה הרוחא הרזחו לוטיבל
choice /c 12
if errorlevel 2 goto :beginning
if errorlevel 1 cls
)


::קביעת התיקיה הנוכחית לתיקית המקור
cd /d "%source_path%"

::קביעת משתנים לצורך הגדרות המשתמש
set "clear_heb=ליעפ"
set cm_heb=הרבעה
set "msg=ורבעוהש"
set "abc_heb=ליעפ אל"
set c_or_m=move
set "sing_heb=ליעפ אל"
set "fixed_heb=ליעפ אל"
set "artist_heb=ליעפ"
set "dir_heb=ליעפ"
::בחירה בהגדרות שונות למשתמש
:options
cls
echo.[30m
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          הריחב תויורשפא - %VER% םילגניסה רדסמ
echo                                       *****[34m
echo.
echo.
echo                             ךילע תופדעומה תורדגהב רחב 
echo               הריחבה יונישל בוש שיקהלו ךתריחבל םאתהב רפסמ שיקהל ןתינ
echo.
::מציג הגדרות לפי המשתנים שנקבעו לעיל

echo              [%clear_heb%] רתוימ ןכותמ םיצבקה תומש יוקינל [0] שקה !שדח
echo                   ------------------------------------------
echo                    [%cm_heb%] הרבעהל הקתעה ןיב הריחבל [1] שקה 
echo              [%abc_heb%] 'ב 'אל תוקלוחמ תויקיתל הקתעהב הריחבל [2] שקה
echo         [%sing_heb%] רמז לכ ךותב "םילגניס" םשב תימינפ היקית תריציל [3] שקה 
echo             [%fixed_heb%] דבלב ךלש תועובקה םירמזה תויקיתל הקתעהל [4] שקה
echo                   ------------------------------------------
echo               [%artist_heb%] רישה יטרפב ןמאה םש יפל ןוימו הקירסל [5] שקה
echo                [%dir_heb%] הנשמ תויקית תקירס לוטיב וא תלעפהל [6] שקה
echo                        ----------------------------------
echo                               הלעפהו םויסל [7] שקה
::ממתין לבחירת המשתמש
choice /c 01234567>nul
::אם הוקש 7 סיים והתחל בסריקה
if errorlevel 8 goto :final



::אם הוקש 6 יתבצע שינוי של משתנה
::בצורת פקודת תנאי לפי המשתנה הנוכחי
if errorlevel 7 if "%dir_heb%"=="ליעפ" (
set "dir_heb=ליעפ אל"
goto :options
) else (
set dir_heb=ליעפ
goto :options
)
::אם הוקש 5 יתבצע שינוי של משתנה
::בצורת פקודת תנאי לפי המשתנה הנוכחי
if errorlevel 6 if "%artist_heb%"=="ליעפ" (
set "artist_heb=ליעפ אל"
goto :options
)else (
set artist_heb=ליעפ
goto :options
)
::אם הוקש 4 יתבצע שינוי של משתנה
::בצורת פקודת תנאי לפי המשתנה הנוכחי
if errorlevel 5 if "%fixed_heb%"=="ליעפ אל" (
set fixed_heb=ליעפ
goto :options
)else (
set "fixed_heb=ליעפ אל"
goto :options
)
::אם הוקש 3 יתבצע שינוי של משתנה
::בצורת פקודת תנאי לפי המשתנה הנוכחי
if errorlevel 4 if "%sing_heb%"=="ליעפ אל" (
set sing_heb=ליעפ
set "s=\סינגלים"
goto :options
)else (
set "sing_heb=ליעפ אל"
set s=
goto :options
)
if errorlevel 3 if "%abc_heb%"=="ליעפ אל" (
set abc_heb=ליעפ
goto :options
) else (
set "abc_heb=ליעפ אל"
goto :options
)
if errorlevel 2 if %c_or_m%==move (
set c_or_m=xcopy
set par=/y
set msg=וקתעוהש
set cm_heb=הקתעה
goto :options
) else (
set c_or_m=move
set par=
set msg=ורבעוהש
set cm_heb=הרבעה
goto :options
)
if errorlevel 1 if "%clear_heb%"=="ליעפ" (
set "clear_heb=ליעפ אל"
goto :options
) else (
set "clear_heb=ליעפ"
goto :options
)
::כאן מתבצע סיכום ההגדרות שנבחרו
:final
cls
echo.
echo                                    __   __   __
echo                                    \ \  \ \  \ \
echo                                     \ \  \ \  \ \
echo                                     / /  / /  / /
echo                                    /_/  /_/  /_/
echo ================================================================================
echo                           הריחב תויורשפא - %VER% םילגניסה רדסמ
echo                                        *****
echo.
echo.
::מציג הודעה אם נבחרה אפשרות המיון המתקדמת

if "%artist_heb%"=="ליעפ" (
echo                הקירסה םויסב ץבוקה ןמא םש יפל םדקתמ ןוימ עצבתי !שדח
)
::מציג הודעה אם אפשרות ניקוי הקבצים פעילה
if "%clear_heb%"=="ליעפ" (
echo               הקירסה לחתש ינפל םכלש םיצבקה תומש לש יוקינ עצבתי !שדח
echo                 -------------------------------------------------
)
::מציג לפי משתנה אם נבחרה העברה או העתקה
echo               םילגניסה לש ---%cm_heb%--- תעכ עצבתת ךיתורדגה יפל !בל םיש
::מציג לפי משתנה האם תתבצע סריקה גם של תיקיות משנה
::או רק של תיקיה ראשית
if "%dir_heb%"=="ליעפ" (
echo                           הנשמה תויקית לש םג הקירס עצבתת            
) else (
echo                     דבלב תישארה היקיתבש םיצבקה לש הקירס עצבתת
)

::תצוגה שכרגע לא פעילה - אולי בהמשך
::echo                                       היקיתהמ
::echo                                    "%p_finish%" 
::echo                                       היקיתה לא
::echo                                     "%h_finish%"

::מציג הגדרות שנבחרו על ידי משתנים
if "%abc_heb%" == "ליעפ" echo               'ב 'אה יפל תוישאר תויקיתב םירמזה תויקית לש הקולח עצבתת
if "%sing_heb%" == "ליעפ" echo                "םילגניס" םשב תימינפ היקית רצווית רמז הייקית לכ ךותב 
if "%fixed_heb%" == "ליעפ" echo                     דעיה תיקיתב רבכ םימייקה םירמז קר וקתעוי
echo.
if "%fixed_heb%" == "ליעפ" if "%abc_heb%" == "ליעפ" echo           !ךלש הקיזומה תיקית הנבמל תמאות 'ב 'א יפל תויקיתה תקולחש בל םיש



echo.
echo.
echo                  [2] שקה הרזחו לוטיבל [1] שקה הנכותה תצרהו רושיאל 
choice /c 12>nul
if errorlevel 2 goto :mesader-singels
if errorlevel 1 goto :intro


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
echo                         !בוש ליחתהל ליבשב והשלכ שקמ לע ץחל
pause>nul
cls
goto :mesader-singels


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



:creat-cvs
@echo off & pushd %~dp0
powershell -noprofile -c "$f=[io.file]::ReadAllText('%~f0') -split ':bat2file\:.*';iex ($f[1]);X 1;"

if not exist "%appdata%\singles-sorter" md "%appdata%\singles-sorter"
move "singer-list.csv" "%appdata%\singles-sorter\singer-list.csv"
exit /b

:bat2file: Compressed2TXT v5.3
Add-Type -Language CSharp -TypeDefinition @"
 using System.IO; public class BAT85{ public static void Decode(string tmp, string s) { MemoryStream ms=new MemoryStream(); n=0;
 byte[] b85=new byte[255]; string a85="0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$&()+,-./;=?@[]^_{|}~";
 int[] p85={52200625,614125,7225,85,1}; for(byte i=0;i<85;i++){b85[(byte)a85[i]]=i;} bool k=false;int p=0; foreach(char c in s){
 switch(c){ case'\0':case'\n':case'\r':case'\b':case'\t':case'\xA0':case' ':case':': k=false;break; default: k=true;break; }
 if(k){ n+= b85[(byte)c] * p85[p++]; if(p == 5){ ms.Write(n4b(), 0, 4); n=0; p=0; } } }         if(p>0){ for(int i=0;i<5-p;i++){
 n += 84 * p85[p+i]; } ms.Write(n4b(), 0, p-1); } File.WriteAllBytes(tmp, ms.ToArray()); ms.SetLength(0); }
 private static byte[] n4b(){ return new byte[4]{(byte)(n>>24),(byte)(n>>16),(byte)(n>>8),(byte)n}; } private static long n=0; }
"@; function X([int]$r=1){ $tmp="$r._"; echo "`n$r.."; [BAT85]::Decode($tmp, $f[$r+1]); expand -R $tmp -F:* .; del $tmp -force }

:bat2file: singer-list.csv~
::O/bZg00000Bntol00000EC2ui000000|5a50RR9100000OaK4@0RRIPYa##u00000001?r;RBmbb7]j8WpXWSX?+WgV{?+[o8u=n(;blJTY!L#0uZt$003kJG,SRy
::4Mc+cN4Ky;$_AMN{QE{SS4^^?i&Z]59hrat0RYGh-}RlbCxJDft4/-D&k(WGK;{/DY}mZBGyi{Enr$_jmO=!woP0S?9cOgiX_KpnHXE2gDTyTG-O-Y/ZsyChMS0V5
::mOHGGTZlfv.~a#nAV?fJ1OP.r1x4?LZg,@B+tELS1)eTE()FmEE0c=lgk$xeuKpZplkXy3jDY,YMjuo/j-NJ~&I_$n8LPIu[TRs&iE4G$NAZi_+;2vw2)jM;TO@Wh
::q[Rl3H?Xz^7SR#GDQxv(y&h+EH,4FG5uTjLOzQq~+e,4tx4.H[B=(aXfWM}Zc/5+fk?g5Axqal8jsQ^f/?jZbsx/.J&ZR8MurHwqpEqXW|C@oS[bZ9T8?0c1d{|4W
::^@6}izS~Sn1)-Tkf}/df6K1U!yC,orBj79286E(4AEU|Wu~Gk~@&cAc8tNq?LB!EF_6ZzYlL#5_uk[WSKe;!C^$s=ok=y^SX^+J7Q0Xg]aInl7;Fl}_)~PG,C!{bh
::.@Yv}328SKBNCi2e4G?4-AAG8)Rg{y{mReGpgUK@+UTAKz{wX0+V_EXV7d~{Re3(2BAt~)X4+x|7oC!ZdVJ8z4psS8PCJ~HXukL7#T0BojDWi0jZuu-at5jDU^](M
::nVR|IwX?BcfL2GfBB5+?U&$TDaKGv7BA$![0kg?]8/X)0BRF~vE5X#FsyHKfS(~+3(hka#D09lg6/qaqt6VMsujG8lYv{nIEF+5t/3&3OdwVVbGi}b+I3q[?M3G9a
::vU7TkR7X$,=baAIi[G^d_0b+[@u4|#1!Y?)_]80p.WXb!D^M$5|2Y8$Nqb5rq8LYpVUHs_vN-,hf_}pdl71YG_~PTPs|vwV7)qpSnpxpXx~hsmJsGKk+0);~Dr1U{
::mEJ-pwReOS&m3cRG}A_Ofhz+zB0Z,ZlR+b8aD83xX_K7MUs^rvMlexv9{oC--dZ0}oFUok3isjt9(KEl9gD(D)@w$$S0yP&-]+JXcU&9kX6|l_UP!t&IP09Dy4rKL
::7cja_6eV(dqo_5n2]BRyxG9-LE?!})Ofw}q^)ml,Xg}1m?l~WBYIzxosvgG[2(yn6!@}D&p|NJvm[cl[;MsGC0mVahQ@N=DsOe1)Wk{~A67vxCsyC0zKsuN&l.,e}
::IH+lSVEq=kAv2w?u?3{5/-b{4]ynhCi2]iU,m^1Q1XP&1L$o42#)bqwxiGt[j8KdKvBE=7[zfCjG{_uZkl4D2CxAC7d]6g3l#)5tnN/ZCcMdl[Jk87.k=/C#8BN]e
::rPF}Rvy7c~iE}[bu$n!KH8Guk/E]Q@lyy{d$oOloJ^kIr$mvOr[z30,2}NkVvlx.]L|KTjbExA,qk!)/$}i&w&QDG6x#_N2Ri[!MkNnMd,+kDf0D~dBKHZCRqgm#N
::gd(IWd0_1f&D(9-JW1X+qXDQ7#;hNOVTCI1q;!7RXeSg5eq,,;@FH8Iok^IS8F{eH^-4mpBj7vIkclC@e8-#S2uLN)&{INN[ln[wlNXqZ5)VG6c-[D(,jdsN]Lm9p
::juu^L1;YMQp1F(yG~Za;EtzGjeWTJdA]b}ZKDvib,ePa9XwW0q_@[v9oc)y{v!^V6BG0L}@_o!07YVkbYunnOvQLxtSu_s6PPW@C.XmY?{hwr[b(Ms3eh(,f?bxaR
::!{OBd+P-KWNHDUp6Xvc1e|X)DNt[ceP4&UInF#;U=S2l_DovtIFqt,OI&1~6_Hyn!IZL]nrqOu$aw_xny1ULuq_g.60g;3b7$4g/b^6v3{uqMwivA+OXplhALj6J@
::mcVyIu#e85v,G~u$PvXXS]sO=?imh2VCn|ZUxN^+Iu!B]Qq9Xo8xh[0YoRUrzW]#+4f}vIsD#T1Pc@78C@J7dXGm(d@3oOi]z+=(j5Wi23/gV_],c{OVy48R$14H]
::LtiZiqh0J7(gkT]k9GXKZwdKO1Ahvz-$]@{)ZXj)rCQE)XNjq}hpr/QE^BdZM4DZ|I.mj!aMyISYdn9u8ZNKo7PE9eQ4l}pwIw4zML(Bd)(]tKX,N{5cLc{Qy6J{P
::oowAD!o6rkeW.m,!~~,rI$lCN$b@07i?s5A?3V&/CcM1-@_F;gh,m,OmO$&#nIs13lUSV8wpFa53el&EyXCQi3xGVQP8Gm#/B-B2/~jp{$.AR~?dQobyP~(,E}/FE
::&^l,]pJsNk0n]?9Q(&&a]wL;P^)=nj.XV,mAFJubQ@+;2I1?B]nv{G2gj.gnw;_@;,@c#$rias[RaO]ku6V570+!bpfbEM=g~ZNe[H2FO$ak{6[KUgj$.+Jw+g6~L
::Pc|H}E3Sb&y5K8gX(;(^N5olp^1IRVbt1e75fnq(ZAgFZ_3$83hfpX6P4qg|Dn()nxU8VT1lIu-c1Ann5euP.ZKu~L.N@EkGt~kntdF6qEHS_|Mn[}!IdOBBj$)@)
::f?5$gGXz^.bb8LP)aCG-2iS#pHXhfXFbJZ(;W@?_P~B0}2-@ILyO5#7h_/,9qMMV4(T_1l^tD&5@#aSG8~)Seh+zoQJKz$n/5pS&EEJOhi$0DRL0c.JKD)m$=g^07
::2sl(Fx+-]!6RkHyLfjNC6L}p]0!p7Yq|b383u@;o&fE[6KuzlzW&$0tkx_S7N$LDQ.dYM3tlf+dFVcw@HQfu~p$Ma]5~P9~Sdq_/?tw}@E!q5!VPVg_yUK]O-Pjgo
::e+7fuUOxE935mB1I+cY3ceY0?t5[$^T&ApxBg[xp()$HtEOS)Nro{h=@ly929|clksA;((+_5UMj==JlBh,I$5rHLV3/6i8pxiVXIDADe0r60wM?N+iROy}JIqy7p
::bbRj5lVKq34,Y?Xv{l33yrbqo05,,VN^(nNwR.#&3r$BgcEqNT+nd;g1ODYctX2OcJ$aC7[Mv+OEoJ~bcaMyNN[HiB.C{or01?TS.kNZ#[6Y;ZCe?j+Z(Y}2_VEm&
::py@tgc!pO?9=2SU8.;hV/V=SAKR7.Mdf+,Dgmt,pbQ?4hu5f1yNmNIOWb)#+yTG5+]^V9Xppd/K-=b,WH-~v}[{Su[-J$M_v.Dv.c]AOa-Qpyd;(SLiLA,83pd1p9
::3!DAYH@Zf7pj5CD9DYBeOUCgKQXgXuLhWpk3-DsnxIQx-=j[h6jTs][dWNUs_KZIOokcaVRgJx]6xEYAs[E,CEx=UmMF[Wh;Yr!d0/DjCEt2&qIa(&s-^6R&FgbAH
::bZ@;hn49D-Tq-T]n_a.25{[paj!+39dO]Rv3760Rvod3C1CAL!.,n0^nXzljG!#PP8SNfd5$FNbpGN3QCx2zEieJYZxPIT[,9oGCpv@w9lH5e$M}HzFm/P5KSHZ4;
::jBy7T]$thji$bA62sr=M[$s)rVQjlfhOPqFIs(5D-CT;#jeUBez=hYN.?T9,@yK[BK75369)RN|R,79WNoo84/F]p26g&lg7+ywdZ+yUc=O_^Q5wS?y5pYPBD[J-8
::.j8Q3tO8N/K$R|0k0Uy0rHoHg6;HcYC(SwMt!78K)7Ab!@]wEG+obGYQ^?)YycGbT
:bat2file: end

::קרדיט: nh.local11@gmail.com


:logo_show

echo.
echo.
echo                                  אימשד אתעיסב
echo. [40;36m
echo         _______ _________ _        _______  _        _______  _______ 
echo        (  ____ \\__   __/( (    /^|(  ____ \( \      (  ____ \(  ____ \
echo        ^| (    \/   ) (   ^|  \  ( ^|^| (    \/^| (      ^| (    \/^| (    \/
echo        ^| (_____    ^| ^|   ^|   \ ^| ^|^| ^|      ^| ^|      ^| (__    ^| (_____ 
echo        (_____  )   ^| ^|   ^| (\ \) ^|^| ^| ____ ^| ^|      ^|  __)   (_____  )
echo              ) ^|   ^| ^|   ^| ^| \   ^|^| ^| \_  )^| ^|      ^| (            ) ^|
echo        /\____) ^|___) (___^| )  \  ^|^| (___) ^|^| (____/\^| (____/\/\____) ^|
echo        \_______)\_______/^|/    )_)(_______)(_______/(_______/\_______)
echo. 
echo   _______  _______  _______ _________ _______  _______     _____     _______ 
echo  (  ____ \(  ___  )(  ____ )\__   __/(  ____ \(  ____ )   / ___ \   (  __   )
echo  ^| (    \/^| (   ) ^|^| (    )^|   ) (   ^| (    \/^| (    )^|  ( (   ) )  ^| (  )  ^|
echo  ^| (_____ ^| ^|   ^| ^|^| (____)^|   ^| ^|   ^| (__    ^| (____)^|  ( (___) ^|  ^| ^| /   ^|
echo  (_____  )^| ^|   ^| ^|^|     __)   ^| ^|   ^|  __)   ^|     __)   \____  ^|  ^| (/ /) ^|
echo        ) ^|^| ^|   ^| ^|^| (\ (      ^| ^|   ^| (      ^| (\ (           ) ^|  ^|   / ^| ^|
echo  /\____) ^|^| (___) ^|^| ) \ \__   ^| ^|   ^| (____/\^| ) \ \__  /\____) )_ ^|  (__) ^|
echo  \_______)(_______)^|/   \__/   )_(   (_______/^|/   \__/  \______/(_)(_______)

timeout 3 | echo.
exit /b




