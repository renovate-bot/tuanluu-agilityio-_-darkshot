# Chay thu dot 13 (chup cuon) trong moi truong cach ly voi du lieu
# tong hop: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-scroll.ps1 -Exe target\demo13\release\darkshot.exe
#   powershell -File scripts/demo-scroll.ps1 -Exe target\demo13\release\darkshot.exe -Lang en
# Chup vung: Ctrl+Alt+Shift+F10 (hoac menu khay > Chup man hinh).
# Chup cuon: chon vung bao quanh tep mau (Notepad mo san), roi bam Ctrl+L (hoac nut Chup cuon),
# roi tu cuon xuong tu tu trong vung do (con lan, thanh cuon hoac phim). Chi cuon doc;
# darkshot khong tu cuon ung dung khac.
# Phim F10 (vk 121, nhu demo-pin.ps1 va demo-ocr.ps1): khong chay hai demo cung luc.
# Khong co khoa R2 va khong mang: Tai len se luu du phong vao thu muc demo (Pictures\darkshot).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-scroll')
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
# Tep mau nhieu dong, moi dong khac nhau (so dong va chu tong hop) de thay ro khi cuon.
$sample = Join-Path $profileDir 'scroll-sample.txt'
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add('darkshot demo chup cuon: du lieu tong hop')
$lines.Add('')
for ($i = 1; $i -le 160; $i++) {
    $lines.Add(('Dong {0:000} - Du lieu mau so {1} cho thu chup cuon, khong phai thong tin that.' -f $i, ($i * 37 % 1000)))
}
$lines.Add('')
$lines.Add('Het tep mau. Cam on ban da thu.')
[System.IO.File]::WriteAllText($sample, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding($false)))
$env:DARKSHOT_PROFILE_DIR = $profileDir
Start-Process -FilePath 'notepad.exe' -ArgumentList $sample
Write-Host "Demo: $profileDir"
Write-Host 'Chup vung: Ctrl+Alt+Shift+F10 (chon vung bao quanh tep mau trong Notepad). Chup cuon: Ctrl+L trong lop phu, roi cuon xuong tu tu.'
Start-Process -FilePath $Exe
