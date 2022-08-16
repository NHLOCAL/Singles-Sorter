::מטרת הסקריפט היא לסדר שירים בודדים בתיקיות לפי אמנים
::הסקריפט מיועד לציבור החרדי ובשל כך הדאטה שלו מותאמת לציבור זה

@echo off
::הגדרות של שפה, צבע, כותרת וגודל החלון
::ועוד מספר משתנים חשובים
chcp 1255>nul
title %VER% מסדר הסינגלים
MODE CON COLS=80 lines=27
color f1
set VER=7.4

::בדיקה אם גרסה חדשה זמינה להורדה
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :call-num else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)


::בדיקת מספר הזמרים הקיים כעת בסקריפט
::הדבר נצרך לצורך חישוב הזמן שעבר
::ולצורך הפונקציה של הוספת זמרים עצמאית

:call-num
del "%temp%\ver-exist-7.tmp"
goto number%ab%


:sln-start
cls
set/a abc=%ab%


goto mesader-singels

::קוד להוספת זמרים על ידי המשתמש
:singer-list-new
setlocal enabledelayedexpansion
cls
echo.
echo.
echo.
echo.
echo                             םירמז תפסוה - םילגניסה רדסמ
echo                      =========================================
echo.
echo.
echo                         ליעפ אל - [1] דחא דחא םירמז תפסוהל
echo                           [2] ץבוק ךותמ םירמז תומכ תפסוהל
echo                     [3] םכרובע חתפיש טסקט ץבוק ךותב םירמז תפסוהל 
echo                                 [4] ישארה טירפתל הרזחל
echo.
echo                                תויורשפאהמ תחאב רחב
choice /c 1234>nul
if errorlevel 4 goto mesader-singels
if errorlevel 3 goto 3.0
if errorlevel 2 goto 2.0
if errorlevel 1 goto :singer-list-new

:3.0
cls
echo.
echo                               טסקט ץבוק חתפי עגר דועב
echo                          ךל היוצרה םירמזה תמישר תא וב סנכה
echo                            רמזל רמז ןיב רטנא לש הדרפה םע
echo                           ותוא רומשו ץבוקה תא רוגס םייסתשכ
echo.
timeout 5|echo                         ץבוקב תוקיר תורוש ריאשהל ןיא !תוריהז
echo הכנס כאן שמות זמרים כרצונך ומחק את שורה זו - שימו לב שלא יוותרו שורות ריקות>"c:\users\public\list.txt"
notepad "c:\users\public\list.txt"
goto :2.3
:2.0
cls
echo.
echo.
echo.
echo.
echo                             םירמז תפסוה - םילגניסה רדסמ
echo                      =========================================
echo.
echo.
echo                            םירמזה תמישר םע ץבוקה תא ןאכל רורג
echo                     !םשל םש ןיב רטנא םע םידרפומ תויהל םיכירצ םירמזה
echo                             (רטנא+0 ושיקה לוטיבו האיציל)
set/p pa=
if %pa%==0 goto :mesader-singels
if not exist %pa% echo. & timeout 2|echo                                !בוש הסנ יוגש ביתנה & goto 2.0
copy %pa% "c:\users\public\list.txt">nul

:2.3

for /f "eol=;tokens=1,1*delims=" %%i in (c:\users\public\list.txt) do (
echo :!ab!>>%0%
echo set c=%%i>>%0%
echo set a=*%%c: =*%%*.*>>%0%
echo set/a d=1+d>>%0%
echo goto start>>%0%
set/a ab=1+ab)
del "c:\users\public\list.txt">nul
echo                                   !המלשוה הלועפה
goto :1.3
:1.0
cls
echo.
echo.
echo.
echo.
echo                             םירמז תפסוה - םילגניסה רדסמ
echo                      =========================================
echo.
echo.
echo                            רטנא וצחלו רמז לש םש ןאכ ומשר
echo                          ןיקת רבדה - ךופה עיפוי רמזה לש ומש
echo                             (רטנא+0 ושיקה לוטיבו האיציל)
goto 1.2
:1.1
cls
echo                              ...ףסונ רמז לש םש ומשר
echo                          (1 ושיקה בלש לכב הרימשו האיציל)
:1.2
set/p singer1=">>>"
if "%singer1%" == "0" goto :mesader-singels
if "%singer1%" == "1" goto 1.3
if "X%singer1%X" == "XX" goto 1.1
echo :%ab%>>%0% & echo set c=%singer1%>>%0% & echo set a=*%%c: =*%%*.*>>%0% & echo set/a d=1+d>>%0% & echo goto start>>%0% & set/a ab=1+ab
goto 1.1
:1.3
echo :%ab% >>%0%
echo set/a d=1+d >>%0%
echo find /c ":%%d%%" %%0%% >>%0%
echo if %%errorlevel%% == 0 goto %%d%% >>%0%
echo if %%errorlevel%% == 1 goto finish >>%0%
echo :number%abc%>>%0%
echo cls>>%0%
echo set/a ab=%ab%+1>>%0%
echo find /c ":number%%ab%%" %%0%% >>%0%
echo if %%errorlevel%%==0 ^(goto number%%ab%%^) else ^(goto :sln-start^)>>%0%
:exitsinglist
endlocal
echo.
timeout 5 | echo                      ...תעכ שדחמ ןעטת איה הנכותה תוניקת ךרוצל
cls & %0%


