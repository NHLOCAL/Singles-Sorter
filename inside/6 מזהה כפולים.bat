@echo off
chcp 1255>nul
MODE CON: COLS=70 lines=40
color f1
echo.
if not [%1]==[] set p=%1 & goto :start
set /p p=">>>"
:start
for %%i in (%p%) do set artist=%%~ni
for %%i in (%p%) do set p=%%~i
cd /d "%p%\�������"
cls
path "C:\Users\�����\Desktop\������� 2.5\���� ��� ������\������ ��� �������\����� ������\���� ��������\���� �������� 2022\������ �������\MediaInfo_CLI_22.06_Windows_x64";%path%

::���� ���� ����� ������� �����
::�. ��� ���� ������
::�. ��� ���� �����
::�. ��� ���� �����
::�. ��� ���� ����� (���� ������ ���)

::goto :len-sort


:name-sort
::���� ��� ���� �����


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
::����� ������
cls
del "%temp%\number-find.txt"
del "%temp%\000.txt"
del "%temp%\list-to-delete-temp.tmp"
if not exist "%temp%\list-to-delete.tmp" (
echo not multiply files
timeout 2
exit
)
::����� ����� ��� ������
cls
echo.
set /a num=1
for /f "eol=;tokens=1,1*delims=" %%a in (%temp%\list-to-delete.tmp) do (
set item=%%a
set viwe_item="%%~na"
call :choice-delete
)
del "%temp%\list-to-delete.tmp"
::���� ����� ������ ������
echo -------------------
echo Pres any key to delete file
:choicer
choice /c ABCDEFGHIJKLMNOPQRSTUVWXYZ
if errorlevel 26 del "%itemZ%" & goto :choicer
if errorlevel 25 del "%itemY%" & goto :choicer
if errorlevel 24 del "%itemX%" & goto :choicer
if errorlevel 23 del "%itemW%" & goto :choicer
if errorlevel 22 del "%itemV%" & goto :choicer
if errorlevel 21 del "%itemU%" & goto :choicer
if errorlevel 20 del "%itemT%" & goto :choicer
if errorlevel 19 del "%itemS%" & goto :choicer
if errorlevel 18 del "%itemR%" & goto :choicer
if errorlevel 17 del "%itemQ%" & goto :choicer
if errorlevel 16 del "%itemP%" & goto :choicer
if errorlevel 15 del "%itemO%" & goto :choicer
if errorlevel 14 del "%itemN%" & goto :choicer
if errorlevel 13 del "%itemM%" & goto :choicer
if errorlevel 12 del "%itemL%" & goto :choicer
if errorlevel 11 del "%itemK%" & goto :choicer
if errorlevel 10 del "%itemJ%" & goto :choicer
if errorlevel 9 del "%itemI%" & goto :choicer
if errorlevel 8 del "%itemH%" & goto :choicer
if errorlevel 7 del "%itemG%" & goto :choicer
if errorlevel 6 del "%itemF%" & goto :choicer
if errorlevel 5 del "%itemE%" & goto :choicer
if errorlevel 4 del "%itemD%" & goto :choicer
if errorlevel 3 del "%itemC%" & goto :choicer
if errorlevel 2 del "%itemB%" & goto :choicer
if errorlevel 1 del "%itemA%" & goto :choicer


pause

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
echo %viwe_item% ]%let%[|rev
set item%let%=%item%
set /a num=num+1
exit /b



:func-scan
::���� ��� ������ �������� ��� �����
::��� ����� �� ������ ������� �� �� ����
::������ �� ����� ������ �����
if "%i% %j%"=="" exit /b
set "file_tokens=%i% %j% %k%"
if "%i% %j%"=="%artist%" set "file_tokens=%k% %l% %m%"
if "%i% %j% %k%"=="%artist%" set "file_tokens=%l% %m%%n%"
if "%j% %k%"=="%artist%" set "file_tokens=%l% %m% %n%"
if "%j% %k% %l%"=="%artist%" set "file_tokens=%m% %n% %o%"
if "%artist%" == "%file_tokens%" exit /b
dir /b | find /c "%file_tokens%">"%temp%\number-find.txt"
::����� ���� ������ ������ ������
set /p num=<"%temp%\number-find.txt"
::����� ��� ����� ����� ��� ����� ������ �������
::������ ����� ��� ��� ����� ��� ������ �� ����� ����� ������
if %num% gtr 1 dir /b | find "%file_tokens%">"%temp%\list-to-delete-temp.tmp"
if exist "%temp%\list-to-delete-temp.tmp" if %num% gtr 1 for /f "eol=;tokens=1,1*delims=" %%b in (%temp%\list-to-delete-temp.tmp) do (find /c "%%b" "%temp%\list-to-delete.tmp"
if errorlevel 1 dir /b | find "%file_tokens%" >> "%temp%\list-to-delete.tmp"
)
cls
exit /b



:size-sort
::���� ��� ���� �����



:len-sort
::���� ��� ���� �����

::����� ����� ������ ������ �����
for %%i in (*.mp3,*.wma,*.wav) do (
set file=%%~i
call :func-len
)


pause
exit


:func-len
::����� ����� ���� ���� ����� �� ��� ��� ������
MediaInfo "%file%" | findstr /b "Duration">"%Temp%\len-song.tmp"
::����� ������� ������ ���� �����
set/p len=<"%Temp%\len-song.tmp"
::����� ���� �� ���� �������
set "len=%len:~43%"
::����� �� ���� + ���� ���� �����
echo %file%,,%len% >>"%temp%\list-len-songs.tmp"
exit /b