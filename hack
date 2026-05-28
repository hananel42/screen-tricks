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



$pos = $Host.UI.RawUI.CursorPosition
$pos.Y = [Math]::Max(0, $pos.Y - 6)
$pos.X = 0
$Host.UI.RawUI.CursorPosition = $pos

for ($i = 0; $i -lt 6; $i++) {
    Write-Host (" " * $Host.UI.RawUI.WindowSize.Width) -NoNewline
}
$Host.UI.RawUI.CursorPosition = $pos



Write-Host "Launching $effect..." -ForegroundColor Green

$url = (Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest").assets |
    Where-Object { $_.name -match $effect } |
    Select-Object -ExpandProperty browser_download_url -First 1

Invoke-WebRequest -Uri $url -OutFile "$effect.exe"
Unblock-File -Path ".\$effect.exe"
Start-Process ".\$effect.exe"