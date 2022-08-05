::îčřú äń÷řéôč äéŕ ěńăř ůéřéí áĺăăéí áúé÷éĺú ěôé ŕîđéí
::äń÷řéôč îéĺňă ěöéáĺř äçřăé ĺáůě ëę äăŕčä ůěĺ îĺúŕîú ěöéáĺř ćä

@echo off
::äâăřĺú ůě ůôä, öáň, ëĺúřú ĺâĺăě äçěĺď
::ĺňĺă îńôř îůúđéí çůĺáéí
chcp 1255>nul
title %VER% îńăř äńéđâěéí
MODE CON: COLS=80 lines=27
color f1
set VER=7.1

::áăé÷ä ŕí âřńä çăůä ćîéđä ěäĺřăä
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :call-num else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)


::áăé÷ú îńôř äćîřéí ä÷ééí ëňú áń÷řéôč
::äăář đöřę ěöĺřę çéůĺá äćîď ůňář
::ĺěöĺřę äôĺđ÷öéä ůě äĺńôú ćîřéí ňöîŕéú

:call-num
del "%temp%\ver-exist-7.tmp"
goto number%ab%


:sln-start
cls
set/a abc=%ab%


goto mesader-singels

::÷ĺă ěäĺńôú ćîřéí ňě éăé äîůúîů
:singer-list-new
setlocal enabledelayedexpansion
cls
echo.
echo.
echo.
echo.
echo                             íéřîć úôńĺä - íéěâđéńä řăńî
echo                      =========================================
echo.
echo.
echo                         ěéňô ŕě - [1] ăçŕ ăçŕ íéřîć úôńĺäě
echo                           [2] őáĺ÷ ęĺúî íéřîć úĺîë úôńĺäě
echo                     [3] íëřĺáň çúôéů čń÷č őáĺ÷ ęĺúá íéřîć úôńĺäě 
echo                                 [4] éůŕřä čéřôúě äřćçě
echo.
echo                                úĺéĺřůôŕäî úçŕá řçá
choice /c 1234>nul
if errorlevel 4 goto mesader-singels
if errorlevel 3 goto 3.0
if errorlevel 2 goto 2.0
if errorlevel 1 goto :singer-list-new

:3.0
cls
echo.
echo                               čń÷č őáĺ÷ çúôé ňâř ăĺňá
echo                          ęě äéĺöřä íéřîćä úîéůř úŕ ĺá ńđëä
echo                            řîćě řîć ďéá řčđŕ ěů äăřôä íň
echo                           ĺúĺŕ řĺîůĺ őáĺ÷ä úŕ řĺâń íééńúůë
echo.
timeout 5|echo                         őáĺ÷á úĺ÷éř úĺřĺů řéŕůäě ďéŕ !úĺřéäć
echo äëđń ëŕď ůîĺú ćîřéí ëřöĺđę ĺîç÷ ŕú ůĺřä ćĺ - ůéîĺ ěá ůěŕ éĺĺúřĺ ůĺřĺú řé÷ĺú>"c:\users\public\list.txt"
notepad "c:\users\public\list.txt"
goto :2.3
:2.0
cls
echo.
echo.
echo.
echo.
echo                             íéřîć úôńĺä - íéěâđéńä řăńî
echo                      =========================================
echo.
echo.
echo                            íéřîćä úîéůř íň őáĺ÷ä úŕ ďŕëě řĺřâ
echo                     !íůě íů ďéá řčđŕ íň íéăřôĺî úĺéäě íéëéřö íéřîćä
echo                             (řčđŕ+0 ĺůé÷ä ěĺčéáĺ äŕéöéě)
set/p pa=
if %pa%==0 goto :mesader-singels
if not exist %pa% echo. & timeout 2|echo                                !áĺů äńđ éĺâů áéúđä & goto 2.0
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
echo                                   !äîěůĺä äěĺňôä
goto :1.3
:1.0
cls
echo.
echo.
echo.
echo.
echo                             íéřîć úôńĺä - íéěâđéńä řăńî
echo                      =========================================
echo.
echo.
echo                            řčđŕ ĺöçěĺ řîć ěů íů ďŕë ĺîůř
echo                          ďé÷ú řáăä - ęĺôä ňéôĺé řîćä ěů ĺîů
echo                             (řčđŕ+0 ĺůé÷ä ěĺčéáĺ äŕéöéě)
goto 1.2
:1.1
cls
echo                              ...óńĺđ řîć ěů íů ĺîůř
echo                          (1 ĺůé÷ä áěů ěëá äřéîůĺ äŕéöéě)
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
timeout 5 | echo                      ...úňë ůăçî ďňčú ŕéä äđëĺúä úĺđé÷ú ęřĺöě
cls & %0%


:mesader-singels
cls

