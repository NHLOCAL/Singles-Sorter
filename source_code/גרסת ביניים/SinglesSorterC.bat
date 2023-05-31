::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
set "VER=12.0"
title %VER% ���� ��������
MODE CON COLS=80 lines=18
::����� ����� ������ ���� �����
set "csv-file=%~dp0singer-list.csv"
set personal-csv-file="%~dp0personal-singer-list.csv"
if not exist %personal-csv-file% set personal-csv-file=

:call-num
::����� ���� ������ ����� ��� �������
::���� ���� ����� ����� ���� ����
::������ �������� �� ����� ����� ������
for /f "tokens=1,2* delims=" %%n in ('type "%csv-file%" ^| find /c ","') do set ab=%%n
if defined personal-csv-file for /f "tokens=1,2* delims=" %%n in ('type "%personal-csv-file%" ^| find /c ","') do set ac=%%n
set/a abc=ab+ac

::���� �������� ����
:mesader-singels
color f1


:beginning
cls
set source_path=%1

::����� ������ ������� ������ �����
cd /d %source_path%

::����� ������ ����� "clean"
::����� ����� ����� ����
if [%2]==[-clean] set "clear_heb=True" & goto :intro

:target_folder
cls
set "h=%~2"

::����� ������ ����� ������ ������
::================================

::����� ����� ������ ����� ������� ������
set in_folder_creating=%5
if "%in_folder_creating%"=="True" (
set sing_heb=True
set "s=\�������"
)else (
set "sing_heb=False"
set s=
)

::����� ����� ������ ������ ������ ������ ��' �'
set abc_dirs_creating=%7
if "%abc_dirs_creating%"=="True" (set "abc_heb=True") else (set "abc_heb=False")

::����� ����� ����� �� �����
set copy_moving=%3
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

::����� ����� ������ ������ ����� �� ����� ������� ������ ����
set creating_folder=%6
if "%creating_folder%"=="True" (set fixed_heb=True) else (set "fixed_heb=False")

::����� ����� ������ ������ ����
set tree_scanning=%4
if "%tree_scanning%"=="True" (set "dir_heb=False") else (set "dir_heb=True")



:intro
::����� ����� ����� ������
::�� ����� �� �� ��� ������
cls
if "%clear_heb%"=="True" (
for /r %%i in (*) do (
cls
echo.
echo                           ...������ ���� �� ����� ����
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)
)

::����� ����� ������ ����� ����� ����
if [%2]==[-clean] pause & exit
::���� ����� ���
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
if "%dir_heb%"=="True" (set tree=/r) else (set tree=)
::����� ������ ���� ����� ������
set/a d=1
::����� ����� ���� ����� ������ ��� ���
set pro_scan=False
:start
::����� ���� ������ ������ ���� �����
for /f "usebackq tokens=1,2 delims=,"  %%i in ("%csv-file%",%personal-csv-file%) do (
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
if "%abc_heb%"=="True" set w=%c:~0,1%\

::����� ���� ��� �� ���� ������ - ����� ������� ������
set b="%h%\%w%%c%%s%"

::����� ����� ��� ����� ���� �����
set xx=v
set ss=z
for %tree% %%c in ("%a%") do if exist %%c set ss=ss
if "%fixed_heb%"=="True" if not exist "%h%\%w%%c%" set ss==z
if %ss%==ss md %b%

::����� ����� ���� ���� ���� ����� �� ����� ��
if %c_or_m%==del set b= & set par=/q

::����� �������� ����� ���� �����
for %tree% %%d in ("%a%") do if exist %%d set xx=xx

if defined b if exist %b% (set xx=xx) else (set xx=vv)

if "%xx%"=="xx" for %tree% %%e in (%a%) do %c_or_m% %par% "%%e" %b%>>�����

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
if not "%pro_scan%"=="True" if %c_or_m%==xcopy (
echo.
echo                 [2] ��� ��� �������� ������ �� ����� ������� ��� ��
echo                         [1] ��� ���� ����� ������� ��� ��
echo.
echo               !������ ����� ���� ����� ���� �� ����� ����� �� !������
choice /c 12>nul
if errorlevel 2 set c_or_m=del & goto preparing
if errorlevel 1 goto :intro_pro
)

:intro_pro
::����� ���� ��������� �������
goto :pro_scanner

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

cls
echo.
echo                                    ...����
echo.
echo.
cd /d "%~dp0"
pro_func_release.exe %source_path% "%h%" %c_or_m% %abc_heb% %fixed_heb% %in_folder_creating% %dir_heb%


echo.
echo.
echo.
echo                             !��� ��� !��� ������ 
echo.
echo                      ����� �� ����� ��� ����� ��� �� ���
pause>nul

::�������� ����� �����
exit

set "artist_heb=False"
set pro_scan=True
goto :finish



::�����: nh.local11@gmail.com