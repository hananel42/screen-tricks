$REPO = "hananel42/screen-tricks"

Clear-Host
Write-Host "=================================" -ForegroundColor Cyan
Write-Host " Select a Screen Effect:" -ForegroundColor Cyan
Write-Host " [1] Particles"
Write-Host " [2] Wave"
Write-Host " [3] Triangulate"
Write-Host "=================================" -ForegroundColor Cyan

# פקודה מיוחדת לקריאת מקלדת שעוקפת את ה-Pipe ב-PowerShell
$key = [Console]::ReadKey($true)
$choice = $key.KeyChar

$effect = "particles"
if ($choice -eq '2') { $effect = "wave" }
if ($choice -eq '3') { $effect = "triangulate" }

Clear-Host

Write-Host "`nLaunching $effect..." -ForegroundColor Green

$url = (Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest").assets | 
    Where-Object { $_.name -like "*$effect*.exe" } | 
    Select-Object -ExpandProperty browser_download_url -First 1

Invoke-WebRequest -Uri $url -OutFile "$effect.exe"
Unblock-File -Path ".\$effect.exe"
Start-Process ".\$effect.exe"
