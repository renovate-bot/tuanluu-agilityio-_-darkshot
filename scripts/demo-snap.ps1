# Chay thu dot 8 (bat cua so khi chon vung, Lich su da gia co) trong moi truong cach ly voi du lieu
# tong hop: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-snap.ps1 -Exe target\demo8\release\darkshot.exe
#   powershell -File scripts/demo-snap.ps1 -Exe target\demo8\release\darkshot.exe -Lang en -Snap off
# Chup vung: Ctrl+Alt+Shift+F9 (hoac menu khay > Chup). Mo Lich su: menu khay > Lich su.
# Khong co khoa R2 nen Tai len se luu du phong vao thu muc demo (Pictures\darkshot).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [ValidateSet('on', 'off')][string]$Snap = 'on',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-snap')
)
$ErrorActionPreference = 'Stop'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir "$Lang-$Snap"
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$snapJson = if ($Snap -eq 'on') { 'true' } else { 'false' }
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
# Phim chup vung Ctrl+Alt+Shift+F9 (vk 120) de khong tranh voi ban dang chay; hai phim toan man hinh tat.
$json = @"
{
  "hotkey": { "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 120 },
  "hotkey_enabled": true,
  "hotkey_save_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 134 },
  "hotkey_upload_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 133 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": true,
  "snap_windows": $snapJson,
  "language": "$Lang",
  "save_dir": "$save"
}
"@
# UTF-8 khong BOM: serde_json khong doc duoc tep co BOM.
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, (New-Object System.Text.UTF8Encoding($false)))
$env:DARKSHOT_DEMO_DIR = $profileDir
cargo test -p darkshot-windows -- --ignored make_demo_store | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Khong dung duoc du lieu demo' }
Remove-Item Env:\DARKSHOT_DEMO_DIR
$env:DARKSHOT_PROFILE_DIR = $profileDir
Write-Host "Demo: $profileDir"
Write-Host 'Chup vung: Ctrl+Alt+Shift+F9'
# --history: mo san cua so Lich su (du lieu tong hop) lam cua so de thu bat khung.
Start-Process -FilePath $Exe -ArgumentList '--history'