:mesader-singels
cls

::מסדר הסינגלים עצמו
:mesader2
echo. [30m
echo          ____  _             _             ____             _
echo         / ___^|(_)_ __   __ _^| ^| ___  ___  / ___^|  ___  _ __^| ^|_ ___ _ __
echo         \___ \^| ^| '_ \ / _` ^| ^|/ _ \/ __^| \___ \ / _ \^| '__^| __/ _ \ '__^|
echo          ___) ^| ^| ^| ^| ^| (_^| ^| ^|  __/\__ \  ___) ^| (_) ^| ^|  ^| ^|^|  __/ ^|
echo         ^|____/^|_^|_^| ^|_^|\__, ^|_^|\___^|^|___/ ^|____/ \___/^|_^|   \__\___^|_^|
echo                        ^|___/                     
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
echo                               םיינכדע הרזע ירושיק *
echo                           דוקב תואסרגה םושיר לועיי *
echo                            םירמזה תמישר לש לק רודיס *
echo.                          
echo                  םיצבק ןכות לש יטמוטוא יוקינל תורשפא הפסוותה
echo               חוור יוותב ףלחוי _ וותה תא ליכמה ץבוק םש :אמגודל
echo             וקחמי המודכו "ליימ קיזוימ" ,"ליימב וציצ" ומכ תומודיק
echo.
echo                            :טידרק nh.local11@gmail.com
echo.
echo                        ישארה טירפתל הרזחל והשלכ שקמ לע ץחל
pause>nul
cls
goto :mesader2
)
if errorlevel 4 goto :singer-list-new
if errorlevel 3 goto :help
if errorlevel 2 goto :begining
if errorlevel 1 goto :updating

:updating
cls
echo. [30m
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================
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
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/SinglesSorter-up.bat -o "%~dp0\מסדר הסינגלים %update%.bat"
cls
echo.
echo                                        ___
echo                                       ^|   ^|
echo                                       \   /
echo                                        \_/
echo                                        ^(_^)
echo ================================================================================
echo.
echo.
echo.                           !ךלצא רבכ %update% הסרג !בוט לזמ
timeout 7 | echo               ...עגר דועב חתפתש היקיתב השדחה הסרגה תא אוצמל לכות 
explorer "%~dp0"
cls & "%~dp0\מסדר הסינגלים %update%.bat"

)

goto :mesader-singels

:help
cls
echo.
echo.
echo.
echo                                הרזע - םילגניסה רדסמ
echo                                       *******
echo.
echo           םינמא יפל תרדוסמ הרוצב םכלש םילגניסה תא רדסל איה הנכותה תרטמ 
echo.
echo                                       :1 בלש
echo         רטנא לע ץוחללו הנכתה ןולח ךותל הייוצרה םילגניסה תייקית תא רורגל שי
echo             רטנא שיקהלו ןולחה ךותל םתרציש דעי תיקית רורגל שי ינשה בלשב
echo.
echo                  תישיא תומאתומ תורדגהל תויורשפא רפסמ םנשי הז בלשב 
echo            םירפסמה ישקמ לע הציחל ידי לע תונושה תויורשפאה תא תוסנל ולכות
echo                     םתרדגהש תורדגהה תא רשאל אלא רתונ אל תעכ
echo                                   !הצר הנכותה !והז
echo.
echo          ידיסחה רנא'זב רקיעב םירמז תואמ שולשל לעמ הנכותב םימייק הז בלשב 
echo                   הנשמ תויקית םג תקרוס הנכותה !הבושח הרעה 
echo.
echo                          ...החמשב ץיפהלו קיתעהל ןתינ    
echo                        mesader.singelim@gmail.com :ליימ
echo.
echo                              1 שקה תשרב תפסונ הרזעל
echo                           2 שקה בשחמל הרזע ץבוק תדרוהל
echo                           3 שקה ליימב הרזע תשקב תחילשל
echo                             4 שקה ישארה טירפתל הרזחל
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start mailto:mesader.singelim@gmail.com?subject=מעוניין%%20בקבלת%%20גרסאות%%20והסברים%%20נוספים%%20על%%20התוכנה^&body= & goto :mesader-singels
if errorlevel 2 (curl https://www.googleapis.com/drive/v3/files/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1?alt=media^&key=AIzaSyDduW1Zbi2MIu8aMUMF6op72pJ1f0sPBi0 -o "%userprofile%\downloads\הוראות למסדר הסינגלים.pdf"
cls
echo.
echo.
echo                         !ךלש תודרוהה תייקיתל דרי ץבוקה
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:Wrong_path
echo                      !בוש ביתנה תא סנכה אנא !םייק וניא ביתנה
timeout 2 >nul
:begining
::del %help%
cls
set a=*אבי*מילר*.*
set c=אבי מילר
set d=1

echo.
echo                                     _  __   __
echo                                    / ^| \ \  \ \
echo                                    ^| ^|  \ \  \ \
echo                                    ^| ^|  / /  / /
echo                                    ^|_^| /_/  /_/
echo ================================================================================
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo                              רטנא+0 ושיקה לוטיבו האיציל
echo.
echo.
echo                הנכותה ןולח ךותל םילגניס שפחנש הצור ךנה הב היקית רורג
echo                       ינדי ןפואב היקית ביתנ ןזה - ןיפוליחל
echo.
echo                         !רטנא שיקהל שי ביתנה תסנכה רחאל
echo.
set/p p=
if 1%p%1 == 101 goto :mesader-singels
if not exist %p% goto Wrong_Path
for %%i in (%p%) do set p_finish=%%~ni

:target_folder
cls
echo.
echo                                  __    ____   __
echo                                  \ \  ^|___ \  \ \
echo                                   \ \   __) ^|  \ \
echo                                   / /  / __/   / /
echo                                  /_/  ^|_____^| /_/
echo ================================================================================
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo                              רטנא+0 ושיקה לוטיבו האיציל
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
if 1%h%1 == 111 md "סינגלים מסודרים" & set h="%~dp0סינגלים מסודרים"
if 1%h%1 == 101 goto :mesader-singels
if not exist %h% goto :target_folder
for %%i in (%h%) do set h=%%~i
cd /d %p%

