md "folder"
set old=9.1
set new=9.2
copy "%old%+version" "%new%+version"
for %%i in (*ver*) do (powershell -Command "(gc %%i) -replace '%old%', '%new%' | Out-File -encoding default folder\%%i")
