@echo off
chcp 1255>nul
chcp 1255
echo.
set /p p=">>>"
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
notepad "%temp%\list-to-delete.tmp"
pause
::����� ����� ��� ������
cls
echo.
set /a num=1
for /f "eol=;tokens=1,1*delims=" %%a in (%temp%\list-to-delete.tmp) do (
set item=%%a
call :choice-delete
)
echo -------------------
:choicer
choice /c 123456789
if errorlevel 24 echo %item24% & pause & goto :choicer
if errorlevel 23 echo %item23% & pause & goto :choicer
if errorlevel 22 echo %item22% & pause & goto :choicer
if errorlevel 21 echo %item21% & pause & goto :choicer
if errorlevel 20 echo %item20% & pause & goto :choicer
if errorlevel 19 echo %item19% & pause & goto :choicer
if errorlevel 18 echo %item18% & pause & goto :choicer
if errorlevel 17 echo %item17% & pause & goto :choicer
if errorlevel 16 echo %item16% & pause & goto :choicer
if errorlevel 15 echo %item15% & pause & goto :choicer
if errorlevel 14 echo %item14% & pause & goto :choicer
if errorlevel 13 echo %item13% & pause & goto :choicer
if errorlevel 12 echo %item12% & pause & goto :choicer
if errorlevel 11 echo %item11% & pause & goto :choicer
if errorlevel 10 echo %item10% & pause & goto :choicer
if errorlevel 9  echo %item9% & pause & goto :choicer
if errorlevel 8  echo %item8% & pause & goto :choicer
if errorlevel 7  echo %item7% & pause & goto :choicer
if errorlevel 6  echo %item6% & pause & goto :choicer
if errorlevel 5  echo %item5% & pause & goto :choicer
if errorlevel 4  echo %item4% & pause & goto :choicer
if errorlevel 3  echo %item3% & pause & goto :choicer
if errorlevel 2  echo %item2% & pause & goto :choicer
if errorlevel 1  echo %item1% & pause & goto :choicer

pause
del "%temp%\list-to-delete.tmp"
exit

:choice-delete
echo [%num%] %item%
set item%num%=%item%
set /a num=num+1
exit /b



:func-scan
::���� ��� ������ �������� ��� �����
if "%i% %j%"=="" exit /b
if "%i% %j%"=="%artist%" exit /b
if "%i% %j% %k%"=="%artist%" exit /b
if "%j% %k%"=="%artist%" exit /b
if "%j% %k% %l%"=="%artist%" exit /b
dir /b | find /c "%i% %j% %k%">"%temp%\number-find.txt"
::����� ���� ������ ������ ������
set /p num=<"%temp%\number-find.txt"
::����� ��� ����� ����� ��� ����� ������ �������
::������ ����� ��� ��� ����� ��� ������ �� ����� ����� ������
if %num% gtr 1 dir /b | find "%i% %j% %k%">"%temp%\list-to-delete-temp.tmp"
if exist "%temp%\list-to-delete-temp.tmp" if %num% gtr 1 for /f "eol=;tokens=1,1*delims=" %%b in (%temp%\list-to-delete-temp.tmp) do (find /c "%%b" "%temp%\list-to-delete.tmp"
if errorlevel 1 dir /b | find "%i% %j% %k%" >> "%temp%\list-to-delete.tmp"
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