@echo off
chcp 1255

::���� �����
if not "%1"=="am_admin" (powershell start -verb runas '%0' am_admin & exit /b)


::����� ������ ���
del "%userprofile%\Desktop\���� �������� 10.lnk"
del "%AppData%\Microsoft\Windows\Start Menu\Programs\���� ��������\���� �������� 10.0.lnk"

::����� ��� ������ ������ �� ����� ������� �����
reg delete "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\SinglesSorter" /f

::����� ���� ������
for %%i in ("%appdata%\singles-sorter\*.*") do (if not "%%~nxi"=="remover_ss.bat" del "%%~i")
del %0%