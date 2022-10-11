@echo off
chcp 1255
if not [%1]==[] set "file=%1" & goto :start
echo.
set /p file=">>>"
:start
for /f "eol=;tokens=1,1*delims=" %%f in (%file%) do echo %%f| rev>>"rev-file-new.txt"
pause