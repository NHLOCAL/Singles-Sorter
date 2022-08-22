::מטרת הסקריפט היא לסדר שירים בודדים בתיקיות לפי אמנים
::הסקריפט מיועד לציבור החרדי ובשל כך הדאטה שלו מותאמת לציבור זה
::קרדיט: nh.local11@gmail.com

@echo oFF
::הגדרות של שפה, צבע, כותרת וגודל החלון
::ועוד מספר משתנים חשובים
chcp 1255>nul
title %VER% מסדר הסינגלים
MODE CON COLS=80 lines=27
color f1
set VER=8.0

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

::בדיקה אם גרסה חדשה זמינה להורדה
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :call-num else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)


:call-num
::בדיקת מספר הזמרים הקיים כעת בסקריפט
::הדבר נצרך לצורך חישוב הזמן שעבר
::ולצורך הפונקציה של הוספת זמרים עצמאית
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"

:sln-start
cls
set/a abc=%ab%

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
echo                                 [2] ישארה טירפתל הרזחל
echo.
echo                                תויורשפאהמ תחאב רחב
choice /c 12>nul
if errorlevel 2 goto mesader-singels
if errorlevel 1 "%csv-file%"


::מסדר הסינגלים עצמו
:mesader-singels
cls
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
goto :mesader-singels
)

if errorlevel 4 goto :singer-list-new
if errorlevel 3 goto :help
if errorlevel 2 goto :beginning
if errorlevel 1 goto :updating

