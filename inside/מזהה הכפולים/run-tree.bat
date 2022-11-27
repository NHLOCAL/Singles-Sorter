@echo off
chcp 1255
FOR /d %%i in ("J:\שמע\כל המוזיקה\*") do call "C:\Users\אורי\Documents\GitHub\Singles-Sorter\inside\מזהה כפולים\Duplicatedetectiontree.bat" "%%~i" && cd "J:\שמע\כל המוזיקה"

pause