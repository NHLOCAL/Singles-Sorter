@echo off


::���� �����
if not "%1"=="am_admin" (powershell start -verb runas '%0' am_admin & exit /b)
chcp 1255
mode con cols=16 lines=1

::����� ������ ���
del "%userprofile%\Desktop\���� �������� 10.lnk"
rd /s /q "%AppData%\Microsoft\Windows\Start Menu\Programs\���� ��������"

::����� ��� ������ ������ �� ����� ������� �����
reg delete "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\SinglesSorter" /f

::����� ���� ������
for %%i in ("%appdata%\singles-sorter\*.*") do (if not "%%~nxi"=="remover_ss.bat" del "%%~i")
del %0%