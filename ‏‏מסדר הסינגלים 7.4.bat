::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
title %VER% ���� ��������
MODE CON COLS=80 lines=27
color f1
set VER=7.4

::����� �� ���� ���� ����� ������
curl https://raw.githubusercontent.com/NHLOCAL/Singles-Sorter/main/versions.data/new-ver-exist -o "%temp%\ver-exist-7.tmp"
if errorlevel 1 goto :call-num else (
set/p update=<"%temp%\ver-exist-7.tmp"
del "%temp%\ver-exist-7.tmp"
if %update% GTR %VER% goto :updating
)


::����� ���� ������ ����� ��� �������
::���� ���� ����� ����� ���� ����
::������ �������� �� ����� ����� ������

:call-num
del "%temp%\ver-exist-7.tmp"
goto number%ab%


:sln-start
cls
set/a abc=%ab%


goto mesader-singels

::��� ������ ����� �� ��� ������
:singer-list-new
setlocal enabledelayedexpansion
cls
echo.
echo.
echo.
echo.
echo                             ����� ����� - �������� ����
echo                      =========================================
echo.
echo.
echo                         ���� �� - [1] ��� ��� ����� ������
echo                           [2] ���� ���� ����� ���� ������
echo                     [3] ������ ����� ���� ���� ���� ����� ������ 
echo                                 [4] ����� ������ �����
echo.
echo                                ���������� ���� ���
choice /c 1234>nul
if errorlevel 4 goto mesader-singels
if errorlevel 3 goto 3.0
if errorlevel 2 goto 2.0
if errorlevel 1 goto :singer-list-new

:3.0
cls
echo.
echo                               ���� ���� ���� ��� ����
echo                          �� ������ ������ ����� �� �� ����
echo                            ���� ��� ��� ���� �� ����� ��
echo                           ���� ����� ����� �� ���� �������
echo.
timeout 5|echo                         ����� ����� ����� ������ ��� !������
echo ���� ��� ���� ����� ������ ���� �� ���� �� - ���� �� ��� ������ ����� �����>"c:\users\public\list.txt"
notepad "c:\users\public\list.txt"
goto :2.3
:2.0
cls
echo.
echo.
echo.
echo.
echo                             ����� ����� - �������� ����
echo                      =========================================
echo.
echo.
echo                            ������ ����� �� ����� �� ���� ����
echo                     !��� �� ��� ���� �� ������� ����� ������ ������
echo                             (����+0 ����� ������ ������)
set/p pa=
if %pa%==0 goto :mesader-singels
if not exist %pa% echo. & timeout 2|echo                                !��� ��� ���� ����� & goto 2.0
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
echo                                   !������ ������
goto :1.3
:1.0
cls
echo.
echo.
echo.
echo.
echo                             ����� ����� - �������� ����
echo                      =========================================
echo.
echo.
echo                            ���� ����� ��� �� �� ��� ����
echo                          ���� ���� - ���� ����� ���� �� ���
echo                             (����+0 ����� ������ ������)
goto 1.2
:1.1
cls
echo                              ...���� ��� �� �� ����
echo                          (1 ����� ��� ��� ������ ������)
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
timeout 5 | echo                      ...��� ���� ���� ��� ������ ������ �����
cls & %0%


:mesader-singels
cls

::���� �������� ����
:mesader2
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
echo                               ������� ���� ������ *
echo                           ���� ������� ����� ����� *
echo                            ������ ����� �� �� ����� *
echo.                          
echo                  ����� ���� �� ������� ������ ������ �������
echo               ���� ����� ����� _ ���� �� ����� ���� �� :������
echo             ����� ������ "���� ������" ,"����� ����" ��� �������
echo.
echo                            :����� nh.local11@gmail.com
echo.
echo                        ����� ������ ����� ����� ��� �� ���
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
echo.
echo                                        ___
echo                                       ^|   ^|
echo                                       \   /
echo                                        \_/
echo                                        ^(_^)
echo ================================================================================
echo.
echo.
echo.                           !���� ��� %update% ���� !��� ���
timeout 7 | echo               ...��� ���� ����� ������ ����� ����� �� ����� ���� 
explorer "%~dp0"
cls & "%~dp0\���� �������� %update%.bat"

)

