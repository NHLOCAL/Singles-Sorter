::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
set "VER=10.0"
title %VER% ���� ��������
MODE CON COLS=80 lines=27
::����� ����� ������ ���� �����
set "csv-file=%appdata%\singles-sorter\singer-list.csv"

:call-num
::����� ���� ������ ����� ��� �������
::���� ���� ����� ����� ���� ����
::������ �������� �� ����� ����� ������
if exist "%temp%\ver-exist-7.tmp" del "%temp%\ver-exist-7.tmp"
type "%csv-file%" | find /c ",">"%temp%\num-singer.tmp"
set /p ab=<"%temp%\num-singer.tmp"
if exist "%temp%\num-singer.tmp" del "%temp%\num-singer.tmp"
set/a abc=%ab%


::���� �������� ����
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

::����� ������ ������� ������ �����
cd /d "%source_path%"

::����� ������ ����� ������ ������

::����� ����� ������ ���� ������
set/p cleaning=<"%tmp%\select7_tmp.tmp"
if "%cleaning%"=="False" (set "clear_heb=���� ��") else (set "clear_heb=����")

::����� ����� ����� �� �����
set/p copy_moving=<"%tmp%\select3_tmp.tmp"
if "%copy_moving%"=="True" (
set c_or_m=xcopy
set par=/y
set "msg=�������"
set cm_heb=�����
) else (
set par=
set c_or_m=move
set "msg=�������"
set cm_heb=�����
)

::����� ����� ������ ������ ������ ������ ��' �'
set/p abc_dirs_creating=<"%tmp%\select2_tmp.tmp"
if "%abc_dirs_creating%"=="True" (set "abc_heb=����") else (set "abc_heb=���� ��")

::����� ����� ������ ����� ������� ������
set/p in_folder_creating=<"%tmp%\select1_tmp.tmp"
if "%in_folder_creating%"=="True" (
set sing_heb=����
set "s=\�������"
)else (
set "sing_heb=���� ��"
set s=
)

::����� ����� ������ ������ ����� �� ����� ������� ������ ����
set/p creating_folder=<"%tmp%\select4_tmp.tmp"
if "%creating_folder%"=="True" (set fixed_heb=����) else (set "fixed_heb=���� ��")

::����� ����� ������ ����� ������
set/p pro_scanning=<"%tmp%\select6_tmp.tmp"
if "%pro_scanning%"=="False" (set "artist_heb=���� ��") else (set "artist_heb=����")

::����� ����� ������ ������ ����
set/p tree_scanning=<"%tmp%\select6_tmp.tmp"
if "%tree_scanning%"=="True" (set "dir_heb=����") else (set "dir_heb=���� ��")


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
echo.
echo.
echo ================================================================================
echo                               !������ ������ ������
pause>nul
exit


:pro_scanner
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
for /f "tokens=1,2 delims=:" %%i in ('mediainfo "%file%" ^| findstr /b "Performer"') do echo %%j>"%Temp%\artist-song.tmp"

::����� �� ����� ��� ��� ���� �����
::������ ��������� �� ������ ������
for %%h in ("%Temp%\artist-song-ansi.tmp") do (if %%~zh==0 exit /b)

::���� ���� ���� ������ ���� ����� ����
powershell "(Get-Content "%Temp%\artist-song.tmp" -Encoding utf8 | Out-File "%Temp%\artist-song-ansi.tmp" -Encoding default)"

::����� �� ������ ���� ������� �����
::������ ��������� �� ������ ������
find /c """" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b
find /c "?" "%Temp%\artist-song-ansi.tmp">nul
if %errorlevel%==0 exit /b

:: ����� ���� ����� ������
set/p artist=<"%Temp%\artist-song-ansi.tmp"


::���� ���� ������ ������ �� ���� ����
set "artist=%artist:~1%"

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




::�����: nh.local11@gmail.com