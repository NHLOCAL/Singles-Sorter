@echo off
chcp 1255>nul
MODE CON: COLS=70 lines=40
color f1
title מזהה הכפולים
echo.

for /f "usebackq skip=2 tokens=1,2,3,4,5 delims= " %%i in (%1) do (set key=%%i
call :funcset)

:ender
echo %key-file%

pause
exit

:funcset
set key-file=%key-file%%key%
exit /b