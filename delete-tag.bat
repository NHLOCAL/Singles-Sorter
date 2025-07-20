@echo off
rem Prompt for tag name
set /p TAG=Enter tag name to delete: 

rem Validate input
if "%TAG%"=="" (
  echo Error: no tag specified.
  exit /b 1
)

rem Delete the local tag (force‑delete)
echo Deleting local tag "%TAG%"...
git tag -d "%TAG%"  || echo Warning: local tag "%TAG%" may not exist.

rem Delete the remote tag
echo Deleting remote tag "%TAG%"...
git push --delete origin "%TAG%"  || echo Warning: remote deletion failed.

echo.
echo Tag "%TAG%" has been deleted (if it existed).
pause
