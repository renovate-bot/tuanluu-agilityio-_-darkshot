# Chay thu dot 9 (ghim anh len man hinh) trong moi truong cach ly voi du lieu
# tong hop: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-pin.ps1 -Exe target\demo9\release\darkshot.exe
#   powershell -File scripts/demo-pin.ps1 -Exe target\demo9\release\darkshot.exe -Lang en
# Chup vung: Ctrl+Alt+Shift+F10 (hoac menu khay > Chup). Lich su mo san bang --history.
# Phim F10 (vk 121) de khong tranh voi demo-snap.ps1 (F9).
# Khong co khoa R2 nen Tai len se luu du phong vao thu muc demo (Pictures\darkshot).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-pin')
)
$ErrorActionPreference = 'Stop'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir $Lang
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
# Phim chup vung Ctrl+Alt+Shift+F10 (vk 121); hai phim toan man hinh tat.
$json = @"
{
  "hotkey": { "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 121 },
  "hotkey_enabled": true,
  "hotkey_save_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 134 },
  "hotkey_upload_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 133 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": true,
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
Write-Host 'Chup vung: Ctrl+Alt+Shift+F10'
# --history: mo san cua so Lich su (du lieu tong hop) lam cua so de thu ghim anh.
Start-Process -FilePath $Exe -ArgumentList '--history'