goto :mesader-singels

:help
cls
echo.
echo.
echo.
echo                                ���� - �������� ����
echo                                       *******
echo.
echo           ����� ��� ������ ����� ���� �������� �� ���� ��� ������ ���� 
echo.
echo                                       :1 ���
echo         ���� �� ������ ����� ���� ���� ������� �������� ������ �� ����� ��
echo             ���� ������ ����� ���� ������ ��� ����� ����� �� ���� ����
echo.
echo                  ����� ������� ������� �������� ���� ���� �� ���� 
echo            ������� ���� �� ����� ��� �� ������ ��������� �� ����� �����
echo                     ������� ������� �� ���� ��� ���� �� ���
echo                                   !��� ������ !���
echo.
echo          ������ ���'�� ����� ����� ���� ����� ��� ������ ������ �� ���� 
echo                   ���� ������ �� ����� ������ !����� ���� 
echo.
echo                          ...����� ������ ������ ����    
echo                        mesader.singelim@gmail.com :����
echo.
echo                              1 ��� ���� ����� �����
echo                           2 ��� ����� ���� ���� ������
echo                           3 ��� ����� ���� ���� ������
echo                             4 ��� ����� ������ �����
choice /c 1234>nul
if errorlevel 4 goto :mesader-singels
if errorlevel 3 start mailto:mesader.singelim@gmail.com?subject=�������%%20�����%%20������%%20�������%%20������%%20��%%20������^&body= & goto :mesader-singels
if errorlevel 2 (curl https://www.googleapis.com/drive/v3/files/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1?alt=media^&key=AIzaSyDduW1Zbi2MIu8aMUMF6op72pJ1f0sPBi0 -o "%userprofile%\downloads\������ ����� ��������.pdf"
cls
echo.
echo.
echo                         !��� ������� ������� ��� �����
pause>nul
goto :mesader-singels
)
if errorlevel 1 start https://drive.google.com/file/d/1RJWxutr4oGNtL11vmsncVyfQ0jOvWQX1/preview & goto :mesader-singels


:Wrong_path
echo                      !��� ����� �� ���� ��� !���� ���� �����
timeout 2 >nul
:begining
::del %help%
cls
set a=*���*����*.*
set c=��� ����
set d=1

echo.
echo                                     _  __   __
echo                                    / ^| \ \  \ \
echo                                    ^| ^|  \ \  \ \
echo                                    ^| ^|  / /  / /
echo                                    ^|_^| /_/  /_/
echo ================================================================================
echo                                 %VER% �������� ����
echo                                       *****
echo                              ����+0 ����� ������ ������
echo.
echo.
echo                ������ ���� ���� ������� ����� ���� ��� �� ����� ����
echo                       ���� ����� ����� ���� ��� - ��������
echo.
echo                         !���� ����� �� ����� ����� ����
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
echo                                 %VER% �������� ����
echo                                       *****
echo                              ����+0 ����� ������ ������
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
if 1%h%1 == 111 md "������� �������" & set h="%~dp0������� �������"
if 1%h%1 == 101 goto :mesader-singels
if not exist %h% goto :target_folder
for %%i in (%h%) do set h=%%~i
cd /d %p%

::����� ������ ����� ������ ������
set "clear_heb=����"
set cm_heb=�����
set "abc_heb=���� ��"
set c_or_m=move
set "sing_heb=���� ��"
set "fixed_heb=���� ��"
::����� ������� ����� ������
:options
cls
echo.
echo                                 __   __    _____
echo                                 \ \  \ \  ^|___ /
echo                                  \ \  \ \   ^|_ \
echo                                  / /  / /  ___) ^|
echo                                 /_/  /_/  ^|____/
echo ================================================================================
echo                          ����� �������� - %VER% �������� ����
echo                                       *****
echo.
echo.
echo                             ���� �������� ������� ��� 
echo               ������ ������ ��� ������ ������� ����� ���� ����� ����
echo.
::���� ������ ��� ������� ������ ����

