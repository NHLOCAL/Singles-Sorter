@echo off

echo.
time /T
echo #1 - duplic singers removing
echo.

cd level1

"1-remove-duplic-singers.py"

echo.
time /T
echo #2 song list processing
echo.

"2-song-list-processor.py"

echo.
time /T
echo #3 - singers randoming
echo.

"3-randomer_singers.py"

echo.
time /T
echo #4 - creat json
echo.

cd..

"1-creat_json.py"

echo.
time /T
echo #5 - cleaner json
echo.

"2-cleaner_json.py"

echo.
time /T
echo Operation completed successfully!
echo.

rem  #6 - pull to git
rem git pull

pause