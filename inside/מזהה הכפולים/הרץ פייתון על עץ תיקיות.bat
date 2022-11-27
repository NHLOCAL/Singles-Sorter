@echo off
chcp 1255

for /d %%i in ("J:\שמע\כל המוזיקה\*") do "C:\Users\אורי\Documents\GitHub\Singles-Sorter\inside\מזהה הכפולים\Find_duplic_ample.py" "%%~i\סינגלים"

pause