::îńăř äńéđâěéí ňöîĺ
:mesader2
echo.
echo          ____  _             _             ____             _
echo         / ___^|(_)_ __   __ _^| ^| ___  ___  / ___^|  ___  _ __^| ^|_ ___ _ __
echo         \___ \^| ^| '_ \ / _` ^| ^|/ _ \/ __^| \___ \ / _ \^| '__^| __/ _ \ '__^|
echo          ___) ^| ^| ^| ^| ^| (_^| ^| ^|  __/\__ \  ___) ^| (_) ^| ^|  ^| ^|^|  __/ ^|
echo         ^|____/^|_^|_^| ^|_^|\__, ^|_^|\___^|^|___/ ^|____/ \___/^|_^|   \__\___^|_^|
echo                        ^|___/
echo ================================================================================
echo                                 %VER% íéěâđéńä řăńî
echo                                       *****
echo.
echo.
echo.                           [0] ů÷ä éčîĺčĺŕ ďĺëăňě !ůăç
echo                                 -----------------
echo                               [1] ů÷ä ěéçúäě úđî ěň
echo                          [2] ů÷ä (ďééěđĺŕĺ ďééěôĺŕ) äřćňě
echo                             [3] ů÷ä ęěůî íéřîć úôńĺäě
echo                                  [4] ů÷ä ?ůăç äî
echo.
echo                       !úĺéĺřůôŕäî úçŕá äřéçáě úăě÷îá řôńî ů÷ä
choice /c 01234>nul
if errorlevel 5 (
cls
echo.
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================
echo                                  ?%VER% äńřâá ůăç äî
echo                                        *****
echo.
echo                                âřăĺůîĺ ůăç áĺöéň .1
echo                              äěĺňô ěë éđôě řö÷ řáńä .2
echo                       ăáěá íéîéé÷ íéřîć úřáňäě úĺřůôŕ úôńĺä .3
echo                  'á-'ŕ éôě úĺé÷éúá íéřîćä ú÷ĺěçě úĺřůôŕ úôńĺä .4
echo                "íéěâđéń" íůá úéîéđô äé÷éúě íéřéůä úńđëäě úĺřůôŕ .5
echo                          íéćĺçŕá äěĺňôä úĺîă÷úä úâĺöú .6
echo                     äěňôää éđôě ęéúĺřăâä úŕ úîëńîä äňăĺä .7
echo                   éčîčĺŕ ďôĺŕá ăňé úé÷éú úřéöéě úĺřůôŕ úôńĺä .8
echo               äđëĺúä úĺŕńřâ ěë úăřĺäě úĺřůôŕĺ úđĺĺ÷î äřćň úôńĺä .9

echo.
echo                            :čéăř÷ nh.local11@gmail.com
echo.
echo                        éůŕřä čéřôúě äřćçě ĺäůěë ů÷î ěň őçě
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
echo.
echo                                        ___
echo                                       ^|__ \
echo                                         / /
echo                                        ^|_^|
echo                                        (_^)
echo ================================================================================
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/%VER%%%2Bversion
echo.
echo.                                 1 ů÷ä úňë ďĺëăňě
echo                              2 ů÷ä éůŕřä čéřôúě äřćçě
echo.
echo                              -----------------------
echo.                              %VER% ŕéä úéçëĺđä äńřâä
echo.
choice /c 12
if errorlevel 2 goto :mesader-singels
if errorlevel 1 (
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/SinglesSorter-up.bat -o "%~dp0\îńăř äńéđâěéí %update%.bat"
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
echo.                           !ęěöŕ řáë %update% äńřâ !áĺč ěćî
timeout 7 | echo               ...ňâř ăĺňá çúôúů äé÷éúá äůăçä äńřâä úŕ ŕĺöîě ěëĺú 
explorer "%~dp0"
cls & "%~dp0\îńăř äńéđâěéí %update%.bat"

)

goto :mesader-singels

:help
cls
echo.
echo.
echo.
echo                                äřćň - íéěâđéńä řăńî
echo                                       *******
echo.
echo           íéđîŕ éôě úřăĺńî äřĺöá íëěů íéěâđéńä úŕ řăńě ŕéä äđëĺúä úřčî 
echo.
echo                                       :1 áěů
echo         řčđŕ ěň őĺçěěĺ äđëúä ďĺěç ęĺúě äééĺöřä íéěâđéńä úéé÷éú úŕ řĺřâě ůé
echo                                       :2 áěů
echo             řčđŕ ůé÷äěĺ ďĺěçä ęĺúě íúřöéů ăňé úé÷éú řĺřâě ůé éđůä áěůá
echo                                       :3 áěů
echo                  úéůéŕ úĺîŕúĺî úĺřăâäě úĺéĺřůôŕ řôńî íđůé äć áěůá 
echo            íéřôńîä éů÷î ěň äöéçě éăé ěň úĺđĺůä úĺéĺřůôŕä úŕ úĺńđě ĺěëĺú
echo                     íúřăâäů úĺřăâää úŕ řůŕě ŕěŕ řúĺđ ŕě úňë
echo                                   !äöř äđëĺúä !ĺäć
echo.
echo          éăéńçä řđŕ'ćá ř÷éňá íéřîć úĺŕî ůĺěůě ěňî äđëĺúá íéîéé÷ äć áěůá 
echo                   äđůî úĺé÷éú íâ ú÷řĺń äđëĺúä !äáĺůç äřňä 
echo.
echo                          ...äçîůá őéôäěĺ ÷éúňäě ďúéđ    
echo                        mesader.singelim@gmail.com :ěééî
echo.
echo                              1 ů÷ä úůřá úôńĺđ äřćňě
echo                           2 ů÷ä áůçîě äřćň őáĺ÷ úăřĺäě
echo                           3 ů÷ä ěééîá äřćň úů÷á úçéěůě
echo                             4 ů÷ä éůŕřä čéřôúě äřćçě
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start mailto:mesader.singelim@gmail.com?subject=îňĺđééď%%20á÷áěú%%20âřńŕĺú%%20ĺäńářéí%%20đĺńôéí%%20ňě%%20äúĺëđä^&body= & goto :mesader-singels
if errorlevel 2 (curl https://www.googleapis.com/drive/v3/files/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1?alt=media^&key=AIzaSyDduW1Zbi2MIu8aMUMF6op72pJ1f0sPBi0 -o "%userprofile%\downloads\äĺřŕĺú ěîńăř äńéđâěéí.pdf"
cls
echo.
echo.
echo                         !ęěů úĺăřĺää úéé÷éúě ăřé őáĺ÷ä
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:Wrong_path
echo                      !áĺů áéúđä úŕ ńđëä ŕđŕ !íéé÷ ĺđéŕ áéúđä
timeout 2 >nul
:begining
::del %help%
cls
set a=*ŕáé*îéěř*.*
set c=ŕáé îéěř
set d=1

echo.
echo                                     _  __   __
echo                                    / ^| \ \  \ \
echo                                    ^| ^|  \ \  \ \
echo                                    ^| ^|  / /  / /
echo                                    ^|_^| /_/  /_/
echo ================================================================================
echo                                 %VER% íéěâđéńä řăńî
echo                                       *****
echo                              řčđŕ+0 ĺůé÷ä ěĺčéáĺ äŕéöéě
echo.
echo.
echo                äđëĺúä ďĺěç ęĺúě íéěâđéń ůôçđů äöĺř ęđä äá äé÷éú řĺřâ
echo                       éđăé ďôĺŕá äé÷éú áéúđ ďćä - ďéôĺěéçě
echo.
echo                         !řčđŕ ůé÷äě ůé áéúđä úńđëä řçŕě
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
echo                                 %VER% íéěâđéńä řăńî
echo                                       *****
echo                              řčđŕ+0 ĺůé÷ä ěĺčéáĺ äŕéöéě
echo.
echo.
echo                   íéöá÷ä ĺ÷úňĺé äéěŕ äé÷éúä úŕ äđëĺúä ďĺěçě řĺřâ
echo                       éđăé ďôĺŕá äé÷éú áéúđ ďćä - ďéôĺěéçě
echo.
echo                         !řčđŕ ůé÷äě ůé áéúđä úńđëä řçŕě
echo.
echo               řčđŕ+1 ĺůé÷ä äđëĺúä úé÷éúá éčîĺčĺŕ ďôĺŕá äé÷éú úřéöéě
echo.
set/p h=
if 1%h%1 == 111 md "ńéđâěéí îńĺăřéí" & set h="%~dp0ńéđâěéí îńĺăřéí"
if 1%h%1 == 101 goto :mesader-singels
if not exist %h% goto :target_folder
for %%i in (%h%) do set h=%%~i
cd /d %p%

::÷áéňú îůúđéí ěöĺřę äâăřĺú äîůúîů
set cm_heb=äřáňä
set "abc_heb=ěéňô ŕě"
set c_or_m=move
set "sing_heb=ěéňô ŕě"
set "fixed_heb=ěéňô ŕě"
::áçéřä áäâăřĺú ůĺđĺú ěîůúîů
:options
cls
echo.
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          äřéçá úĺéĺřůôŕ - %VER% íéěâđéńä řăńî
echo                                       *****
echo.
echo.
echo                             ęéěň úĺôăňĺîä úĺřăâäá řçá 
echo               äřéçáä éĺđéůě áĺů ůé÷äěĺ ęúřéçáě íŕúäá řôńî ůé÷äě ďúéđ
echo.
::îöéâ äâăřĺú ěôé äîůúđéí ůđ÷áňĺ ěňéě
echo                   [%cm_heb%] äřáňäě ä÷úňä ďéá äřéçáě [1] ů÷ä 
echo              [%abc_heb%] 'á 'ŕě úĺ÷ěĺçî úĺé÷éúě ä÷úňäá äřéçáě [2] ů÷ä
echo         [%sing_heb%] řîć ěë ęĺúá "íéěâđéń" íůá úéîéđô äé÷éú úřéöéě [3] ů÷ä 
echo             [%fixed_heb%] ăáěá ęěů úĺňĺá÷ä íéřîćä úĺé÷éúě ä÷úňäě [4] ů÷ä
echo                                äěňôäĺ íĺéńě [5] ů÷ä
::îîúéď ěáçéřú äîůúîů
choice /c 12345>nul
::ŕí äĺ÷ů 5 úńééí
if errorlevel 5 goto :final
::ŕí äĺ÷ů 4 éúáöň ůéđĺé ůě îůúđä
::áöĺřú ô÷ĺăú úđŕé ěôé äîůúđä äđĺëçé
if errorlevel 4 if "%fixed_heb%"=="ěéňô ŕě" (
set fixed_heb=ěéňô
goto :options
)else (
set "fixed_heb=ěéňô ŕě"
goto :options
)
::ŕí äĺ÷ů 3 éúáöň ůéđĺé ůě îůúđä
::áöĺřú ô÷ĺăú úđŕé ěôé äîůúđä äđĺëçé
if errorlevel 3 if "%sing_heb%"=="ěéňô ŕě" (
set sing_heb=ěéňô
set "s=\ńéđâěéí"
goto :options
)else (
set "sing_heb=ěéňô ŕě"
set s=
goto :options
)
if errorlevel 2 if "%abc_heb%"=="ěéňô ŕě" (
set abc_heb=ěéňô
goto :options
) else (
set "abc_heb=ěéňô ŕě"
goto :options
)
if errorlevel 1 if %c_or_m%==move (
set c_or_m=xcopy
set par=/y
set msg=ĺ÷úňĺäů
set cm_heb=ä÷úňä
goto :options
) else (
set c_or_m=move
set par=
set msg=ĺřáňĺäů
set cm_heb=äřáňä
goto :options
)

::ëŕď îúáöň ńéëĺí ääâăřĺú ůđáçřĺ
:final
cls
echo.
echo                                    __   __   __
echo                                    \ \  \ \  \ \
echo                                     \ \  \ \  \ \
echo                                     / /  / /  / /
echo                                    /_/  /_/  /_/
echo ================================================================================
echo                           äřéçá úĺéĺřůôŕ - %VER% íéěâđéńä řăńî
echo                                        *****
echo.
echo.
::îöéâ ěôé îůúđä ŕí đáçřä äňářä ŕĺ äňú÷ä
echo               íéěâđéńä ěů ---%cm_heb%--- úňë ňöáúú ęéúĺřăâä éôě !áě íéů

::úöĺâä ůëřâň ěŕ ôňéěä - ŕĺěé áäîůę
::echo                                       äé÷éúäî
::echo                                    "%p_finish%" 
::echo                                       äé÷éúä ěŕ
::echo                                     "%h_finish%"

::îöéâ äâăřĺú ůđáçřĺ ňě éăé îůúđéí
if "%abc_heb%" == "ěéňô" echo               'á 'ŕä éôě úĺéůŕř úĺé÷éúá íéřîćä úĺé÷éú ěů ä÷ĺěç ňöáúú
if "%sing_heb%" == "ěéňô" echo                "íéěâđéń" íůá úéîéđô äé÷éú řöĺĺéú řîć äéé÷éú ěë ęĺúá 
if "%fixed_heb%" == "ěéňô" echo                     ăňéä úé÷éúá řáë íéîéé÷ä íéřîć ÷ř ĺ÷úňĺé
echo.
if "%fixed_heb%" == "ěéňô" if "%abc_heb%" == "ěéňô" echo           !ęěů ä÷éćĺîä úé÷éú äđáîě úîŕĺú 'á 'ŕ éôě úĺé÷éúä ú÷ĺěçů áě íéů



echo.
echo.
echo                  [2] ů÷ä äřćçĺ ěĺčéáě [1] ů÷ä äđëĺúä úöřäĺ řĺůéŕě 
choice /c 12>nul
if errorlevel 2 goto :mesader-singels
if errorlevel 1 goto :begining2

:begining2
cls
echo.
echo                                    ...ăáĺň

::äâăřä řŕůĺđä ůě îůúđéí - řŕä áäîůę
set cm_heb=
set a=*ŕáé*îéěř*.*
set c=ŕáé îéěř
set d=1

:start
::äâăřú îůúđä ěäöâú îńôř ŕçĺćéí ůäĺůěîĺ
set/a en=%d%00/ab
if not "%en%"=="%enb%" cls & echo. & echo                                    ...ăáĺň & echo. & echo. & echo. & echo                               ...ĺîěůĺä íéćĺçŕ %en%
set/a enb=%d%00/ab

::÷ĺáň ŕí éĺĺöřĺ ú÷éĺú řŕůéĺú ěôé ŕ á
if "%abc_heb%"=="ěéňô" set w=%c:~0,1%\

::÷áéňú đúéá éňă ňí îńôř îůúđéí - áäúŕí ěäâăřĺú äîůúîů
set b="%h%\%w%%c%%s%"

::éöéřú úé÷éú ćîř áëôĺó ěëîä úđŕéí

set xx=v
set ss=z
for /r %%i in ("%a%") do if exist %%i set ss=ss

if "%fixed_heb%"=="ěéňô" if not exist "%h%\%w%%c%" set ss==z

if %ss%==ss md %b%
if %c_or_m%==del set b= & set par=/q

::äňú÷ú äńéđâěéí áëôĺó ěëîä úđŕéí
for /r %%i in ("%a%") do if exist %%i set xx=xx
if %xx%==xx for /r %%i in (%a%) do %c_or_m% %par% "%%i" %b%>>íĺëéń

::äĺřŕä ěń÷řéôč ě÷ôĺő ŕě äîńôř ůäĺâăř áîůúđä
goto %d%

::ăĺâîä á÷čň äřŕůĺď

::ëĺúřú - ěëŕď ůĺěçú ô÷ĺăú goto
:1
::äâăřú ůí ÷ĺáő ěçéôĺů
set a=*ŕáéúř*áđŕé*.*
::äâăřú ůí úé÷éä ěäňářä
set c=ŕáéúř áđŕé
::äâăřú äîńôř äáŕ ě÷ôéöä
set d=2
::÷ĺôő ěěîňěä  - ůí îúáöňú ôňĺěú ääňářä
goto start
:2
set a=*ŕářäí*ôřéă*.*
set c=ŕářäí ôřéă
set d=3
goto :start
:3
set a=*ŕářĺîé*ĺééđářâ*.*
set c=ŕářĺîé ĺééđářâ
set d=4
goto :start
:4
set a=*ŕářééîé*řĺč*.*
set c=ŕářééîé řĺč
set d=5
goto :start
:5
set a=*ŕäřě'ä*ńîč*.*
set c=ŕäřě'ä ńîč
set d=6
goto :start
:6
set a=*ŕäřěä*ĺéđčřĺá*.*
set c=ŕäřěä ĺéđčřĺá
set d=7
goto :start
:7
set a=*ŕäřď**řćŕě*.*
set c=ŕäřď řćŕě
set d=8
goto :start
:8
set a=*ŕĺăé*ŕĺěîď*.*
set c=ŕĺăé ŕĺěîď
set d=9
goto :start
:9
set a=*ŕĺăé*ăĺéăé*.*
set c=ŕĺăé ăĺéăé
set d=10
goto :start
:10
set a=*ŕĺäă*îĺů÷ĺáéő*.*
set c=ŕĺäă îĺů÷ĺáéő
set d=11
goto :start
:11
set a=*ŕĺřé*ăĺéăé*.*
set c=ŕĺřé ăĺéăé
set d=12
goto :start
:12
set a=*ŕéăéů*đçú*.*
set c=ŕéăéů đçú
set d=13
goto :start
:13
set a=*ŕééćé÷*äŕđéâ*.*
set c=ŕééćé÷ äŕđéâ
set d=14
goto :start
:14
set a=*ŕéöé÷*ŕůě*.*
set c=ŕéöé÷ ŕůě
set d=15
goto :start
:15
set a=*ŕéöé÷*ăăéä*.*
set c=ŕéöé÷ ăăéä
set d=16
goto :start
:16
set a=*ŕěé*äřöěéę*.*
set c=ŕěé äřöěéę
set d=17
goto :start
:17
set a=**ŕěé*îřëĺń*.*
set c=ŕěé îřëĺń
set d=19
goto :start
:19
set a=*ŕěé*ôřéăîď*.*
set c=ŕěé ôřéăîď
set d=20
goto :start
:20
set a=*ŕôřéí*îđăěńĺď*.*
set c=ŕôřéí îđăěńĺď
set d=21
goto :start
:21
set a=*ŕřé*âĺěăĺĺŕâ*.*
set c=ŕřé âĺěăĺĺŕâ
set d=22
goto :start
:22
set a=*ŕřé*řééę*.*
set c=ŕřé řééę
set d=24
goto :start
:24
set a=*áéđé*ěđăŕĺ*.*
set c=áéđé ěđăŕĺ
set d=25
goto :start
:25
set a=*áď*öéĺď*ůđ÷ř*.*
set c=áď öéĺď ůđ÷ř
set d=26
goto :start
:26
set a=*áđé*ôřéăîď*.*
set c=áđé ôřéăîď
set d=27
goto :start
:27
set a=*áđöé*ůčééď*.*
set c=áđöé ůčééď
set d=28
goto :start
:28
set a=*áňřé*ĺĺňář*.*
set c=áňřé ĺĺňář
set d=29
goto :start
:29
set a=*ářĺę*ěĺéď*.*
set c=ářĺę ěĺéď
set d=30
goto :start
:30
set a=*ářĺę*đôúě*.*
set c=ářĺę đôúě
set d=31
goto :start
:31
set a=*ářĺę*ůěĺí*.*
set c=ářĺę ůěĺí
set d=32
goto :start
:32
set a=*âă*ŕěáć*.*
set c=âă ŕěáć
set d=33
goto :start
:33
set a=*ăá*äđăěř*.*
set c=ăá äđăěř
set d=35
goto :start
:35
set a=*ăăé*âřŕĺëř*.*
set c=ăăé âřŕĺëř
set d=36
goto :start
:36
set a=*ăĺă*ěĺŕé*.*
set c=ăĺă ěĺŕé
set d=37
goto :start
:37
set a=*ăĺă*ůîçä*.*
set c=ăĺă ůîçä
set d=38
goto :start
:38
set a=*ăĺăĺ*ôéůř*.*
set c=ăĺăĺ ôéůř
set d=39
goto :start
:39
set a=*ăĺăé*÷ŕěéů*.*
set c=ăĺăé ÷ŕěéů
set d=40
goto :start
:40
set a=*ăĺăé*÷đŕôěňř*.*
set c=ăĺăé ÷đŕôěňř
set d=44
goto :start
:44
set a=*äŕń÷*.*
set c=äŕń÷
set d=46
goto :start
goto :start
:46
set a=*äěě*ôěŕé*.*
set c=äěě ôěŕé
set d=47
goto :start
:47
set a=*äîćîřéí*.*
set c=äîćîřéí
set d=48
goto :start
:48
set a=*äîđâđéí*.*
set c=äîđâđéí
set d=49
goto :start
:49
set a=*äňřůé*ĺééń*.*
set c=äňřůé ĺééń
set d=50
goto :start
:50
set a=*ĺĺ÷ŕěé*.*
set c=ĺĺ÷ŕěé
set d=51
goto :start
:51
set a=*ćŕđĺĺéě*ĺééđářâř*.*
set c=ćŕđĺĺéě ĺééđářâř
set d=52
goto :start
:52
set a=*ćĺůŕ*.*
set c=ćĺůŕ
set d=53
goto :start
:53
set a=*çééí?éůřŕě*.*
set c=çééí éůřŕě
set d=54
goto :start
:54
set a=*çééí*ůěîä*îŕééňń*.*
set c=çééí ůěîä îŕééňń
set d=55
goto :start
:55
set a=*çéěé÷*ôřŕđ÷*.*
set c=çéěé÷ ôřŕđ÷
set d=56
goto :start
:56
set a=*çđď*áď*ŕřé*.*
set c=çđď áď ŕřé
set d=57
goto :start
:57
set a=*éäĺăä*âěŕđő*.*
set c=éäĺăä âěŕđő
set d=58
goto :start
:58
set a=*éäĺăä*âřéď*.*
set c=éäĺăä âřéď
set d=59
goto :start
:59
set a=*éäĺăä*ăéí*.*
set c=éäĺăä ăéí
set d=60
goto :start
:60
set a=*éĺŕěé*âřéđôěă*.*
set c=éĺŕěé âřéđôěă
set d=61
goto :start
:61
set a=*éĺŕěé*ăé÷îď*.*
set c=éĺŕěé ăé÷îď
set d=62
goto :start
:62
set a=*éĺŕěé*ôě÷ĺáéő*.*
set c=éĺŕěé ôě÷ĺáéő
set d=63
goto :start
:63
set a=*éĺŕěé*÷ěééď*.*
set c=éĺŕěé ÷ěééď
set d=64
goto :start
:64
set a=*éĺîé*ěĺŕé*.*
set c=éĺîé ěĺŕé
set d=65
goto :start
:65
set a=*éĺđâňřěéę*.*
set c=éĺđâňřěéę
set d=66
goto :start
:66
set a=*éĺđé*ćéâěáĺéí*.*
set c=éĺđé ćéâěáĺéí
set d=67
goto :start
:67
set a=*éĺđúď*řćŕě*.*
set c=éĺđúď řćŕě
set d=68
goto :start
:68
set a=*éĺđúď*ůééđôěă*.*
set c=éĺđúď ůééđôěă
set d=69
goto :start
:69
set a=*éĺńé*âřéď*.*
set c=éĺńé âřéď
set d=70
goto :start
:70
set a=*éĺńó*çééí*ůĺĺŕ÷é*.*
set c=éĺńó çééí ůĺĺŕ÷é
set d=71
goto :start
:71
set a=*éĺńó*îůä*ëäđŕ*.*
set c=éĺńó îůä ëäđŕ
set d=72
goto :start
:72
set a=*éĺńó*÷řăĺđř*.*
set c=éĺńó ÷řăĺđř
set d=73
goto :start
:73
set a=*ééăě*.*
set c=ééăě ĺřăéâř
set d=74
goto :start
:74
set a=*éň÷á*ůĺĺŕ÷é*.*
set c=éň÷á ůĺĺŕ÷é
set d=75
goto :start
:75
set a=*éůé*ěôéăĺú*.*
set c=éůé ěôéăĺú
set d=76
goto :start
:76
set a=*éůé*řéáĺ*.*
set c=éůé řéáĺ
set d=77
goto :start
:77
set a=*éůéáä*áĺéń*.*
set c=éůéáä áĺéń
set d=78
goto :start
:78
set a=*éůřŕě*ŕăěř*.*
set c=éůřŕě ŕăěř
set d=79
goto :start
:79
set a=*éůřŕě*ăâď*.*
set c=éůřŕě ăâď
set d=80
goto :start
:80
set a=*éůřŕě*ĺéěéâř*.*
set c=éůřŕě ĺéěéâř
set d=81
goto :start
:81
set a=*éůřŕě*ĺřăéâř*.*
set c=éůřŕě ĺřăéâř
set d=84
goto :start
:84
set a=*ěĺé*ëäď*.*
set c=ěĺé ëäď
set d=85
goto :start
:85
set a=*ěĺé*ôě÷ĺáéő*.*
set c=ěĺé ôě÷ĺáéő
set d=86
goto :start
:86
set a=*ěéôŕ*ůîňěöř*.*
set c=ěéôŕ ůîňěöř
set d=87
goto :start
:87
set a=*îŕéř*ŕăěř*.*
set c=îŕéř ŕăěř
set d=89
goto :start
:89
set a=*îĺčé*ůčééđîő*.*
set c=îĺčé ůčééđîő
set d=90
goto :start
:90
set a=*îĺđä*řĺćđáěĺí*.*
set c=îĺđä řĺćđáěĺí
set d=91
goto :start
:91
set a=*îéăă*čńä*.*
set c=îéăă čńä
set d=92
goto :start
:92
set a=*îééěę*÷äŕď*.*
set c=îééěę ÷äŕď
set d=93
goto :start
:93
set a=*îéëŕě*ôřĺćđń÷é*.*
set c=îéëŕě ôřĺćđń÷é
set d=94
goto :start
:94
set a=*îéëŕě*ůčřééëř*.*
set c=îéëŕě ůčřééëř
set d=95
goto :start
:95
set a=*îéëä*âîřîď*.*
set c=îéëä âîřîď
set d=96
goto :start
:96
set a=*î÷äěú*îěëĺú*.*
set c=î÷äěú îěëĺú
set d=97
goto :start
:97
set a=*îđăé*â'řĺôé*.*
set c=îđăé â'řĺôé
set d=98
goto :start
:98
set a=*îđăé*ĺééń*.*
set c=îđăé ĺééń
set d=100
goto :start
:100
set a=*îáă*.*
set c=îřăëé áď ăĺă
set d=101
goto :start
:101
set a=*îřăëé*áď*ăĺă*.*
set c=îřăëé áď ăĺă
set d=102
goto :start
:102
set a=*îřăëé*ůôéřŕ*.*
set c=îřăëé ůôéřŕ
set d=103
goto :start
:103
set a=*îůä*ěŕĺôř*.*
set c=îůä ěŕĺôř
set d=104
goto :start
:104
set a=*îůä*ôěă*.*
set c=îůä ôěă
set d=105
goto :start
:105
set a=*îůĺěí*âřéđářâř*.*
set c=îůĺěí âřéđářâř
set d=106
goto :start
:106
set a=*đúđŕě*îđú*.*
set c=đúđŕě îđú
set d=107
goto :start
:107
set a=*đôúěé*÷îôä*.*
set c=đôúěé ÷îôä
set d=108
goto :start
:108
set a=*ňăé*řď*.*
set c=ňăé řď
set d=109
goto :start
:109
set a=*ňĺćéä*öăĺ÷*.*
set c=ňĺćéä öăĺ÷
set d=110
goto :start
:110
set a=*ňîéřď*ăáéř*.*
set c=ňîéřď ăáéř
set d=111
goto :start
:111
set a=*ňîéú*ěéńčĺĺđă*.*
set c=ňîéú ěéńčĺĺđă
set d=112
goto :start
:112
set a=*ôéđé*ŕéđäĺřď*.*
set c=ôéđé ŕéđäĺřď
set d=114
goto :start
:114
set a=*ôřçé*éřĺůěéí*.*
set c=ôřçé éřĺůěéí
set d=115
goto :start
:115
set a=*ôřçé*îéŕîé*.*
set c=ôřçé îéŕîé
set d=116
goto :start
:116
set a=*öáé*ćéěářůčééď*.*
set c=öáé ćéěářůčééď
set d=117
goto :start
:117
set a=*öěéě*ĺćîř*.*
set c=öěéě ĺćîř
set d=118
goto :start
:118
set a=*öîŕä*.*
set c=öîŕä
set d=119
goto :start
:119
set a=*öîă*éěă*.*
set c=öîă éěă
set d=120
goto :start
:120
set a=*÷ĺáé*ářĺîř*.*
set c=÷ĺáé ářĺîř
set d=121
goto :start
:121
set a=*÷ĺáé*âřéđáĺéí*.*
set c=÷ĺáé âřéđáĺéí
set d=122
goto :start
:122
set a=*÷ĺîćéđâ*.*
set c=÷ĺîćéđâ
set d=123
goto :start
:123
set a=*÷éđăřěňę*.*
set c=ä÷éđăřěňę
set d=124
goto :start
:124
set a=*řĺěé*ăé÷îď*.*
set c=řĺěé ăé÷îď
set d=125
goto :start
:125
set a=*ůĺěé*řđă*.*
set c=ůĺěé řđă
set d=128
goto :start
:128
set a=*ůěäáú*.*
set c=ůěäáú
set d=129
goto :start
:129
set a=*ůěĺéîé*âřčđř*.*
set c=ůěĺéîé âřčđř
set d=130
goto :start
:130
set a=*ůěĺéîé*ăń÷ě*.*
set c=ůěĺéîé ăń÷ě
set d=131
goto :start
:131
set a=*ůěĺí*ěňîňř*.*
set c=ůěĺí ěňîňř
set d=132
goto :start
:132
set a=*ůěĺîé*čĺéńéâ*.*
set c=ůěĺéîé čĺéńéâ
set d=134
goto :start
:134
set a=*ůěîä*éäĺăä*řëđéő*.*
set c=ůěîä éäĺăä řëđéő
set d=135
goto :start
:135
set a=*ůěîä*ëäď*.*
set c=ůěîä ëäď
set d=136
goto :start
:136
set a=*ůěîä*÷řěéáę*.*
set c=ůěîä ÷řěéáę
set d=137
goto :start
:137
set a=*ůěîä*ůîçä*.*
set c=ůěîä ůîçä
set d=138
goto :start
:138
set a=*ůěůěú*.*
set c=ůěůěú
set d=139
goto :start
:139
set a=*ůîĺŕě*âřéđîď*.*
set c=ůîĺŕě âřéđîď
set d=140
goto :start
:140
set a=*ůîçä*ěééđř*.*
set c=ůîçä ěééđř
set d=141
goto :start
:141
set a=*ůîçä*ôřéăîď*.*
set c=ůîçä ôřéăîď
set d=142
goto :start
:142
set a=*ůîçú*äçééí*.*
set c=ůîçú äçééí
set d=143
goto :start
:143
set a=*ůîéěé*ŕĺđâř*.*
set c=ůîéěé ŕĺđâř
set d=144
goto :start
:144
set a=*ůîňé*ŕđâě*.*
set c=ůîňé ŕđâě
set d=145
goto :start
:145
set a=*ůřâé*.*
set c=ůřâé
set d=146
goto :start
:146
set a=*îůä*÷ěééď*.*
set c=îůä ÷ěééď
set d=147
goto :start
:147
set a=*ăĺă*áď*ŕřćä*.*
set c=ăĺă áď ŕřćä
set d=148
goto :start
:148
set a=*ŕĺăé*ăîŕřé*.*
set c=ŕĺăé ăîŕřé
set d=149
goto :start
:149
set a=*ŕáéůé*ŕůě*.*
set c=ŕáéůé ŕůě
set d=150
goto :start
:150
set a=*ŕářäí*îřăëé*ůĺĺřő*.*
set c=ŕářäí îřăëé ůĺĺřő
set d=151
goto :start
:151
set a=*ŕáé*÷řŕĺń*.*
set c=ŕáé ÷řŕĺń
set d=152
goto :start
:152
set a=*éĺđúď*ůéđôěă*.*
set c=éĺđúď ůééđôěă
set d=153
goto :start
:153
set a=*ůěĺîé*âřčđř*.*
set c=ůěĺéîé âřčđř
set d=154
goto :start
:154
set a=*ŕäřĺď*řćŕě*.*
set c=ŕäřď řćŕě
set d=155
goto :start
:155
set a=*ŕřé*äéě*.*
set c=ŕřé äéě
set d=156
goto :start
:156
set a=*ŕářéîé*řĺč*.*
set c=ŕářééîé řĺč
set d=157
goto :start
:157
set a=*ŕéöé÷*ĺééđâřčď*.*
set c=ŕéöé÷ ĺééđâřčď
set d=158
goto :start
:158
set a=*ŕáé*ŕéěńĺď*.*
set c=ŕáé ŕéěńĺď
set d=159
goto :start
:159
set a=*ůîĺŕěé*ŕĺđâř*.*
set c=ůîéěé ŕĺđâř
set d=160
goto :start
:160
set a=*ćŕđĺĺéě*ĺéđářâř*.*
set c=ćŕđĺĺéě ĺééđářâř
set d=161
goto :start
:161
set a=*ůĺ÷é*ńěĺîĺď*.*
set c=ůĺ÷é ńěĺîĺď
set d=162
goto :start
:162
set a=*ăĺăé*÷đĺôěř*.*
set c=ăĺăé ÷đĺôěř
set d=163
goto :start
:163
set a=*äřŕě*čě*.*
set c=äřŕě čě
set d=164
goto :start
:164
set a=*ăĺáé*îééćňěń*.*
set c=ăĺáé îééćěń
set d=165
goto :start
:165
set a=*ăĺáé*îééćěń*.*
set c=ăĺáé îééćěń
set d=166
goto :start
:166
set a=*đéńéí*áěŕ÷*.*
set c=đéńéí áěŕ÷
set d=167
goto :start
:167
set a=*ň÷éáŕ*âňěá*.*
set c=ň÷éáŕ âňěá
set d=168
goto :start
:168
set a=*îĺčé*ĺééń*.*
set c=îĺčé ĺééń
set d=169
goto :start
:169
set a=*ůěîä*čĺéńéâ*.*
set c=ůěĺéîé čĺéńéâ
set d=170
goto :start
:170
set a=*ŕîđé*éůřŕě*.*
set c=ŕîđé éůřŕě
set d=171
goto :start
:171
set a=*đçîď*âĺěăářâ*.*
set c=đçîď âĺěăářâ
set d=172
goto :start
:172
set c=ňîří âřéď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:173
set c=áď öéĺď ÷ěö÷ĺ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:174
set c=âáé ŕäřĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:175
set c=ŕáéâăĺř řĺč
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:176
set c=ŕáé áď éůřŕě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:177
set c=ŕěé ńěĺîĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:178
set c=ŕěéůň ÷ěö÷ĺ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:179
set c=áđöéĺď ĺĺářîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:180
set c=ăĺă ůôéřŕ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:181
set c=ćŕáé äĺôîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:182
set c=éň÷á ůčééđřîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:183
set c=îňđăě řŕčä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:184
set c=řŕĺáď éćăéŕď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:185
set c=ůěĺí ářđäĺěő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:186
set c=ůěĺîé ářđůčééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:187
set c=ůěîä řâä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:188
set c=ůîňĺď čĺáĺě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:189
set c=ŕěéäĺ ěééôňř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:190
set c=ŕéúîř ůčééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:191
set c=ŕřé áĺěćđůčééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:192
set c=áđé ěŕĺôř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:193
set c=ůěĺîé ĺéěîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:194
set c=éřĺď ář
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:195
set c=âéě éůřŕěĺá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:196
set c=ăĺă çćéćä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:197
set c=ăĺă çôöăé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:198
set c=ăđé ôěâĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:199
set c=éĺđé ćéâîĺđă
set a=*éĺđé*ćé*.*
set/a d=1+d 
goto start 
:200
set c=çééí ůěîä îééňń
set a=*çééí*ůěîä*îééŕń*.*
set/a d=1+d 
goto start 
:201
set c=éäĺăä ůîňä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:202
set c=éůřŕě âářŕ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:203
set c=đúé ěĺé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:204
set c=ňîéř áđéĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:205
set c=îĺčé ŕéěĺĺéčů
set a=*îŕčé*ŕéěĺĺéčů*.*
set/a d=1+d 
goto start 
:206
set c=îĺčé ŕéěĺĺéčů
set a=*îĺčé*ŕéěĺáéő*.*
set/a d=1+d 
goto start 
:207
set c=îĺčé ŕéěĺĺéčů
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:208
set c=îŕéř îńĺŕřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:209
set c=ăď ŕáéçé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:210
set c=ůéîé ůôéő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:211
set c=ůîĺŕě éöç÷é
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:212
set c=ůîĺŕě ŕěäřř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:213
set c=éĺŕě ŕěäřř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:214
set c=éĺńó đčéá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:215
set c=ůěĺéîé čĺéńéâ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:216
set c=âéě ň÷éáéĺá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:217
set c=ăĺéăé đçůĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:218
set c=öáé âřéđäééí
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:219
set c=ăĺă ůŕáé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:220
set c=ŕéöé÷ ŕĺřěá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:221
set c=ŕěňă ůňř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:222
set c=ŕôřéí îř÷ĺáéő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:223
set c=ŕäřě'ä ńîč
set a=*ŕřäěä*ńîč*.*
set/a d=1+d 
goto start 
:224
set c=éŕéř ŕěééöĺř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:225
set c=éŕéř ŕěééöĺř
set a=*ŕěééöĺř*.*
set/a d=1+d 
goto start 
:226
set c=ŕäřě'ä ńîč
set a=*ŕäřěä*ńîč*.*
set/a d=1+d 
goto start 
:227
set c=áđöé ÷ěö÷éď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:228
set c=ŕěéäĺ çééč
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:229
set c=ŕřéä ÷ĺđńčěř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:230
set c=çééí ůěîä îééňń
set a=*çééí*ůěîä*îŕéňń*.*
set/a d=1+d 
goto start 
:231
set c=îĺčé ŕčéŕń
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:232
set c=îĺčé ĺéćě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:233
set c=ăĺăé ôěăîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:234
set c=îĺéůé ůĺĺřő
set a=*îĺéůé*ůĺĺŕřő*.*
set/a d=1+d 
goto start 
:235
set c=îé÷é ůôéöř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:236
set c=îůä ěĺ÷
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:237
set c=řŕĺáď âřář
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:238
set c=ůřĺěé âřéď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:239
set c=îůä ăĺĺé÷
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:240
set c=éĺńó çééí áéčĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:241
set c=âřůé ŕĺřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:242
set c=řôŕě ń÷ĺřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:243
set c=ŕáé ÷ěö÷ĺ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:244
set c=ŕééě ŕáéá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:245
set c=ŕěéňă ńôéř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:246
set c=âéŕ ÷äěđé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:247
set c=îůä ăăĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:248
set c=äřůé řĺčđářâ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:249
set c=ćěîď ůčĺá
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:250
set c=éĺđé ůěîä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:251
set c=éůřŕě âřĺôé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:252
set c=ňĺáăéä çîîä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:253
set c=ůéîé ůčééđîňő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:254
set c=ůěĺîé ůáú
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:255
set c=îĺčé řĺćđôěă
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:256
set c=ŕäřď ÷ěééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:257
set c=ŕěňćř ŕńúřćĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:258
set c=éĺŕě ářéćě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:259
set c=ŕçéä äëäď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:260
set c=đçîď ěééôř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:261
set c=îđçí ŕéřđůčééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:262
set c=ŕěé ÷ěééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:263
set c=ůîĺŕě äřřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:264
set c=ůîéěé ůčééđáňřâ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:265
set c=îĺéůé ôřééđă
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:266
set c=éĺđé ćéâîĺđă
set a=*éĺđéZ*.*
set/a d=1+d 
goto start 
:267
set c=éâě äřĺů
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:268
set c=çééí âđő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:269
set c=ŕńó ůôř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:270
set c=ŕřéä ÷řěđéń÷é
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:271
set c=ŕéúď ëő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:272
set c=îŕéř řéá÷éď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:273
set c=îŕéř ÷ěééđř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:274
set c=ŕářäí ăĺă ĺřöářâř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:275
set c=ŕáé ěřđř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:276
set c=çééí éůëéě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:277
set c=éůéáůéř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:278
set c=éĺńé âěđő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:279
set c=îĺéůé ĺěăîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:280
set c=ňĺîř áéčĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:281
set c=řôŕě îěĺě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:282
set c=ůîĺŕě äĺđéâ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:283
set c=îĺčé ŕéěĺĺéčů
set a=*îŕčé*ŕéěŕĺĺéčů*.*
set/a d=1+d 
goto start 
:284
set c=đç ôěŕé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:285
set c=îĺčé ŕěčîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:286
set c=îđăé âřĺôé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:287
set c=ŕřéä ÷řěđéń÷é
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:288
set c=îĺéůé ôřééđă
set a=*îĺůé*ôřééđă*.*
set/a d=1+d 
goto start 
:289
set c=ŕńó äřĺů
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:290
set c=ăđéŕě ăäŕď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:291
set c=îëáéčń
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:292
set c=ůéîé ěôůéő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:293
set c=ňĺăă îđůřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:294
set c=éäĺůň ěéîĺđé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:295
set c=îĺčé ëäď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:296
set c=çééí đçîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:297
set c=éĺçđď ŕĺřé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:298
set c=ůîĺěé÷ ÷ěééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:299
set c=ŕééě čĺĺéčĺ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:300
set c=ŕéöé÷ ůĺĺřő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:301
set c=çééí îřăëé ŕ÷ůčééď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:302
set c=đäĺřŕé ŕřéŕěé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:303
set c=ňĺôř ůîéř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:304
set c=ň÷éáŕ ůëčř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:305
set c=ŕřéŕě řééëě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:306
set c=îđçí ůĺ÷řĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:307
set c=ňăé âáéńĺď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:308
set c=đúđŕě éůřŕě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:309
set c=ôéđ÷é ĺĺáňř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:310
set c=ŕřé ářîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:311
set c=ŕěé÷éí áĺčä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:312
set c=ăĺăé ôřéîůď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:313
set c=ůěĺéîé ÷öđěáĺâď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:314
set c=ŕäřě'ä ńîč
set a=*ŕäřě'ň*ńŕîňč*.*
set/a d=1+d 
goto start 
:315
set c=ůîĺŕě éĺđä
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:316
set c=îéëŕě ůđéöěř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:317
set c=îéëŕě ŕćĺěŕé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:318
set c=đîĺŕě äřĺů
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:319
set c=ář÷ ëäď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:320
set c=ňîđĺŕě ůĺěîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:321
set c=ůěĺéîé áĺ÷ůôď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:322
set c=ôééáě âřéđáňřâ
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:323
set c=ůřĺěé÷ řĺćđčě
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:324
set c=ůěîä ëő
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:325
set c=ůřĺěé÷ řééćîď
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:326
set c=éöç÷ îŕéř
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:327
set c=đĺňí řîúé
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:328
set c=éçéŕě ěéëčéâř
set a=*%c: =*%*.*
set/a d=1+d
goto start
:329
set/a d=1+d
find /c ":%d%" %0%
if %errorlevel%==0 goto %d%
if %errorlevel%==1 goto finish

:finish
cls
echo.
echo.
echo                                 %VER% íéěâđéńä řăńî
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !ĺ÷çîđ íéöá÷ä & echo. & del íĺëéń & goto pause
if exist íĺëéń (echo                              :%msg% íéöá÷ä řôńî & find /c "1" íĺëéń
) else (
echo                                   !řáă ŕöîđ ŕě
)
if not exist íĺëéń set c_or_m=xxx
if exist íĺëéń del íĺëéń
echo.
if %c_or_m%==xcopy echo. & echo                 [2] úňë ů÷ä íééřĺ÷îä íéöá÷ä úŕ ÷ĺçîě ďééđĺňî äúŕ íŕ & echo                         [1] ů÷ä íúĺŕ řĺîůě ďééđĺňî äúŕ íŕ & echo. & echo               !ä÷éçîá řĺçáě řĺńŕ äé÷éú äúĺŕ íä ăňéäĺ áéúđä íŕ !úĺřéäć & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto begining2 & if errorlevel 1 goto :pause
:pause
echo.
echo                         !áĺů ěéçúäě ěéáůá ĺäůěë ů÷î ěň őçě
pause>nul
cls
goto :mesader-singels

:number
cls
set/a ab=329+1
find /c "number%ab%" %0%
if %errorlevel%==0 (goto :number%ab%) else (goto :sln-start)
::÷řăéč: nh.local11@gmail.com







:330
set c=ôřĺé÷č ÷ôéöú äăřę
set a=*%c: =*%*.*
set/a d=1+d
goto start
:331
set c=ôřĺéé÷č ÷ôéöú äăřę
set a=*%c: =*%*.*
set/a d=1+d
goto start
:332
set c=đç ôěŕé
set a=*%c: =*%*.*
set/a d=1+d
goto start
:333 
set/a d=1+d 
find /c ":%d%" %0% 
if %errorlevel% == 0 goto %d% 
if %errorlevel% == 1 goto finish 
:number330
cls
set/a ab=333+1
find /c ":number%ab%" %0% 
if %errorlevel%==0 (goto number%ab%) else (goto :sln-start)
