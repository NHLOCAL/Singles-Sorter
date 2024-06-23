md "folder"
set old=13.0
set new=13.1
copy "%old%+version" "%new%+version"
for %%i in (*ver*) do (powershell -Command "(gc %%i) -replace '%old%', '%new%' | Out-File -encoding default folder\%%i")
