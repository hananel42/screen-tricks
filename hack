$REPO = "hananel42/screen-tricks"


Write-Host "=================================" -ForegroundColor Cyan
Write-Host " Select a Screen Effect:" -ForegroundColor Cyan
Write-Host " [1] Particles"
Write-Host " [2] Wave"
Write-Host " [3] Triangulate"
Write-Host "=================================" -ForegroundColor Cyan

$key = [Console]::ReadKey($true)
$choice = $key.KeyChar

$effect = "particles"
if ($choice -eq '2') { $effect = "wave" }
if ($choice -eq '3') { $effect = "triangulate" }


$currentTop = [Console]::CursorTop
[Console]::SetCursorPosition(0, $currentTop - 6)


$host.UI.RawUI.ScrollBufferContents(
    (New-Object System.Management.Automation.Host.Rectangle(0, $currentTop - 6, [Console]::BufferWidth, $currentTop)),
    (New-Object System.Management.Automation.Host.Coordinates(0, $currentTop - 12)),
    (New-Object System.Management.Automation.Host.Rectangle(0, $currentTop - 6, [Console]::BufferWidth, $currentTop)),
    (New-Object System.Management.Automation.Host.BufferCell(' ', [Console]::ForegroundColor, [Console]::BackgroundColor, 'Complete'))
)


Write-Host "Launching $effect..." -ForegroundColor Green

$url = (Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest").assets |
    Where-Object { $_.name -match $effect } |
    Select-Object -ExpandProperty browser_download_url -First 1

Invoke-WebRequest -Uri $url -OutFile "$effect.exe"
Unblock-File -Path ".\$effect.exe"
Start-Process ".\$effect.exe"