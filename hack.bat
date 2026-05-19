@echo off
set "REPO=hananel42/screen-tricks"

powershell -Command ^
    "$url = (Invoke-RestMethod -Uri 'https://api.github.com/repos/%REPO%/releases/latest').assets | Where-Object { $_.name -like '*.exe' } | Select-Object -ExpandProperty browser_download_url;" ^
    "Invoke-WebRequest -Uri $url -OutFile 'particles.exe';" ^
    "Unblock-File -Path '.\particles.exe';" ^
    "Start-Process '.\particles.exe'"