echo              [%clear_heb%] ����� ����� ������ ���� ������ [0] ��� !���
echo                   ------------------------------------------
echo                   [%cm_heb%] ������ ����� ��� ������ [1] ��� 
echo              [%abc_heb%] '� '�� ������� ������� ������ ������ [2] ���
echo         [%sing_heb%] ��� �� ���� "�������" ��� ������ ����� ������ [3] ��� 
echo             [%fixed_heb%] ���� ��� ������� ������ ������� ������ [4] ���
echo                                ������ ����� [5] ���
::����� ������ ������
choice /c 012345>nul
::�� ���� 5 �����
if errorlevel 6 goto :final
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
::���� ����� �� ������ ����� ������ �����
if "%clear_heb%"=="����" (
echo                ������ ���� ���� ���� ������ ���� �� ����� ����� !���
echo                  -------------------------------------------------
)
::���� ��� ����� �� ����� ����� �� �����
echo               �������� �� ---%cm_heb%--- ��� ����� �������� ��� !�� ���

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

::����� ������ �� ������ - ��� �����
set cm_heb=
set a=*���*����*.*
set c=��� ����
set d=1

:start
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
for /r %%i in ("%a%") do if exist %%i set ss=ss

if "%fixed_heb%"=="����" if not exist "%h%\%w%%c%" set ss==z

if %ss%==ss md %b%
if %c_or_m%==del set b= & set par=/q

::����� �������� ����� ���� �����
for /r %%i in ("%a%") do if exist %%i set xx=xx
if %xx%==xx for /r %%i in (%a%) do %c_or_m% %par% "%%i" %b%>>�����

::����� ������� ����� �� ����� ������ ������
goto %d%

::����� ���� ������

