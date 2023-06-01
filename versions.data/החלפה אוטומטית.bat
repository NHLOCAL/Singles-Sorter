md "folder"
set old=10.0
set new=12.0
copy "%old%+version" "%new%+version"
for %%i in (*ver*) do (powershell -Command "(gc %%i) -replace '%old%', '%new%' | Out-File -encoding default folder\%%i")