:updating
cls
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
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/SinglesSorter-up.bat -o "%~dp0\מסדר הסינגלים %update%.bat"
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
echo            .םינמא יפל תרדוסמ הרוצב םכלש םילגניסה תא רדסל איה הנכותה תרטמ
echo.
echo         .רטנא לע ץוחללו הנכתה ןולח ךותל הייוצרה םילגניסה תייקית תא רורגל שי
echo             .רטנא שיקהלו ןולחה ךותל םתרציש דעי תיקית רורגל שי ינשה בלשב
echo.
echo                  תישיא תומאתומ תורדגהל תויורשפא רפסמ םנשי הז בלשב
echo            .םירפסמה ישקמ לע הציחל ידי לע תונושה תויורשפאה תא תוסנל ולכות
echo                     םתרדגהש תורדגהה תא רשאל אלא רתונ אל תעכ
echo                                !הצר הנכותה !והז
echo.
echo          .ידיסחה רנא'זב רקיעב םירמז תואמ שולשל לעמ הנכותב םימייק הז בלשב 
echo                     !הנשמ תויקית םג תקרוס הנכותה !הבושח הרעה
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
if errorlevel 3 start https://mail.google.com/mail/u/0/?fs=1^&tf=cm^&source=mailto^&to=mesader.singelim@gmail.com & goto :mesader-singels
if errorlevel 2 (curl https://www.googleapis.com/drive/v3/files/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1?alt=media^&key=AIzaSyDduW1Zbi2MIu8aMUMF6op72pJ1f0sPBi0 -o "%userprofile%\downloads\הוראות למסדר הסינגלים.pdf"
cls
echo.
echo.
echo                         !ךלש תודרוהה תייקיתל דרי ץבוקה
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
set a=*אבי*מילר*.*
set c=אבי מילר
set d=1

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

::קביעת התיקיה הנוכחית לתיקית המקור
cd /d "%source_path%"

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
echo               הקירסה לחתש ינפל םכלש םיצבקה תומש לש יוקינ עצבתי !שדח
echo                 -------------------------------------------------
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
set /a d=1

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
for /r %%c in ("%a%") do if exist %%c set ss=ss
if "%fixed_heb%"=="ליעפ" if not exist "%h%\%w%%c%" set ss==z
if %ss%==ss md %b%

::הגדרת מחיקת קבצי מקור לאחר העתקה אם הוגדר כך
if %c_or_m%==del set b= & set par=/q

::העתקת הסינגלים בכפוף לכמה תנאים
for /r %%d in ("%a%") do if exist %%d set xx=xx
if %xx%==xx for /r %%e in (%a%) do %c_or_m% %par% "%%e" %b%>>םוכיס

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

::קרדיט: nh.local11@gmail.com



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
::O/bZg00000J^.N/00000EC2ui000000|5a50RR9100000OaK4@0RRIPZ65#t00000000)M9i|_vb7]j8WpXWSX?+WgV{?+[[$ef??IrQhTY!L#0T9ym002}3Bx3-z
::4Mc+cN4Ky;$_AMN{QF_tj]xi=WSi+zG~+mQ0{}BZaA#&!@,ywrwMbnckLhuw0zG!8dBx9[s^wSGyVbC6y-zN#4xM+iphJq}Nj^}|18LN/Jnj]$?e;c1rd&q]-g!R,
::om!$.@.T$1]}hhZ001+pR5VjS@G9t#VTw=]Y.1}x_}^a;n&GH(n1VOq|NNGlI!ZE1K$)G73H?=pomIAu?nC![$)ME,DJ05S0wI4i;#@PkxbVIV+0kcS)r@kH=2$G@
::W|R((/WS[P?2dh.SpKo6r~Twyl~Oz?X#ITZk1W{Zo/sXF.!9Ye8#PS)jle7NPhB(@ARNsVyDE[8Xa$v)f=Sjk3xWdgQ84.E$Jvw{lQtzWh5#(Iv[,$)Sh6=jWTO.D
::z..YWy6S+?fmV7kd9o+t1q4NU#e.lZ=MYK#tS82Cc/y-DO^h[vDB|9u5hke(oCq5ruk=@ZesfT5;,e!]Wb!+?,Fe|Z$;o/./Ao,YM_~edCR$XZ.9.Yu#p6|vB&~d6
::tw@eLbZ|[[aPM~LN[Nc=_i.cYfq2C&t&Rn07,e[RRjtb^46rxhRh_p(NT$295{-s^0LI^kTc09g#=$4X)rur(9M0;QvYN0hq!mtCyFq|+ecm_)2hH/A8Pl57rmm~=
::,bwxxStgim_&GN@PG^/-FyhY()=eH2yg@IbID!]-Um?j+qs3go.qOiB{xEkWj(gDyt)dY|TJ4tso+J$HzJ?yP$-IF?g089(yf.QXJ!9gWinLO&J0Zl9;LtR-LDeZ3
::lzQ(dDx/&L8.Bs4v_a~Cl|jK4pnDmjzkc~z&Pn)=JihiaP|~rL3_Q!@c;PZ?zih;ZjRU^JqvSh?l?WuEsal|DAC6c}x]AoZm[+;;Xm9B1/548ztID|Ie5GZOj?w;V
::;IO_Z33e-/l&c&?aw0(e]P8Qj[o=19/(B[HPEugpBCRK]aJu,Cd-rSpUg]2l(=^6|+#;s-yzep!&b-^UF;K@f4AZx#5Bnzl&Wz.hx5S5KH(j_p^F5ZeTq?hP4h2;J
::cu54yF5ldgniM_-0=.PMidXWHYwXb/+1$W;HG|axGZ?]j4;.;3fmVihlaz}Fm}P]@;7R.[;nIO+j#,6UcS2B6YIZHfc+geyqNr7$Ubnz?L1iev]ki[l=MZ80EOSG+
::KVo)Hjd-3[rp+PHl#C|@5^#aQA-Z&/W1bnjlk1uIyvFFlN,.vWXax!u.g=U$t]lrK,1=rHs,LjjJB9D5zsOA~+$vz}0sZOBe_[h_dN@B-fMz$Ey3J1q0h8ugmr5oM
::{!qedel#]fozLLmP6#UOtnQg{-hR+(e-5NSPIHTw#Eqk1huA_ul@hAaDp[qA2!3.2t9u#pS#iJR^I&A8Q|F{C^[IwZ3no~h7|mG,5s!E$?cy_)C0H=vFYLi6.$VIb
::CyC68GywW=aBDYbR^OAe-o[$td)vPaUxuFKlml3COuSmsBlk9X|09hL1sq3uGHqrVNjY/Z0[6vef=y=C^|S{W#E)qM69VD6cv;V)z}2g]nb#rwb-UK++.aX;dfxuU
::ujoi2-dIoP[_kHtLioQPeRP{oSS$rk/h.bueZD.$C/xsrY&H|@]Nl^((8D(56HH$)wryA6K7rcL)d^c0ywygZE86z=+34C}MmD3mw/DyiUWQJR3[69op^ajKFqgAT
::4YLp7c(^JuspFrDn=QXag;^iwD@qsesG[eA&-X7rv3zkIhg13cn/hp3bZV/=jF^.-uaE;HmV4rpqHG=le6.SNAX+|]]v788uwv722}K_L(_x8vmw(Lb3g7p+6#]~H
::_o}W,aKZ^86d{Gb8b$wYNaXNAs,e)Dct.7txUbIzwBQ,d@1)k(fewImc~,h6]4O(s71/L&?SpmC)3r-Qqk3nVLCosoxa,eiz6p_B7v{R,86X,H,LHFm(mirMN}u~-
::)68/bpA.A=qaX}Nw2^DxwRT(y=wOSsI,WYkCllwwIjvBnS^YzHEkhf}Lx-?d]SL;7/EbV,2tMf{Mt}Y;{?D$dlXq3bZM=5a3b5Jjl-cEJfLkx@_Kp!+RLOyrll9dV
::fQtd_b_he)ZZ;Mb_gHZ^.n~CXfj2f!cGhCV6;=rzyw0dgCy~90#hC;SR5=;#67.45fw8F!bv$s6&i}2!SE&.Ma&BRjPaYkLi$XGvfZ}^~ANEOCyAj16EU.v?,Se$h
::Uwx}bzX@FNJfyJ{vz7n(S|gh~gZqDkO38;!!kt=6l461Am)OKW9zKJzSy_.U/bnfiaIx@eY+_}jGG?kUek1q+ct^xiKPT&7uZ#d2Nkg@w2h6VM6@d@_W&!jr/jZIm
::D_qXad-aL{JF#FS3JOEuZODe,]jlkn4naeVch@GapS{wEjL-|OAN3IIxvRntYQ}MhO4_P@9PYk@Ine@rX(=EiUgLlgq]]7VIgxjn9U^(U]sE5i.w@K.Y4,Kgl/gM)
::(o2n[ns{tK$N)wwFg)lfq13CW)c#Y-mmxcY5qtKJMl~J}yxk+obcYT{flnLIZu_}+D?]CQ{+TC!hDWIObittz81.{SCE8YL[4/;|cMd1iS]/Iska#sSDB[J7w6ll1
::F]$XtE1?@i1Zh1|Xpy^{dfc2C0o9-skJ0+jMuu/~A,=BVd26c_seYcfc$v/~uK8H^4[QPANVtlwcV$tFxRe;!wr1)SE_|PW9alVLMW2tP^]B8I_}y}X5/Axxx@/zP
::JKTGgD^GAQVVO/yBgqrSZ|acaZoPd6(PKmzH~)03A([2avL;DC9c;l$3Jd;~S}^nXgzr/T1^,s#)Ctc&Za;j|/C$;4D/hQJwYuLj,T4TR$EW-uGps]@z6;x_Th&m;
::w]tvGbQ5U-wHS+wtLR[;)8OrBSA2xf9yh-z[Q@o+t&];9)f^)VaFBTL=5K2+uJ8w!+?uHT@cRO4L?!g[W4[^(|CaA9saE2)qY96kZ[!$;VUIxpG#FPxu+V)XA?]sN
::6,u5{A6I1N.nW2)b!E9VdINEdwU=(i5gj3[$zSe~!;u3,X@}wIg-OfqFWhqp[iQ7c@Y_h9GT7SJrc~.1&ZMw|NzK){6f&$g!V3c)-EM!ou!)=o1KZwcPL)Yg&ZE?|
::67+Rm|5xUqh_TIPvl-lS4A~?}+^yW@a@|7|)O]|vzAx}[qtPX^;-ZL^Mqtzp3hv9SfvgTo2)gdB/+3N$1W1G+[sT4ciV^wX/@6NXip2iDM+&nYg_5j^+}$&IaRcyE
::bwZ]]?PQ58sW;uCpEepH{nT0b0K[
:bat2file: end
