::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
set "VER=8.9.4"
title %VER% ���� ��������
MODE CON COLS=80 lines=27
color f1


::����� �� ���� ���� ���� �-������� �� ������ �������
::������ ����� ������ ���� �����
::����� ���� ���� ����� ����� ���� �� ����
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
::����� ���� ������ ����� ��� �������
::���� ���� ����� ����� ���� ����
::������ �������� �� ����� ����� ������
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"
set/a abc=%ab%

::����� ����� ������ ����� ������ ���� 2
::����� ������� ��� ����� �� ��� �������
if not [%1]==[] (
set "source_path=%1"
call :drag_func
)

goto :new_ver

:drag_func
for %%i in (%source_path%) do set source_path=%%~i
if exist "%source_path%\" goto :target_folder
exit /b

:new_ver
::����� �� ���� ���� ����� ������
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :mesader-singels else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)

goto :mesader-singels

::��� ������ ����� �� ��� ������
:singer-list-new
cls
echo.
echo.
echo.
echo.[30m 
echo                             ����� ����� - �������� ����
echo                      ========================================= [34m 
echo.
echo.
echo                           [1] ���� ���� ����� ���� ������
echo                        [2] ������ ������ ������ ����� ������
echo                               [3] ����� ������ �����
echo.
echo                                 ���������� ���� ���
choice /c 123>nul
if errorlevel 3 goto :mesader-singels
if errorlevel 2 (@copy "%csv-file%" "%~dp0"
echo.
timeout 2 | echo                                    !����� �����
goto :mesader-singels
)
if errorlevel 1 "%csv-file%" & call :call-num


::���� �������� ����
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
echo                                 %VER% �������� ����
echo                                       ***** [34m 
echo.
echo.
echo.                           [0] ��� ������� ������ !���
echo                                 -----------------
echo                               [1] ��� ������ ��� ��
echo                          [2] ��� (�������� �������) �����
echo                             [3] ��� ���� ����� ������
echo                                  [4] ��� ?��� ��
echo.
echo                       !���������� ���� ������ ������ ���� ���
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
echo                                  ?%VER% ����� ��� ��
echo                                        ***** [34m
echo.
echo                        ������� ����� ���� ����� ����� *
echo                      ���� ����� ����� ���� ������ ����� *
echo                     !����� ��� �� ��� ����� ���� :����� *
echo                         ����� �� ����� ������ ����� *
echo                                   ...���� *
echo.                          
echo                   ����� ���� �� ������� ������ ������ �������
echo                ���� ����� ����� _ ���� �� ����� ���� �� :������
echo              ����� ������ "���� ������" ,"����� ����" ��� �������
echo.
echo                            :����� nh.local11@gmail.com
echo.
echo                        ����� ������ ����� ����� ��� �� ���
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
echo.                                 1 ��� ��� ������
echo                              2 ��� ����� ������ �����
echo.
echo                              -----------------------
echo.                              %VER% ��� ������� �����
echo.
choice /c 12
if errorlevel 2 goto :mesader-singels
if errorlevel 1 (
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/SinglesSorter-up.bat -o "%~dp0\���� �������� %update%.bat"
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
echo.                           !���� ��� %update% ���� !��� ���
timeout 7 | echo               ...��� ���� ����� ������ ����� ����� �� ����� ���� 
explorer "%~dp0"
cls & "%~dp0\���� �������� %update%.bat"

)

goto :mesader-singels

:help
cls
echo.[30m
echo                                ���� - �������� ����
echo ================================================================================
echo.[34m
echo         .����� ��� ������ ����� ���� �������� �� ���� ��� ������ ����
echo             :������ ����� ������ ��� ���� ���� ?�� ������� ��� ��
echo                    ----------------------------------------
echo   .���� �� ������ ����� ���� ���� ������� �������� ������ �� ����� �� .1 ���
echo.
echo            .���� ������ ����� ���� ������ ��� ����� ����� �� .2 ���
echo.
echo            ����� ������� ������� �������� ���� ���� �� ���� .3 ���
echo         .������� ���� �� ����� ��� �� ������ ��������� �� ����� �����
echo                       !���� ������� 7 ���� ���� ?�������
echo.
echo       !��� ������ !��� - ������� ������� �� ���� ��� ���� �� ��� .4 ���
echo                    ----------------------------------------
echo           ������ ���'�� ����� ����� 350-� ��� ������ ������ �� ����
echo             ����� ������ 3 �� ����� ����� ����� ����� ������ ����
echo.
echo                          !!!����� ������ ������ ����
echo                        mesader.singelim@gmail.com :����
echo.
echo                              1 ��� ���� ����� �����
echo                           2 ��� ����� ���� ���� ������
echo                           3 ��� ����� ���� ���� ������
echo                             4 ��� ����� ������ �����
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start https://mail.google.com/mail/u/0/?fs=1^&tf=cm^&source=mailto^&to=mesader.singelim@gmail.com & goto :mesader-singels
if errorlevel 2 (start https://github.com/NHLOCAL/Singles-Sorter/releases/download/v8.2/help-singles-sorter.pdf
cls
echo.
echo.
echo                         !��� ������� ������� ��� �����
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:wrong_path
echo.
echo                      !��� ����� �� ���� ��� !���� ���� �����
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
echo                                 %VER% �������� ����
echo                                       *****
echo                             ���� + 0 ����� ������ ������[34m
echo.
echo.
echo                ������ ���� ���� ������� ����� ���� ��� �� ����� ����
echo                       ���� ����� ����� ���� ��� - ��������
echo.
echo                         !���� ����� �� ����� ����� ����
echo.
set/p source_path=
::����� �� ���� 0 ����� ���� ������ �����
if [%source_path%] == [0] goto :mesader-singels
::���� ������ �������
for %%i in (%source_path%) do set source_path=%%~i
::����� �� ����� ����� ���� �� ���� �� ����
if not exist "%source_path%\" call :wrong_path & goto :beginning
::����� �� ����� ����� ���
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
echo                                 %VER% �������� ����
echo                                       *****
echo                             ���� + 0 ����� ������ ������[34m
echo.
echo.
echo                   ������ ������ ���� ������ �� ������ ����� ����
echo                       ���� ����� ����� ���� ��� - ��������
echo.
echo                         !���� ����� �� ����� ����� ����
echo.
echo               ����+1 ����� ������ ������ ������� ����� ����� ������
echo.
set/p h=
::����� �� ���� 1 ����� ����� ��� ����� �������
if [%h%] == [1] md "������� �������" & set h="%~dp0������� �������"
::����� �� ���� 0 ����� ���� ������ �����
if [%h%] == [0] goto :mesader-singels
::���� ������ �������
for %%i in (%h%) do set h=%%~i
::����� �� ����� ����� ���� �� ���� �� ����
if not exist "%h%\" call :wrong_Path & goto :target_folder
::����� �� ����� ����� ���
if "%h%"=="" call :wrong_path & goto :target_folder

::����� ����� �� ������ ����� ����� ����
if "%source_path%"=="%h%" (
cls
echo.
echo.
echo                     ���� ����� ����� ����� ������� �� ����
echo                                ����� ���� ����
echo                      1 ��� ��� ��� ������ ������� ��� ��
echo                            2 ��� ����� ����� ������
choice /c 12
if errorlevel 2 goto :beginning
if errorlevel 1 cls
)


::����� ������ ������� ������ �����
cd /d "%source_path%"

::����� ������ ����� ������ ������
set "clear_heb=����"
set cm_heb=�����
set "msg=�������"
set "abc_heb=���� ��"
set c_or_m=move
set "sing_heb=���� ��"
set "fixed_heb=���� ��"
set "artist_heb=���� ��"
set "dir_heb=����"
::����� ������� ����� ������
:options
cls
echo.[30m
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          ����� �������� - %VER% �������� ����
echo                                       *****[34m
echo.
echo.
echo                             ���� �������� ������� ��� 
echo               ������ ������ ��� ������ ������� ����� ���� ����� ����
echo.
::���� ������ ��� ������� ������ ����

echo              [%clear_heb%] ����� ����� ������ ���� ������ [0] ��� !���
echo                   ------------------------------------------
echo                    [%cm_heb%] ������ ����� ��� ������ [1] ��� 
echo              [%abc_heb%] '� '�� ������� ������� ������ ������ [2] ���
echo         [%sing_heb%] ��� �� ���� "�������" ��� ������ ����� ������ [3] ��� 
echo             [%fixed_heb%] ���� ��� ������� ������ ������� ������ [4] ���
echo                   ------------------------------------------
echo              [%artist_heb%] ���� ����� ���� �� ��� ����� ������ [5] ���
echo                [%dir_heb%] ���� ������ ����� ����� �� ������ [6] ���
echo                        ----------------------------------
echo                               ������ ����� [7] ���
::����� ������ ������
choice /c 01234567>nul
::�� ���� 7 ���� ����� ������
if errorlevel 8 goto :final



::�� ���� 6 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 7 if "%dir_heb%"=="����" (
set "dir_heb=���� ��"
goto :options
) else (
set dir_heb=����
goto :options
)
::�� ���� 5 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 6 if "%artist_heb%"=="���� ��" (
set artist_heb=����
goto :options
)else (
set "artist_heb=���� ��"
goto :options
)
::�� ���� 4 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 5 if "%fixed_heb%"=="���� ��" (
set fixed_heb=����
goto :options
)else (
set "fixed_heb=���� ��"
goto :options
)
::�� ���� 3 ����� ����� �� �����
::����� ����� ���� ��� ������ ������
if errorlevel 4 if "%sing_heb%"=="���� ��" (
set sing_heb=����
set "s=\�������"
goto :options
)else (
set "sing_heb=���� ��"
set s=
goto :options
)
if errorlevel 3 if "%abc_heb%"=="���� ��" (
set abc_heb=����
goto :options
) else (
set "abc_heb=���� ��"
goto :options
)
if errorlevel 2 if %c_or_m%==move (
set c_or_m=xcopy
set par=/y
set msg=�������
set cm_heb=�����
goto :options
) else (
set c_or_m=move
set par=
set msg=�������
set cm_heb=�����
goto :options
)
if errorlevel 1 if "%clear_heb%"=="����" (
set "clear_heb=���� ��"
goto :options
) else (
set "clear_heb=����"
goto :options
)
::��� ����� ����� ������� ������
:final
cls
echo.
echo                                    __   __   __
echo                                    \ \  \ \  \ \
echo                                     \ \  \ \  \ \
echo                                     / /  / /  / /
echo                                    /_/  /_/  /_/
echo ================================================================================
echo                           ����� �������� - %VER% �������� ����
echo                                        *****
echo.
echo.
::���� ����� �� ����� ������ ����� �������

if "%artist_heb%"=="����" (
echo                ������ ����� ����� ��� �� ��� ����� ���� ����� !���
)
::���� ����� �� ������ ����� ������ �����
if "%clear_heb%"=="����" (
echo               ������ ���� ���� ���� ������ ���� �� ����� ����� !���
echo                 -------------------------------------------------
)
::���� ��� ����� �� ����� ����� �� �����
echo               �������� �� ---%cm_heb%--- ��� ����� �������� ��� !�� ���
::���� ��� ����� ��� ����� ����� �� �� ������ ����
::�� �� �� ����� �����
if "%dir_heb%"=="����" (
echo                           ����� ������ �� �� ����� �����            
) else (
echo                     ���� ������ ������� ������ �� ����� �����
)

::����� ����� �� ����� - ���� �����
::echo                                       �������
::echo                                    "%p_finish%" 
::echo                                       ������ ��
::echo                                     "%h_finish%"

::���� ������ ������ �� ��� ������
if "%abc_heb%" == "����" echo               '� '�� ��� ������ ������� ������ ������ �� ����� �����
if "%sing_heb%" == "����" echo                "�������" ��� ������ ����� ������ ��� ������ �� ���� 
if "%fixed_heb%" == "����" echo                     ���� ������ ��� ������� ����� �� ������
echo.
if "%fixed_heb%" == "����" if "%abc_heb%" == "����" echo           !��� ������� ����� ����� ����� '� '� ��� ������� ������ �� ���



echo.
echo.
echo                  [2] ��� ����� ������ [1] ��� ������ ����� ������ 
choice /c 12>nul
if errorlevel 2 goto :mesader-singels
if errorlevel 1 goto :intro


:intro
::����� ����� ����� ������
::�� ����� �� �� ��� ������
cls
if "%clear_heb%"=="����" (
for /r %%i in (*) do (
cls
echo.
echo                           ...������ ���� �� ����� ����
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)
)
goto :preparing

:clear-func
::�������� ����� ����� �� ���� ������
set "new_filename=%file:_= %"
set "new_filename=%new_filename: -���� ������=%"
set "new_filename=%new_filename: - ���� �����=%"
set "new_filename=%new_filename: -���� ������=%"
set "new_filename=%new_filename:-����� �������=%"
set "new_filename=%new_filename: - ����� �������=%"
set "new_filename=%new_filename: - ����=%"
set "new_filename=%new_filename: ������ ��� ���=%"
set "new_filename=%new_filename: - ���� ������=%"
ren "%file%%ext%" "%new_filename%%ext%"
exit /b


:preparing
cls
echo.
echo                                    ...����
::���� ������ ������ ������

::����� ����� ������ ���� ��� ������ ������
if "%dir_heb%"=="����" (set tree=/r) else (set tree=)
::����� ������ ���� ����� ������
set/a d=1
::����� ����� ���� ����� ������ ��� ���
set pro_scan=False
:start
::����� ���� ������ ������ ���� �����
for /f "usebackq tokens=1,2 delims=,"  %%i in (%csv-file%) do (
set a=%%i
set c=%%j
call :sort-func
)
goto :finish

:sort-func
::����� �� ���� ������
set a=*%a: =?%*.*

::����� ����� ����� ���� ������ �������
set/a en=%d%00/ab
if not "%en%"=="%enb%" cls & echo. & echo                                    ...���� & echo. & echo. & echo. & echo                               ...������ ������ %en%
set/a enb=%d%00/ab

::���� �� ������ ����� ������ ��� � �
if "%abc_heb%"=="����" set w=%c:~0,1%\

::����� ���� ��� �� ���� ������ - ����� ������� ������
set b="%h%\%w%%c%%s%"

::����� ����� ��� ����� ���� �����
set xx=v
set ss=z
for %tree% %%c in ("%a%") do if exist %%c set ss=ss
if "%fixed_heb%"=="����" if not exist "%h%\%w%%c%" set ss==z
if %ss%==ss md %b%

::����� ����� ���� ���� ���� ����� �� ����� ��
if %c_or_m%==del set b= & set par=/q

::����� �������� ����� ���� �����
for %tree% %%d in ("%a%") do if exist %%d set xx=xx
if exist %b% if %xx%==xx for %tree% %%e in (%a%) do %c_or_m% %par% "%%e" %b%>>�����

::���� ����� ��� ����� ����� ��������
set/a d=d+1

::����� ��������� ����� ������ ����
exit /b


:finish
cls
echo.
echo.
echo                                 %VER% �������� ����
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !����� ������ & echo. & del ����� & goto :intro_pro
if exist ����� (echo                                 :%msg% ������ ���� & find /c "1" �����
) else (
echo                                   !��� ���� ��
)
if exist ����� (del �����) else (goto :intro_pro)
echo.
::�� ������ ����� ����� ������ ����� �� ���� �����
if not "%pro_scan%"=="True" if %c_or_m%==xcopy echo. & echo                 [2] ��� ��� �������� ������ �� ����� ������� ��� �� & echo                         [1] ��� ���� ����� ������� ��� �� & echo. & echo               !������ ����� ���� ����� ���� �� ����� ����� �� !������ & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto preparing & if errorlevel 1 goto :intro_pro

:intro_pro
::�� ������ ����� ��� ���
::����� ���� �������� ��
if "%artist_heb%"=="����" goto :pro_scanner

:pause
echo.
echo                         !��� ������ ����� ����� ��� �� ���
pause>nul
cls
goto :mesader-singels


:pro_scanner
::������� ������ ������� ����� ��� �� ���� ������ �����
if not exist "%AppData%\singles-sorter\MediaInfo.exe" if exist "%~dp0MediaInfo.exe" copy "%~dp0MediaInfo.exe" "%AppData%\singles-sorter"
::����� ����� ������ �������� ������ ������
path "%AppData%\singles-sorter";%path%
echo.
timeout 10 | echo            ��� ������� �� �� ��� ��� ������ ����� ��� ����� ���� ����
for %tree% %%s in (*.mp3,*.wma,*.wav) do (
set file=%%~s
call :scanner_func
set/a d=d+1
)
::����� ����� ������ ������ ������ ������
::���� ����� ������
del "%Temp%\artist-song.tmp"
del "%Temp%\artist-song-ansi.tmp"
set "artist_heb=���� ��"
set pro_scan=True
goto :finish

:scanner_func
::����� ������
cls
echo.
echo                                    ...����

:: ����� ������ ������� ������ ����� ���� �����
::����� ���� ������ �� �� ���� ���� �����
:: ������ ����� �����
mediainfo "%file%" | findstr /b "Performer">"%Temp%\artist-song.tmp"
::���� ���� ���� ������ ���� ����� ����
powershell "(Get-Content "%Temp%\artist-song.tmp" -Encoding utf8 | Out-File "%Temp%\artist-song-ansi.tmp" -Encoding default)"

::����� �� ������ ���� ������� �����
find /c """" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b
find /c "?" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b
::����� �� ����� ���
if ;;;;;;;;;;;; exit /b

:: ����� ���� ����� ������
set/p artist=<"%Temp%\artist-song-ansi.tmp"


::���� ���� ������ ������ �� ���� ����
set "artist=%artist:~43%"

::����� ��������� ����� ������� ���
for %%c in ("a")do (if "[%artist%]"=="[~43]" exit /b)

::����� ���:
::����� �� ���� ���� ����� ������
::�� ��� ���� ������ ����� �� ����
::���� ����� ������� ��� ����
if "%abc_heb%"=="����" set w=%artist:~0,1%\
find /c "%artist%" "%csv-file%">nul
if %errorlevel%==0 (
if "%fixed_heb%"=="���� ��" (if not exist "%h%\%w%%artist%%s%" md "%h%\%w%%artist%%s%"
) else (
if exist "%h%\%w%%artist%" if not exist "%h%\%w%%artist%%s%" md "%h%\%w%%artist%%s%")
if exist "%h%\%w%%artist%%s%" %c_or_m% %par% "%file%" "%h%\%w%%artist%%s%">>�����
)
::����� ��������� ����� ������ ����
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

::�����: nh.local11@gmail.com

