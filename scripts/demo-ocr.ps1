# Chay thu dot 11 (nhan dang chu cuc bo) trong moi truong cach ly voi du lieu
# tong hop: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-ocr.ps1 -Exe target\demo11\release\darkshot.exe
#   powershell -File scripts/demo-ocr.ps1 -Exe target\demo11\release\darkshot.exe -Lang en
# Chup vung: Ctrl+Alt+Shift+F10 (hoac menu khay > Chup man hinh).
# Nhan dang chu: chon vung tren file mau (Notepad mo san), roi bam Ctrl+T (hoac nut Nhan dang chu).
# Phim F10 (vk 121, nhu demo-pin.ps1 va demo-repeat.ps1): khong chay hai demo cung luc.
# Khong co khoa R2 va khong mang: Tai len se luu du phong vao thu muc demo (Pictures\darkshot).
# Ngon ngu nhan dang theo goi OCR cua Windows: may chua co goi tieng Viet thi tieng Viet khong nhan duoc.
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-ocr')
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir $Lang
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
# Phim chup vung Ctrl+Alt+Shift+F10 (vk 121) bat; cac phim con lai tat.
$json = @"
{
  "hotkey": { "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 121 },
  "hotkey_enabled": true,
  "hotkey_save_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 134 },
  "hotkey_upload_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 133 },
  "hotkey_repeat_last": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 122 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": false,
  "language": "$Lang",
  "save_dir": "$save"
}
"@
# UTF-8 khong BOM: serde_json khong doc duoc tep co BOM.
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, (New-Object System.Text.UTF8Encoding($false)))
# File mau co chu de nhan dang (Notepad mo san).
$sample = Join-Path $profileDir 'ocr-sample.txt'
$text = @"
Hello darkshot 123
Nhan dang chu cuc bo tren may.
"@
[System.IO.File]::WriteAllText($sample, $text, (New-Object System.Text.UTF8Encoding($false)))
$env:DARKSHOT_PROFILE_DIR = $profileDir
Start-Process -FilePath 'notepad.exe' -ArgumentList $sample
Write-Host "Demo: $profileDir"
Write-Host 'Chup vung: Ctrl+Alt+Shift+F10 (chon vung chu trong Notepad). Nhan dang: Ctrl+T trong lop phu.'
Start-Process -FilePath $Exe
