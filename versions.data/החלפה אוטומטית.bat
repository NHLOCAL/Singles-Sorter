md "folder"
for %%i in (*ver*) do (powershell -Command "(gc %%i) -replace '9.0', '9.1' | Out-File -encoding default folder\%%i")
