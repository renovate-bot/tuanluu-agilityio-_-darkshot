# Chay thu dot 10 (lap lai lan chup vung truoc) trong moi truong cach ly voi du lieu
# tong hop: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-repeat.ps1 -Exe target\demo10\release\darkshot.exe
#   powershell -File scripts/demo-repeat.ps1 -Exe target\demo10\release\darkshot.exe -Lang en
# Chup vung: Ctrl+Alt+Shift+F10 (hoac menu khay > Chup man hinh).
# Lap lai: Ctrl+Alt+Shift+F11 (hoac menu khay > Chup lai vung truoc).
# Phim F10 (vk 121, nhu demo-pin.ps1) va F11 (vk 122). Dung chung F10 voi demo-pin.ps1: khong chay hai demo cung luc.
# Tep last_capture.json tong hop: mot lan chup vung (200, 200, 640 x 360, hanh dong Luu)
# de phim lap lai chay ngay, khong can chup truoc.
# Khong co khoa R2 nen Tai len se luu du phong vao thu muc demo (Pictures\darkshot).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-repeat')
)
$ErrorActionPreference = 'Stop'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir $Lang
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
# Phim chup vung Ctrl+Alt+Shift+F10 (vk 121) bat; phim lap lai Ctrl+Alt+Shift+F11 (vk 122) bat;
# hai phim toan man hinh tat.
$json = @"
{
  "hotkey": { "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 121 },
  "hotkey_enabled": true,
  "hotkey_save_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 134 },
  "hotkey_upload_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 133 },
  "hotkey_repeat_last": { "enabled": true, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 122 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": true,
  "language": "$Lang",
  "save_dir": "$save"
}
"@
# UTF-8 khong BOM: serde_json khong doc duoc tep co BOM.
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, (New-Object System.Text.UTF8Encoding($false)))
# Lan chup vung tong hop: version 1, vung 640 x 360 tai (200, 200), hanh dong Luu.
$last = '{"version":1,"x":200,"y":200,"w":640,"h":360,"action":"save"}'
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\last_capture.json'), $last, (New-Object System.Text.UTF8Encoding($false)))
$env:DARKSHOT_DEMO_DIR = $profileDir
cargo test -p darkshot-windows -- --ignored make_demo_store | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Khong dung duoc du lieu demo' }
Remove-Item Env:\DARKSHOT_DEMO_DIR
$env:DARKSHOT_PROFILE_DIR = $profileDir
Write-Host "Demo: $profileDir"
Write-Host 'Chup vung: Ctrl+Alt+Shift+F10. Chup lai vung truoc: Ctrl+Alt+Shift+F11'
# --history: mo san cua so Lich su (du lieu tong hop) lam cua so de thu.
Start-Process -FilePath $Exe -ArgumentList '--history'