::קביעת משתנים לצורך הגדרות המשתמש
set "clear_heb=ליעפ"
set cm_heb=הרבעה
set "abc_heb=ליעפ אל"
set c_or_m=move
set "sing_heb=ליעפ אל"
set "fixed_heb=ליעפ אל"
::בחירה בהגדרות שונות למשתמש
:options
cls
echo.
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          הריחב תויורשפא - %VER% םילגניסה רדסמ
echo                                       *****
echo.
echo.
echo                             ךילע תופדעומה תורדגהב רחב 
echo               הריחבה יונישל בוש שיקהלו ךתריחבל םאתהב רפסמ שיקהל ןתינ
echo.
::מציג הגדרות לפי המשתנים שנקבעו לעיל

echo              [%clear_heb%] רתוימ ןכותמ םיצבקה תומש יוקינל [0] שקה !שדח
echo                   ------------------------------------------
echo                   [%cm_heb%] הרבעהל הקתעה ןיב הריחבל [1] שקה 
echo              [%abc_heb%] 'ב 'אל תוקלוחמ תויקיתל הקתעהב הריחבל [2] שקה
echo         [%sing_heb%] רמז לכ ךותב "םילגניס" םשב תימינפ היקית תריציל [3] שקה 
echo             [%fixed_heb%] דבלב ךלש תועובקה םירמזה תויקיתל הקתעהל [4] שקה
echo                                הלעפהו םויסל [5] שקה
::ממתין לבחירת המשתמש
choice /c 012345>nul
::אם הוקש 5 תסיים
if errorlevel 6 goto :final
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
::מציג הודעה אם אפשרות ניקוי הקבצים פעילה
if "%clear_heb%"=="ליעפ" (
echo                הקירסה לחתש ינפל םכלש םיצבקה תומש לש יוקינ עצבתי !שדח
echo                  -------------------------------------------------
)
::מציג לפי משתנה אם נבחרה העברה או העתקה
echo               םילגניסה לש ---%cm_heb%--- תעכ עצבתת ךיתורדגה יפל !בל םיש

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

::הגדרה ראשונה של משתנים - ראה בהמשך
set cm_heb=
set a=*אבי*מילר*.*
set c=אבי מילר
set d=1

:start
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
for /r %%i in ("%a%") do if exist %%i set ss=ss

if "%fixed_heb%"=="ליעפ" if not exist "%h%\%w%%c%" set ss==z

if %ss%==ss md %b%
if %c_or_m%==del set b= & set par=/q

::העתקת הסינגלים בכפוף לכמה תנאים
for /r %%i in ("%a%") do if exist %%i set xx=xx
if %xx%==xx for /r %%i in (%a%) do %c_or_m% %par% "%%i" %b%>>םוכיס

::הוראה לסקריפט לקפוץ אל המספר שהוגדר במשתנה
goto %d%

::דוגמה בקטע הראשון

