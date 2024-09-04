@echo off

echo.
echo #1 - duplic singers removing

cd level1

"1-remove-duplic-singers.py"

echo.
echo #2 song list processing

"2-song-list-processor.py"

echo.
echo #3 - singers randoming

"3-randomer_singers.py"

echo.
echo #4 - creat json

cd..

"1-creat_json.py"

echo.
echo #5 - cleaner json

"2-cleaner_json.py"

echo.
echo #6 - pull to git

rem git pull

pause