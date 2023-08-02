@echo off
chcp 1255
set "csv-file=%appdata%\singles-sorter\singer-list.csv"
set /p path_cd="PATH:>>>"
cd /d %path_cd%
set "is_file=%0%"
path "C:\Users\�����\Desktop\������� 2.5\���� ��� ������\������ ��� �������\����� ������\���� ��������\���� �������� 2022\������ �������\MediaInfo_CLI_22.06_Windows_x64";%path%

for /r %%s in (*.mp3) do (
set file=%%~s
call :func
)
del "%Temp%\artist-song.tmp"
del "%Temp%\artist-song-ansi.tmp"
pause
exit

:func
:: ����� ������ ������� ������ ����� ���� �����
::����� ���� ������ �� �� ���� ���� �����
:: ������ ����� �����
mediainfo "%file%" | findstr /b "Performer">"%Temp%\artist-song.tmp"
::���� ���� ���� ������ ���� ����� ����
powershell "(Get-Content "%Temp%\artist-song.tmp" -Encoding utf8 | Out-File "%Temp%\artist-song-ansi.tmp" -Encoding default)"

:: ����� ���� ����� ������
set/p artist=<"%Temp%\artist-song-ansi.tmp"
::����� ��������� ����� ������� ���
if "%artist%"=="" exit /b
::����� ���:
::���� ���� ������ ������ �� ���� ����
set "artist=%artist:~43%"
::����� �� ���� ���� ����� ������
::�� ��� ���� ������ ����� �� ����
::���� ����� ������� ��� ����
find /c "%artist%" "%csv-file%">nul
if %errorlevel%==0 (
md "�����\%artist%"
copy "%file%" "�����\%artist%"
)
::����� ��������� ����� ������ ����
exit /b