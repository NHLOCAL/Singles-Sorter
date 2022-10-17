
for %%i in (*ver*) do (powershell -Command "(gc %%i) -replace '8.9', '9.0' | Out-File -encoding default folder\%%i")
