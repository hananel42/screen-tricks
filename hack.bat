@echo off
setlocal enabledelayedexpansion

set "REPO=hananel42/screen-tricks"

echo =================================
echo  Select a Screen Effect:
echo  [1] Particles
echo  [2] Wave
echo  [3] Triangulate
echo =================================

:: הטריק: קריאת קלט ישירות מהמקלדת הפיזית (CON) כדי שלא יישבר מה-Pipe
for /f "delims=" %%A in ('powershell -Command "[Console]::In.ReadLine()" ^< CON') do set "choice=%%A"

set "EFFECT=particles"
if "%choice%"=="2" set "EFFECT=wave"
if "%choice%"=="3" set "EFFECT=triangulate"

echo.
echo Launching %EFFECT%...

powershell -Command ^
    "$url = (Invoke-RestMethod -Uri 'https://api.github.com/repos/%REPO%/releases/latest').assets | Where-Object { $_.name -like '*%EFFECT%*.exe' } | Select-Object -ExpandProperty browser_download_url -First 1;" ^
    "Invoke-WebRequest -Uri $url -OutFile '%EFFECT%.exe';" ^
    "Unblock-File -Path '.\%EFFECT%.exe';" ^
    "Start-Process '.\%EFFECT%.exe'"

endlocal
