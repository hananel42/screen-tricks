@echo off
setlocal enabledelayedexpansion

set "REPO=hananel42/screen-tricks"
set "FLAG=%~1"
set "EFFECT=particles"

:: בדיקת הדגל שהמשתמש שלח
if "%FLAG%"=="-w" set "EFFECT=wave"
if "%FLAG%"=="-t" set "EFFECT=triangulate"
if "%FLAG%"=="-p" set "EFFECT=particles"

echo Launching %EFFECT%...

powershell -Command ^
    "$url = (Invoke-RestMethod -Uri 'https://api.github.com/repos/%REPO%/releases/latest').assets | Where-Object { $_.name -like '*%EFFECT%*.exe' } | Select-Object -ExpandProperty browser_download_url -First 1;" ^
    "Invoke-WebRequest -Uri $url -OutFile '%EFFECT%.exe';" ^
    "Unblock-File -Path '.\%EFFECT%.exe';" ^
    "Start-Process '.\%EFFECT%.exe'"

endlocal
