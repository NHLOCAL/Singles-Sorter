@echo off
echo Installing Singles Sorter...
mkdir %AppData%\singles-sorter
xcopy /s /e /i %~dp0source_code %AppData%\singles-sorter
echo Installation complete.