::����� - ���� ����� ����� goto
:1
::����� �� ���� ������
set a=*�����*����*.*
::����� �� ����� ������
set c=����� ����
::����� ����� ��� ������
set d=2
::���� ������  - �� ������ ����� ������
goto start
:2
set a=*�����*����*.*
set c=����� ����
set d=3
goto :start
:3
set a=*������*�������*.*
set c=������ �������
set d=4
goto :start
:4
set a=*�������*���*.*
set c=������� ���
set d=5
goto :start
:5
set a=*����'�*���*.*
set c=����'� ���
set d=6
goto :start
:6
set a=*�����*�������*.*
set c=����� �������
set d=7
goto :start
:7
set a=*����**����*.*
set c=���� ����
set d=8
goto :start
:8
set a=*����*�����*.*
set c=���� �����
set d=9
goto :start
:9
set a=*����*�����*.*
set c=���� �����
set d=10
goto :start
:10
set a=*����*��������*.*
set c=���� ��������
set d=11
goto :start
:11
set a=*����*�����*.*
set c=���� �����
set d=12
goto :start
:12
set a=*�����*���*.*
set c=����� ���
set d=13
goto :start
:13
set a=*������*�����*.*
set c=������ �����
set d=14
goto :start
:14
set a=*�����*���*.*
set c=����� ���
set d=15
goto :start
:15
set a=*�����*����*.*
set c=����� ����
set d=16
goto :start
:16
set a=*���*������*.*
set c=��� ������
set d=17
goto :start
:17
set a=**���*�����*.*
set c=��� �����
set d=19
goto :start
:19
set a=*���*������*.*
set c=��� ������
set d=20
goto :start
:20
set a=*�����*�������*.*
set c=����� �������
set d=21
goto :start
:21
set a=*���*��������*.*
set c=��� ��������
set d=22
goto :start
:22
set a=*���*����*.*
set c=��� ����
set d=24
goto :start
:24
set a=*����*�����*.*
set c=���� �����
set d=25
goto :start
:25
set a=*��*����*����*.*
set c=�� ���� ����
set d=26
goto :start
:26
set a=*���*������*.*
set c=��� ������
set d=27
goto :start
:27
set a=*����*�����*.*
set c=���� �����
set d=28
goto :start
:28
set a=*����*�����*.*
set c=���� �����
set d=29
goto :start
:29
set a=*����*����*.*
set c=���� ����
set d=30
goto :start
:30
set a=*����*����*.*
set c=���� ����
set d=31
goto :start
:31
set a=*����*����*.*
set c=���� ����
set d=32
goto :start
:32
set a=*��*����*.*
set c=�� ����
set d=33
goto :start
:33
set a=*��*�����*.*
set c=�� �����
set d=35
goto :start
:35
set a=*���*������*.*
set c=��� ������
set d=36
goto :start
:36
set a=*���*����*.*
set c=��� ����
set d=37
goto :start
:37
set a=*���*����*.*
set c=��� ����
set d=38
goto :start
:38
set a=*����*����*.*
set c=���� ����
set d=39
goto :start
:39
set a=*����*�����*.*
set c=���� �����
set d=40
goto :start
:40
set a=*����*�������*.*
set c=���� �������
set d=44
goto :start
:44
set a=*����*.*
set c=����
set d=46
goto :start
goto :start
:46
set a=*���*����*.*
set c=��� ����
set d=47
goto :start
:47
set a=*�������*.*
set c=�������
set d=48
goto :start
:48
set a=*�������*.*
set c=�������
set d=49
goto :start
:49
set a=*�����*����*.*
set c=����� ����
set d=50
goto :start
:50
set a=*������*.*
set c=������
set d=51
goto :start
:51
set a=*�������*��������*.*
set c=������� ��������
set d=52
goto :start
:52
set a=*����*.*
set c=����
set d=53
goto :start
:53
set a=*����?�����*.*
set c=���� �����
set d=54
goto :start
:54
set a=*����*����*������*.*
set c=���� ���� ������
set d=55
goto :start
:55
set a=*�����*�����*.*
set c=����� �����
set d=56
goto :start
:56
set a=*���*��*���*.*
set c=��� �� ���
set d=57
goto :start
:57
set a=*�����*�����*.*
set c=����� �����
set d=58
goto :start
:58
set a=*�����*����*.*
set c=����� ����
set d=59
goto :start
:59
set a=*�����*���*.*
set c=����� ���
set d=60
goto :start
:60
set a=*�����*�������*.*
set c=����� �������
set d=61
goto :start
:61
set a=*�����*�����*.*
set c=����� �����
set d=62
goto :start
:62
set a=*�����*�������*.*
set c=����� �������
set d=63
goto :start
:63
set a=*�����*�����*.*
set c=����� �����
set d=64
goto :start
:64
set a=*����*����*.*
set c=���� ����
set d=65
goto :start
:65
set a=*���������*.*
set c=���������
set d=66
goto :start
:66
set a=*����*��������*.*
set c=���� ��������
set d=67
goto :start
:67
set a=*�����*����*.*
set c=����� ����
set d=68
goto :start
:68
set a=*�����*�������*.*
set c=����� �������
set d=69
goto :start
:69
set a=*����*����*.*
set c=���� ����
set d=70
goto :start
:70
set a=*����*����*������*.*
set c=���� ���� ������
set d=71
goto :start
:71
set a=*����*���*����*.*
set c=���� ��� ����
set d=72
goto :start
:72
set a=*����*������*.*
set c=���� ������
set d=73
goto :start
:73
set a=*����*.*
set c=���� ������
set d=74
goto :start
:74
set a=*����*������*.*
set c=���� ������
set d=75
goto :start
:75
set a=*���*������*.*
set c=��� ������
set d=76
goto :start
:76
set a=*���*����*.*
set c=��� ����
set d=77
goto :start
:77
set a=*�����*����*.*
set c=����� ����
set d=78
goto :start
:78
set a=*�����*����*.*
set c=����� ����
set d=79
goto :start
:79
set a=*�����*���*.*
set c=����� ���
set d=80
goto :start
:80
set a=*�����*������*.*
set c=����� ������
set d=81
goto :start
:81
set a=*�����*������*.*
set c=����� ������
set d=84
goto :start
:84
set a=*���*���*.*
set c=��� ���
set d=85
goto :start
:85
set a=*���*�������*.*
set c=��� �������
set d=86
goto :start
:86
set a=*����*������*.*
set c=���� ������
set d=87
goto :start
:87
set a=*����*����*.*
set c=���� ����
set d=89
goto :start
:89
set a=*����*�������*.*
set c=���� �������
set d=90
goto :start
:90
set a=*����*��������*.*
set c=���� ��������
set d=91
goto :start
:91
set a=*����*���*.*
set c=���� ���
set d=92
goto :start
:92
set a=*�����*����*.*
set c=����� ����
set d=93
goto :start
:93
set a=*�����*��������*.*
set c=����� ��������
set d=94
goto :start
:94
set a=*�����*�������*.*
set c=����� �������
set d=95
goto :start
:95
set a=*����*�����*.*
set c=���� �����
set d=96
goto :start
:96
set a=*�����*�����*.*
set c=����� �����
set d=97
goto :start
:97
set a=*����*�'����*.*
set c=���� �'����
set d=98
goto :start
:98
set a=*����*����*.*
set c=���� ����
set d=100
goto :start
:100
set a=*���*.*
set c=����� �� ���
set d=101
goto :start
:101
set a=*�����*��*���*.*
set c=����� �� ���
set d=102
goto :start
:102
set a=*�����*�����*.*
set c=����� �����
set d=103
goto :start
:103
set a=*���*�����*.*
set c=��� �����
set d=104
goto :start
:104
set a=*���*���*.*
set c=��� ���
set d=105
goto :start
:105
set a=*�����*��������*.*
set c=����� ��������
set d=106
goto :start
:106
set a=*�����*���*.*
set c=����� ���
set d=107
goto :start
:107
set a=*�����*����*.*
set c=����� ����
set d=108
goto :start
:108
set a=*���*��*.*
set c=��� ��
set d=109
goto :start
:109
set a=*�����*����*.*
set c=����� ����
set d=110
goto :start
:110
set a=*�����*����*.*
set c=����� ����
set d=111
goto :start
:111
set a=*����*��������*.*
set c=���� ��������
set d=112
goto :start
:112
set a=*����*�������*.*
set c=���� �������
set d=114
goto :start
:114
set a=*����*�������*.*
set c=���� �������
set d=115
goto :start
:115
set a=*����*�����*.*
set c=���� �����
set d=116
goto :start
:116
set a=*���*����������*.*
set c=��� ����������
set d=117
goto :start
:117
set a=*����*����*.*
set c=���� ����
set d=118
goto :start
:118
set a=*����*.*
set c=����
set d=119
goto :start
:119
set a=*���*���*.*
set c=��� ���
set d=120
goto :start
:120
set a=*����*�����*.*
set c=���� �����
set d=121
goto :start
:121
set a=*����*��������*.*
set c=���� ��������
set d=122
goto :start
:122
set a=*�������*.*
set c=�������
set d=123
goto :start
:123
set a=*��������*.*
set c=���������
set d=124
goto :start
:124
set a=*����*�����*.*
set c=���� �����
set d=125
goto :start
:125
set a=*����*���*.*
set c=���� ���
set d=128
goto :start
:128
set a=*�����*.*
set c=�����
set d=129
goto :start
:129
set a=*������*�����*.*
set c=������ �����
set d=130
goto :start
:130
set a=*������*����*.*
set c=������ ����
set d=131
goto :start
:131
set a=*����*�����*.*
set c=���� �����
set d=132
goto :start
:132
set a=*�����*������*.*
set c=������ ������
set d=134
goto :start
:134
set a=*����*�����*�����*.*
set c=���� ����� �����
set d=135
goto :start
:135
set a=*����*���*.*
set c=���� ���
set d=136
goto :start
:136
set a=*����*������*.*
set c=���� ������
set d=137
goto :start
:137
set a=*����*����*.*
set c=���� ����
set d=138
goto :start
:138
set a=*�����*.*
set c=�����
set d=139
goto :start
:139
set a=*�����*������*.*
set c=����� ������
set d=140
goto :start
:140
set a=*����*�����*.*
set c=���� �����
set d=141
goto :start
:141
set a=*����*������*.*
set c=���� ������
set d=142
goto :start
:142
set a=*����*�����*.*
set c=���� �����
set d=143
goto :start
:143
set a=*�����*�����*.*
set c=����� �����
set d=144
goto :start
:144
set a=*����*����*.*
set c=���� ����
set d=145
goto :start
:145
set a=*����*.*
set c=����
set d=146
goto :start
:146
set a=*���*�����*.*
set c=��� �����
set d=147
goto :start
:147
set a=*���*��*����*.*
set c=��� �� ����
set d=148
goto :start
:148
set a=*����*�����*.*
set c=���� �����
set d=149
goto :start
:149
set a=*�����*���*.*
set c=����� ���
set d=150
goto :start
:150
set a=*�����*�����*�����*.*
set c=����� ����� �����
set d=151
goto :start
:151
set a=*���*�����*.*
set c=��� �����
set d=152
goto :start
:152
set a=*�����*������*.*
set c=����� �������
set d=153
goto :start
:153
set a=*�����*�����*.*
set c=������ �����
set d=154
goto :start
:154
set a=*�����*����*.*
set c=���� ����
set d=155
goto :start
:155
set a=*���*���*.*
set c=��� ���
set d=156
goto :start
:156
set a=*������*���*.*
set c=������� ���
set d=157
goto :start
:157
set a=*�����*��������*.*
set c=����� ��������
set d=158
goto :start
:158
set a=*���*������*.*
set c=��� ������
set d=159
goto :start
:159
set a=*������*�����*.*
set c=����� �����
set d=160
goto :start
:160
set a=*�������*�������*.*
set c=������� ��������
set d=161
goto :start
:161
set a=*����*������*.*
set c=���� ������
set d=162
goto :start
:162
set a=*����*������*.*
set c=���� ������
set d=163
goto :start
:163
set a=*����*��*.*
set c=���� ��
set d=164
goto :start
:164
set a=*����*�������*.*
set c=���� ������
set d=165
goto :start
:165
set a=*����*������*.*
set c=���� ������
set d=166
goto :start
:166
set a=*�����*����*.*
set c=����� ����
set d=167
goto :start
:167
set a=*�����*����*.*
set c=����� ����
set d=168
goto :start
:168
set a=*����*����*.*
set c=���� ����
set d=169
goto :start
:169
set a=*����*������*.*
set c=������ ������
set d=170
goto :start
:170
set a=*����*�����*.*
set c=���� �����
set d=171
goto :start
:171
set a=*����*�������*.*
set c=���� �������
set d=172
goto :start
:172
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:173
set c=�� ���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:174
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:175
set c=������� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:176
set c=��� �� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:177
set c=��� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:178
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:179
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:180
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:181
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:182
set c=���� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:183
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:184
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:185
set c=���� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:186
set c=����� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:187
set c=���� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:188
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:189
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:190
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:191
set c=��� ����������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:192
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:193
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:194
set c=���� ��
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:195
set c=��� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:196
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:197
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:198
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:199
set c=���� �������
set a=*����*��*.*
set/a d=1+d 
goto start 
:200
set c=���� ���� �����
set a=*����*����*�����*.*
set/a d=1+d 
goto start 
:201
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:202
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:203
set c=��� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:204
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:205
set c=���� ��������
set a=*����*��������*.*
set/a d=1+d 
goto start 
:206
set c=���� ��������
set a=*����*�������*.*
set/a d=1+d 
goto start 
:207
set c=���� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:208
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:209
set c=�� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:210
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:211
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:212
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:213
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:214
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:215
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:216
set c=��� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:217
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:218
set c=��� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:219
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:220
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:221
set c=���� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:222
set c=����� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:223
set c=����'� ���
set a=*�����*���*.*
set/a d=1+d 
goto start 
:224
set c=���� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:225
set c=���� �������
set a=*�������*.*
set/a d=1+d 
goto start 
:226
set c=����'� ���
set a=*�����*���*.*
set/a d=1+d 
goto start 
:227
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:228
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:229
set c=���� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:230
set c=���� ���� �����
set a=*����*����*�����*.*
set/a d=1+d 
goto start 
:231
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:232
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:233
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:234
set c=����� �����
set a=*�����*������*.*
set/a d=1+d 
goto start 
:235
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:236
set c=��� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:237
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:238
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:239
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:240
set c=���� ���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:241
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:242
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:243
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:244
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:245
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:246
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:247
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:248
set c=���� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:249
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:250
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:251
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:252
set c=������ ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:253
set c=���� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:254
set c=����� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:255
set c=���� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:256
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:257
set c=����� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:258
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:259
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:260
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:261
set c=���� ���������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:262
set c=��� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:263
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:264
set c=����� ���������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:265
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:266
set c=���� �������
set a=*����Z*.*
set/a d=1+d 
goto start 
:267
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:268
set c=���� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:269
set c=��� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:270
set c=���� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:271
set c=���� ��
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:272
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:273
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:274
set c=����� ��� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:275
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:276
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:277
set c=���� ������
set a=*�������*.*
set/a d=1+d 
goto start 
:278
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:279
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:280
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:281
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:282
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:283
set c=���� ��������
set a=*����*���������*.*
set/a d=1+d 
goto start 
:284
set c=�� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:285
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:286
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:287
set c=���� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:288
set c=����� ������
set a=*����*������*.*
set/a d=1+d 
goto start 
:289
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:290
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:291
set c=������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:292
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:293
set c=���� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:294
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:295
set c=���� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:296
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:297
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:298
set c=������ �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:299
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:300
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:301
set c=���� ����� �������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:302
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:303
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:304
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:305
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:306
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:307
set c=��� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:308
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:309
set c=����� �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:310
set c=��� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:311
set c=������ ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:312
set c=���� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:313
set c=������ ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:314
set c=����'� ���
set a=*����'�*�����*.*
set/a d=1+d 
goto start 
:315
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:316
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:317
set c=����� ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:318
set c=����� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:319
set c=��� ���
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:320
set c=������ �����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:321
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:322
set c=����� ��������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:323
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:324
set c=���� ��
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:325
set c=������ ������
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:326
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:327
set c=���� ����
set a=*%c: =*%*.*
set/a d=1+d 
goto start 
:328
set c=����� �������
set a=*%c: =*%*.*
set/a d=1+d
goto start
:329
set c=���� ������
set a=*������*.*
set/a d=1+d
goto start
:330
set c=������ ����� ����
set a=*%c: =*%*.*
set/a d=1+d
goto start
:331
set c=������ ����� ����
set a=*������?�����?����*.*
set/a d=1+d
goto start
:332
set c=�� ����
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
echo                                 %VER% �������� ����
echo                                       *****
echo.
echo.
if %c_or_m%==del echo                                   !����� ������ & echo. & del ����� & goto pause
if exist ����� (echo                              :%msg% ������ ���� & find /c "1" �����
) else (
echo                                   !��� ���� ��
)
if not exist ����� set c_or_m=xxx
if exist ����� del �����
echo.
if %c_or_m%==xcopy echo. & echo                 [2] ��� ��� �������� ������ �� ����� ������� ��� �� & echo                         [1] ��� ���� ����� ������� ��� �� & echo. & echo               !������ ����� ���� ����� ���� �� ����� ����� �� !������ & choice /c 12>nul & if errorlevel 2 set c_or_m=del & goto preparing & if errorlevel 1 goto :pause
:pause
echo.
echo                         !��� ������ ����� ����� ��� �� ���
pause>nul
cls
goto :mesader-singels

:number
cls
set/a ab=333+1
find /c "number%ab%" %0%
if %errorlevel%==0 (goto :number%ab%) else (goto :sln-start)

::�����: nh.local11@gmail.com
