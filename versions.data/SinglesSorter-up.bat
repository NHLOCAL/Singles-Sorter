@echo off
chcp 1255
color f1
title עדכון מסדר הסינגלים
MODE CON COLS=80 lines=27

echo.
echo                    הווצא ץבוקכ רתוי ןימז וניא םילגניסה רדסמ
echo                         !תבצועמהו השדחה הסרגה דרת דימ
echo.
timeout 3

start https://nhlocal.github.io/Singles-Sorter/site/download.html

del %0%