::כותרת - לכאן שולחת פקודת goto
:1
::הגדרת שם קובץ לחיפוש
set a=*אביתר*בנאי*.*
::הגדרת שם תיקיה להעברה
set c=אביתר בנאי
::הגדרת המספר הבא לקפיצה
set d=2
::קופץ ללמעלה  - שם מתבצעת פעולת ההעברה
goto start
:2
set a=*אברהם*פריד*.*
set c=אברהם פריד
set d=3
goto :start
:3
set a=*אברומי*ויינברג*.*
set c=אברומי ויינברג
set d=4
goto :start
:4
set a=*אבריימי*רוט*.*
set c=אבריימי רוט
set d=5
goto :start
:5
set a=*אהרל'ה*סמט*.*
set c=אהרל'ה סמט
set d=6
goto :start
:6
set a=*אהרלה*וינטרוב*.*
set c=אהרלה וינטרוב
set d=7
goto :start
:7
set a=*אהרן**רזאל*.*
set c=אהרן רזאל
set d=8
goto :start
:8
set a=*אודי*אולמן*.*
set c=אודי אולמן
set d=9
goto :start
:9
set a=*אודי*דוידי*.*
set c=אודי דוידי
set d=10
goto :start
:10
set a=*אוהד*מושקוביץ*.*
set c=אוהד מושקוביץ
set d=11
goto :start
:11
set a=*אורי*דוידי*.*
set c=אורי דוידי
set d=12
goto :start
:12
set a=*אידיש*נחת*.*
set c=אידיש נחת
set d=13
goto :start
:13
set a=*אייזיק*האניג*.*
set c=אייזיק האניג
set d=14
goto :start
:14
set a=*איציק*אשל*.*
set c=איציק אשל
set d=15
goto :start
:15
set a=*איציק*דדיה*.*
set c=איציק דדיה
set d=16
goto :start
:16
set a=*אלי*הרצליך*.*
set c=אלי הרצליך
set d=17
goto :start
:17
set a=**אלי*מרכוס*.*
set c=אלי מרכוס
set d=19
goto :start
:19
set a=*אלי*פרידמן*.*
set c=אלי פרידמן
set d=20
goto :start
:20
set a=*אפרים*מנדלסון*.*
set c=אפרים מנדלסון
set d=21
goto :start
:21
set a=*ארי*גולדוואג*.*
set c=ארי גולדוואג
set d=22
goto :start
:22
set a=*ארי*רייך*.*
set c=ארי רייך
set d=24
goto :start
:24
set a=*ביני*לנדאו*.*
set c=ביני לנדאו
set d=25
goto :start
:25
set a=*בן*ציון*שנקר*.*
set c=בן ציון שנקר
set d=26
goto :start
:26
set a=*בני*פרידמן*.*
set c=בני פרידמן
set d=27
goto :start
:27
set a=*בנצי*שטיין*.*
set c=בנצי שטיין
set d=28
goto :start
:28
set a=*בערי*וועבר*.*
set c=בערי וועבר
set d=29
goto :start
:29
set a=*ברוך*לוין*.*
set c=ברוך לוין
set d=30
goto :start
:30
set a=*ברוך*נפתל*.*
set c=ברוך נפתל
set d=31
goto :start
:31
set a=*ברוך*שלום*.*
set c=ברוך שלום
set d=32
goto :start
:32
set a=*גד*אלבז*.*
set c=גד אלבז
set d=33
goto :start
:33
set a=*דב*הנדלר*.*
set c=דב הנדלר
set d=35
goto :start
:35
set a=*דדי*גראוכר*.*
set c=דדי גראוכר
set d=36
goto :start
:36
set a=*דוד*לואי*.*
set c=דוד לואי
set d=37
goto :start
:37
set a=*דוד*שמחה*.*
set c=דוד שמחה
set d=38
goto :start
:38
set a=*דודו*פישר*.*
set c=דודו פישר
set d=39
goto :start
:39
set a=*דודי*קאליש*.*
set c=דודי קאליש
set d=40
goto :start
:40
set a=*דודי*קנאפלער*.*
set c=דודי קנאפלער
set d=44
goto :start
:44
set a=*האסק*.*
set c=האסק
set d=46
goto :start
goto :start
:46
set a=*הלל*פלאי*.*
set c=הלל פלאי
set d=47
goto :start
:47
set a=*המזמרים*.*
set c=המזמרים
set d=48
goto :start
:48
set a=*המנגנים*.*
set c=המנגנים
set d=49
goto :start
:49
set a=*הערשי*וייס*.*
set c=הערשי וייס
set d=50
goto :start
:50
set a=*ווקאלי*.*
set c=ווקאלי
set d=51
goto :start
:51
set a=*זאנוויל*ויינברגר*.*
set c=זאנוויל ויינברגר
set d=52
goto :start
:52
set a=*זושא*.*
set c=זושא
set d=53
goto :start
:53
set a=*חיים?ישראל*.*
set c=חיים ישראל
set d=54
goto :start
:54
set a=*חיים*שלמה*מאייעס*.*
set c=חיים שלמה מאייעס
set d=55
goto :start
:55
set a=*חיליק*פראנק*.*
set c=חיליק פראנק
set d=56
goto :start
:56
set a=*חנן*בן*ארי*.*
set c=חנן בן ארי
set d=57
goto :start
:57
set a=*יהודה*גלאנץ*.*
set c=יהודה גלאנץ
set d=58
goto :start
:58
set a=*יהודה*גרין*.*
set c=יהודה גרין
set d=59
goto :start
:59
set a=*יהודה*דים*.*
set c=יהודה דים
set d=60
goto :start
:60
set a=*יואלי*גרינפלד*.*
set c=יואלי גרינפלד
set d=61
goto :start
:61
set a=*יואלי*דיקמן*.*
set c=יואלי דיקמן
set d=62
goto :start
:62
set a=*יואלי*פלקוביץ*.*
set c=יואלי פלקוביץ
set d=63
goto :start
:63
set a=*יואלי*קליין*.*
set c=יואלי קליין
set d=64
goto :start
:64
set a=*יומי*לואי*.*
set c=יומי לואי
set d=65
goto :start
:65
set a=*יונגערליך*.*
set c=יונגערליך
set d=66
goto :start
:66
set a=*יוני*זיגלבוים*.*
set c=יוני זיגלבוים
set d=67
goto :start
:67
set a=*יונתן*רזאל*.*
set c=יונתן רזאל
set d=68
goto :start
:68
set a=*יונתן*שיינפלד*.*
set c=יונתן שיינפלד
set d=69
goto :start
:69
set a=*יוסי*גרין*.*
set c=יוסי גרין
set d=70
goto :start
:70
set a=*יוסף*חיים*שוואקי*.*
set c=יוסף חיים שוואקי
set d=71
goto :start
:71
set a=*יוסף*משה*כהנא*.*
set c=יוסף משה כהנא
set d=72
goto :start
:72
set a=*יוסף*קרדונר*.*
set c=יוסף קרדונר
set d=73
goto :start
:73
set a=*יידל*.*
set c=יידל ורדיגר
set d=74
goto :start
:74
set a=*יעקב*שוואקי*.*
set c=יעקב שוואקי
set d=75
goto :start
:75
set a=*ישי*לפידות*.*
set c=ישי לפידות
set d=76
goto :start
:76
set a=*ישי*ריבו*.*
set c=ישי ריבו
set d=77
goto :start
:77
set a=*ישיבה*בויס*.*
set c=ישיבה בויס
set d=78
goto :start
:78
set a=*ישראל*אדלר*.*
set c=ישראל אדלר
set d=79
goto :start
:79
set a=*ישראל*דגן*.*
set c=ישראל דגן
set d=80
goto :start
:80
set a=*ישראל*ויליגר*.*
set c=ישראל ויליגר
set d=81
goto :start
:81
set a=*ישראל*ורדיגר*.*
set c=ישראל ורדיגר
set d=84
goto :start
:84
set a=*לוי*כהן*.*
set c=לוי כהן
set d=85
goto :start
:85
set a=*לוי*פלקוביץ*.*
set c=לוי פלקוביץ
set d=86
goto :start
:86
set a=*ליפא*שמעלצר*.*
set c=ליפא שמעלצר
set d=87
goto :start
:87
set a=*מאיר*אדלר*.*
set c=מאיר אדלר
set d=89
goto :start
:89
set a=*מוטי*שטיינמץ*.*
set c=מוטי שטיינמץ
set d=90
goto :start
:90
set a=*מונה*רוזנבלום*.*
set c=מונה רוזנבלום
set d=91
goto :start
:91
set a=*מידד*טסה*.*
set c=מידד טסה
set d=92
goto :start
:92
set a=*מיילך*קהאן*.*
set c=מיילך קהאן
set d=93
goto :start
:93
set a=*מיכאל*פרוזנסקי*.*
set c=מיכאל פרוזנסקי
set d=94
goto :start
:94
set a=*מיכאל*שטרייכר*.*
set c=מיכאל שטרייכר
set d=95
goto :start
:95
set a=*מיכה*גמרמן*.*
set c=מיכה גמרמן
set d=96
goto :start
:96
set a=*מקהלת*מלכות*.*
set c=מקהלת מלכות
set d=97
goto :start
:97
set a=*מנדי*ג'רופי*.*
set c=מנדי ג'רופי
set d=98
goto :start
:98
set a=*מנדי*וייס*.*
set c=מנדי וייס
set d=100
goto :start
:100
set a=*מבד*.*
set c=מרדכי בן דוד
set d=101
goto :start
:101
set a=*מרדכי*בן*דוד*.*
set c=מרדכי בן דוד
set d=102
goto :start
:102
set a=*מרדכי*שפירא*.*
set c=מרדכי שפירא
set d=103
goto :start
:103
set a=*משה*לאופר*.*
set c=משה לאופר
set d=104
goto :start
:104
set a=*משה*פלד*.*
set c=משה פלד
set d=105
goto :start
:105
set a=*משולם*גרינברגר*.*
set c=משולם גרינברגר
set d=106
goto :start
:106
set a=*נתנאל*מנת*.*
set c=נתנאל מנת
set d=107
goto :start
:107
set a=*נפתלי*קמפה*.*
set c=נפתלי קמפה
set d=108
goto :start
:108
set a=*עדי*רן*.*
set c=עדי רן
set d=109
goto :start
:109
set a=*עוזיה*צדוק*.*
set c=עוזיה צדוק
set d=110
goto :start
:110
set a=*עמירן*דביר*.*
set c=עמירן דביר
set d=111
goto :start
:111
set a=*עמית*ליסטוונד*.*
set c=עמית ליסטוונד
set d=112
goto :start
:112
set a=*פיני*אינהורן*.*
set c=פיני אינהורן
set d=114
goto :start
:114
set a=*פרחי*ירושלים*.*
set c=פרחי ירושלים
set d=115
goto :start
:115
set a=*פרחי*מיאמי*.*
set c=פרחי מיאמי
set d=116
goto :start
:116
set a=*צבי*זילברשטיין*.*
set c=צבי זילברשטיין
set d=117
goto :start
:117
set a=*צליל*וזמר*.*
set c=צליל וזמר
set d=118
goto :start
:118
set a=*צמאה*.*
set c=צמאה
set d=119
goto :start
:119
set a=*צמד*ילד*.*
set c=צמד ילד
set d=120
goto :start
:120
set a=*קובי*ברומר*.*
set c=קובי ברומר
set d=121
goto :start
:121
set a=*קובי*גרינבוים*.*
set c=קובי גרינבוים
set d=122
goto :start
:122
set a=*קומזינג*.*
set c=קומזינג
set d=123
goto :start
:123
set a=*קינדרלעך*.*
set c=הקינדרלעך
set d=124
goto :start
:124
set a=*רולי*דיקמן*.*
set c=רולי דיקמן
set d=125
goto :start
:125
set a=*שולי*רנד*.*
set c=שולי רנד
set d=128
goto :start
:128
set a=*שלהבת*.*
set c=שלהבת
set d=129
goto :start
:129
set a=*שלוימי*גרטנר*.*
set c=שלוימי גרטנר
set d=130
goto :start
:130
set a=*שלוימי*דסקל*.*
set c=שלוימי דסקל
set d=131
goto :start
:131
set a=*שלום*לעמער*.*
set c=שלום לעמער
set d=132
goto :start
:132
set a=*שלומי*טויסיג*.*
set c=שלוימי טויסיג
set d=134
goto :start
:134
set a=*שלמה*יהודה*רכניץ*.*
set c=שלמה יהודה רכניץ
set d=135
goto :start
:135
set a=*שלמה*כהן*.*
set c=שלמה כהן
set d=136
goto :start
:136
set a=*שלמה*קרליבך*.*
set c=שלמה קרליבך
set d=137
goto :start
:137
set a=*שלמה*שמחה*.*
set c=שלמה שמחה
set d=138
goto :start
:138
set a=*שלשלת*.*
set c=שלשלת
set d=139
goto :start
:139
set a=*שמואל*גרינמן*.*
set c=שמואל גרינמן
set d=140
goto :start
:140
set a=*שמחה*ליינר*.*
set c=שמחה ליינר
set d=141
goto :start
:141
set a=*שמחה*פרידמן*.*
set c=שמחה פרידמן
set d=142
goto :start
:142
set a=*שמחת*החיים*.*
set c=שמחת החיים
set d=143
goto :start
:143
set a=*שמילי*אונגר*.*
set c=שמילי אונגר
set d=144
goto :start
:144
set a=*שמעי*אנגל*.*
set c=שמעי אנגל
set d=145
goto :start
:145
set a=*שרגי*.*
set c=שרגי
set d=146
goto :start
:146
set a=*משה*קליין*.*
set c=משה קליין
set d=147
goto :start
:147
set a=*דוד*בן*ארזה*.*
set c=דוד בן ארזה
set d=148
goto :start
:148
set a=*אודי*דמארי*.*
set c=אודי דמארי
set d=149
goto :start
:149
set a=*אבישי*אשל*.*
set c=אבישי אשל
set d=150
goto :start
:150
set a=*אברהם*מרדכי*שוורץ*.*
set c=אברהם מרדכי שוורץ
set d=151
goto :start
:151
set a=*אבי*קראוס*.*
set c=אבי קראוס
set d=152
goto :start
:152
set a=*יונתן*שינפלד*.*
set c=יונתן שיינפלד
set d=153
goto :start
:153
set a=*שלומי*גרטנר*.*
set c=שלוימי גרטנר
set d=154
goto :start
:154
set a=*אהרון*רזאל*.*
set c=אהרן רזאל
set d=155
goto :start
:155
set a=*ארי*היל*.*
set c=ארי היל
set d=156
goto :start
:156
set a=*אברימי*רוט*.*
set c=אבריימי רוט
set d=157
goto :start
:157
set a=*איציק*ויינגרטן*.*
set c=איציק ויינגרטן
set d=158
goto :start
:158
set a=*אבי*אילסון*.*
set c=אבי אילסון
set d=159
goto :start
:159
set a=*שמואלי*אונגר*.*
set c=שמילי אונגר
set d=160
goto :start
:160
set a=*זאנוויל*וינברגר*.*
set c=זאנוויל ויינברגר
set d=161
goto :start
:161
set a=*שוקי*סלומון*.*
set c=שוקי סלומון
set d=162
goto :start
:162
set a=*דודי*קנופלר*.*
set c=דודי קנופלר
set d=163
goto :start
:163
set a=*הראל*טל*.*
set c=הראל טל
set d=164
goto :start
:164
set a=*דובי*מייזעלס*.*
set c=דובי מייזלס
set d=165
goto :start
:165
set a=*דובי*מייזלס*.*
set c=דובי מייזלס
set d=166
goto :start
:166
set a=*ניסים*בלאק*.*
set c=ניסים בלאק
set d=167
goto :start
:167
set a=*עקיבא*געלב*.*
set c=עקיבא געלב
set d=168
goto :start
:168
set a=*מוטי*וייס*.*
set c=מוטי וייס
set d=169
goto :start
:169
set a=*שלמה*טויסיג*.*
set c=שלוימי טויסיג
set d=170
goto :start
:170
set a=*אמני*ישראל*.*
set c=אמני ישראל
set d=171
goto :start
:171
set a=*נחמן*גולדברג*.*
set c=נחמן גולדברג
set d=172
goto :start
:172
set c=עמרם גרין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:173
set c=בן ציון קלצקו
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:174
set c=גבי אהרון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:175
set c=אביגדור רוט
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:176
set c=אבי בן ישראל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:177
set c=אלי סלומון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:178
set c=אלישע קלצקו
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:179
set c=בנציון ווברמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:180
set c=דוד שפירא
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:181
set c=זאבי הופמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:182
set c=יעקב שטיינרמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:183
set c=מענדל ראטה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:184
set c=ראובן יזדיאן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:185
set c=שלום ברנהולץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:186
set c=שלומי ברנשטיין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:187
set c=שלמה רגה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:188
set c=שמעון טובול
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:189
set c=אליהו לייפער
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:190
set c=איתמר שטיין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:191
set c=ארי בולזנשטיין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:192
set c=בני לאופר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:193
set c=שלומי וילמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:194
set c=ירון בר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:195
set c=גיל ישראלוב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:196
set c=דוד חזיזה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:197
set c=דוד חפצדי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:198
set c=דני פלגון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:199
set c=יוני זיגמונד
set a=*יוני*זי*.*
set/a d=1+d 
goto start 
:200
set c=חיים שלמה מייעס
set a=*חיים*שלמה*מייאס*.*
set/a d=1+d 
goto start 
:201
set c=יהודה שמעה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:202
set c=ישראל גברא
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:203
set c=נתי לוי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:204
set c=עמיר בניון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:205
set c=מוטי אילוויטש
set a=*מאטי*אילוויטש*.*
set/a d=1+d 
goto start 
:206
set c=מוטי אילוויטש
set a=*מוטי*אילוביץ*.*
set/a d=1+d 
goto start 
:207
set c=מוטי אילוויטש
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:208
set c=מאיר מסוארי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:209
set c=דן אביחי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:210
set c=שימי שפיץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:211
set c=שמואל יצחקי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:212
set c=שמואל אלהרר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:213
set c=יואל אלהרר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:214
set c=יוסף נטיב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:215
set c=שלוימי טויסיג
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:216
set c=גיל עקיביוב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:217
set c=דוידי נחשון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:218
set c=צבי גרינהיים
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:219
set c=דוד שאבי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:220
set c=איציק אורלב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:221
set c=אלעד שער
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:222
set c=אפרים מרקוביץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:223
set c=אהרל'ה סמט
set a=*ארהלה*סמט*.*
set/a d=1+d 
goto start 
:224
set c=יאיר אלייצור
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:225
set c=יאיר אלייצור
set a=*אלייצור*.*
set/a d=1+d 
goto start 
:226
set c=אהרל'ה סמט
set a=*אהרלה*סמט*.*
set/a d=1+d 
goto start 
:227
set c=בנצי קלצקין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:228
set c=אליהו חייט
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:229
set c=אריה קונסטלר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:230
set c=חיים שלמה מייעס
set a=*חיים*שלמה*מאיעס*.*
set/a d=1+d 
goto start 
:231
set c=מוטי אטיאס
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:232
set c=מוטי ויזל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:233
set c=דודי פלדמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:234
set c=מוישי שוורץ
set a=*מוישי*שווארץ*.*
set/a d=1+d 
goto start 
:235
set c=מיקי שפיצר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:236
set c=משה לוק
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:237
set c=ראובן גרבר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:238
set c=שרולי גרין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:239
set c=משה דוויק
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:240
set c=יוסף חיים ביטון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:241
set c=גרשי אורי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:242
set c=רפאל סקורי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:243
set c=אבי קלצקו
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:244
set c=אייל אביב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:245
set c=אליעד ספיר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:246
set c=גיא קהלני
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:247
set c=משה דדון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:248
set c=הרשי רוטנברג
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:249
set c=זלמן שטוב
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:250
set c=יוני שלמה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:251
set c=ישראל גרופי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:252
set c=עובדיה חממה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:253
set c=שימי שטיינמעץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:254
set c=שלומי שבת
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:255
set c=מוטי רוזנפלד
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:256
set c=אהרן קליין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:257
set c=אלעזר אסתרזון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:258
set c=יואל בריזל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:259
set c=אחיה הכהן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:260
set c=נחמן לייפר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:261
set c=מנחם אירנשטיין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:262
set c=אלי קליין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:263
set c=שמואל הררי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:264
set c=שמילי שטיינבערג
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:265
set c=מוישי פריינד
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:266
set c=יוני זיגמונד
set a=*יוניZ*.*
set/a d=1+d 
goto start 
:267
set c=יגל הרוש
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:268
set c=חיים גנץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:269
set c=אסף שפר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:270
set c=אריה קרלניסקי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:271
set c=איתן כץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:272
set c=מאיר ריבקין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:273
set c=מאיר קליינר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:274
set c=אברהם דוד ורצברגר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:275
set c=אבי לרנר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:276
set c=חיים ישכיל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:277
set c=שירת המונים
set a=*ישיבשיר*.*
set/a d=1+d 
goto start 
:278
set c=יוסי גלנץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:279
set c=מוישי ולדמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:280
set c=עומר ביטון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:281
set c=רפאל מלול
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:282
set c=שמואל הוניג
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:283
set c=מוטי אילוויטש
set a=*מאטי*אילאוויטש*.*
set/a d=1+d 
goto start 
:284
set c=נח פלאי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:285
set c=מוטי אלטמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:286
set c=מנדי גרופי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:287
set c=אריה קרלניסקי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:288
set c=מוישי פריינד
set a=*מושי*פריינד*.*
set/a d=1+d 
goto start 
:289
set c=אסף הרוש
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:290
set c=דניאל דהאן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:291
set c=מכביטס
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:292
set c=שימי לפשיץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:293
set c=עודד מנשרי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:294
set c=יהושע לימוני
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:295
set c=מוטי כהן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:296
set c=חיים נחמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:297
set c=יוחנן אורי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:298
set c=שמוליק קליין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:299
set c=אייל טוויטו
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:300
set c=איציק שוורץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:301
set c=חיים מרדכי אקשטיין
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:302
set c=נהוראי אריאלי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:303
set c=עופר שמיר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:304
set c=עקיבא שכטר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:305
set c=אריאל רייכל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:306
set c=מנחם שוקרון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:307
set c=עדי גביסון
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:308
set c=נתנאל ישראל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:309
set c=פינקי וובער
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:310
set c=ארי ברמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:311
set c=אליקים בוטה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:312
set c=דודי פרימשן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:313
set c=שלוימי קצנלבוגן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:314
set c=אהרל'ה סמט
set a=*אהרל'ע*סאמעט*.*
set/a d=1+d 
goto start 
:315
set c=שמואל יונה
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:316
set c=מיכאל שניצלר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:317
set c=מיכאל אזולאי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:318
set c=נמואל הרוש
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:319
set c=ברק כהן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:320
set c=עמנואל שולמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:321
set c=שלוימי בוקשפן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:322
set c=פייבל גרינבערג
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:323
set c=שרוליק רוזנטל
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:324
set c=שלמה כץ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:325
set c=שרוליק רייזמן
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:326
set c=יצחק מאיר
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:327
set c=נועם רמתי
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:328
set c=יחיאל ליכטיגר
set a=*%c: =*%*.*
set/a d=1+d
goto start
:329
set c=שירת המונים
set a=*קולולם*.*
set/a d=1+d
goto start
:330
set c=פרויקט קפיצת הדרך
set a=*%c: =*%*.*
set/a d=1+d
goto start
:331
set c=פרויקט קפיצת הדרך
set a=*פרויקט?קפיצת?הדרך*.*
set/a d=1+d
goto start
:332
set c=נח פלאי
set a=*%c: =*%*.*
set/a d=1+d
goto start
:333 
set/a d=1+d 
find /c ":%d%" %0% 
if %errorlevel% == 0 goto %d% 
if %errorlevel% == 1 goto finish

:finish
cls
echo.
echo.
echo                                 %VER% םילגניסה רדסמ
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !וקחמנ םיצבקה & echo. & del םוכיס & goto pause
if exist םוכיס (echo                              :%msg% םיצבקה רפסמ & find /c "1" םוכיס
) else (
echo                                   !רבד אצמנ אל
)
if not exist םוכיס set c_or_m=xxx
if exist םוכיס del םוכיס
echo.
if %c_or_m%==xcopy echo. & echo                 [2] תעכ שקה םיירוקמה םיצבקה תא קוחמל ןיינועמ התא םא & echo                         [1] שקה םתוא רומשל ןיינועמ התא םא & echo. & echo               !הקיחמב רוחבל רוסא היקית התוא םה דעיהו ביתנה םא !תוריהז & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto preparing & if errorlevel 1 goto :pause
:pause
echo.
echo                         !בוש ליחתהל ליבשב והשלכ שקמ לע ץחל
pause>nul
cls
goto :mesader-singels

:number
cls
set/a ab=333+1
find /c "number%ab%" %0%
if %errorlevel%==0 (goto :number%ab%) else (goto :sln-start)

::קרדיט: nh.local11@gmail.com
