@echo off

del_uninstall_list.reg

for %%i in ("%appdata%\singles-sorter\*.*") do (if not "%%~nxi"=="remover_ss.bat" del "%%~i")

del %0%