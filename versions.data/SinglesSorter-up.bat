@echo off
chcp 1255
color f1
title ����� ���� ��������
MODE CON COLS=80 lines=27

echo.
echo                    ����� ����� ���� ���� ���� �������� ����
echo                         !�������� ����� ����� ��� ���
echo.
timeout 3

start https://nhlocal.github.io/Singles-Sorter/site/download?utm_source=singles_sorter_program&utm_medium=desktop

del %0%

