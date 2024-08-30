::���� ������� ��� ���� ����� ������ ������� ��� �����
::������� ����� ������ ����� ���� �� ����� ��� ������ ������ ��
::�����: nh.local11@gmail.com

@echo off
::������ �� ���, ���, ����� ����� �����
::���� ���� ������ ������
chcp 1255>nul
set "VER=12.8"
title %VER% ���� ��������
MODE CON COLS=80 lines=18
color f1


:beginning
cls
set source_path=%1
set "h=%~2"

::����� ������ ����� "clean"
::����� ����� ����� ����
if [%2]==[-clean] goto :intro


:preparing
echo.
echo.
echo.

singles_sorter_func.exe %source_path% "%h%" %3 %4 %5 %6 %7


echo.
echo.
echo.
echo                             !��� ��� !��� ������ 
echo.
echo                      ����� �� ����� ��� ����� ��� �� ���
pause>nul

::�������� ����� �����
exit



:intro
::����� ����� ����� ������
::�� ����� �� �� ��� ������
cls
cd /d %source_path%
for /r %%i in (*) do (
cls
echo.
echo                           ...������ ���� �� ����� ����
set "file=%%~ni"
set "ext=%%~xi"
call :clear-func
)

::����� ����� ������ ����� ����� ����
if [%2]==[-clean] pause & exit

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