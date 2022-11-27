@echo off
chcp 1255>nul
MODE CON: COLS=70 lines=40
color f1
title ���� ������� 7.0 ���
echo.
if not [%1]==[] set p=%1 & goto :start

:begin
cls
echo Insert singer folder path to search
echo (on the folder to contain singles folder)
echo.
set /p p=">>>"

:start
if exist "%temp%\list-to-del.csv" del "%temp%\list-to-del.csv"
for %%i in (%p%) do set artist=%%~ni
for %%i in (%p%) do set p=%%~i
if exist "%p%\�������" (
cd /d "%p%\�������"
) else (
echo !������� ����� ����� �� ������
timeout 2
exit /b )

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

dir /b >"%temp%\000.tmp"

for /f "usebackq tokens=1,2,3,4,5,6,7,8,9 delims= " %%i in (%temp%\000.tmp) do (
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
if exist "%temp%\000.tmp" del "%temp%\000.tmp"
if exist %del_file_tmp% del %del_file_tmp%
if not exist %del_file% (
echo not multiply files
timeout 2
exit /b
)


::����� ����� ��� ������
cls
echo.
set /a num=1
for /f "eol=;tokens=1,1*delims=" %%a in (%temp%\list-to-delete.tmp) do (
set item="%%~a"
set viwe_item="%%~na"
call :choice-delete
)
del %del_file%


::���� ����� ������ ������

echo -------------------
echo Press any key to delete file
echo Press 0 to start again

:choicer
set item_num=
set/p item_num=">>>"
if not defined item_num goto :choicer
if %item_num%==0 exit /b
for /f "tokens=1,2 delims=:"  %%i in ('type "%Temp%\list-to-del.csv"  ^| findstr /l "[%item_num%]"') do (
del %%j && echo the file is deleted!
)
goto :choicer

pause
exit /b

:choice-delete

echo [%num%] %viwe_item% 
echo [%num%]:%item%>>"%Temp%\list-to-del.csv"
set /a num=num+1
exit /b



:func-scan
::���� ��� ������ �������� ��� �����
::��� ����� �� ������ ������� �� �� ����
::������ �� ����� ������ �����
if "%i% %j%"=="" exit /b
set "file_tokens=%i% %j% %k%"
if "%i% %j%"=="%artist%" set "file_tokens=%k% %l% %m%"
if "%i% %j% %k%"=="%artist%" set "file_tokens=%l% %m% %n%"
if "%j% %k%"=="%artist%" set "file_tokens=%l% %m% %n%"
if "%j% %k% %l%"=="%artist%" set "file_tokens=%m% %n% %o%"
if "%artist%" == "%file_tokens%" exit /b

::����� ���� ������ ������ ������
for /f "tokens=1,2* delims=" %%i in ('dir /b ^| find /c "%file_tokens%"') do (set num=%%i)


::����� ��� ����� ����� ��� ����� ������ �������
::������ ����� ��� ��� ����� ��� ������ �� ����� ����� ������
set del_file="%temp%\list-to-delete.tmp"
set del_file_tmp="%temp%\list-to-delete-temp.tmp"

if %num% gtr 1 dir /b | find "%file_tokens%">%del_file_tmp%
if exist %del_file_tmp% if %num% gtr 1 for /f "eol=;tokens=1,1*delims=" %%b in (%temp%\list-to-delete-temp.tmp) do (find /c "%%b" %del_file%
if errorlevel 1 dir /b | find "%file_tokens%">>%del_file%
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
exit /b


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