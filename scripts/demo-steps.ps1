# Chay thu dot 12 (chup tung buoc) trong moi truong cach ly voi du lieu tong hop:
# khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-steps.ps1 -Exe target\demo12\release\darkshot.exe
#   powershell -File scripts/demo-steps.ps1 -Exe target\demo12\release\darkshot.exe -Lang en
# Script tao san mot phien 3 buoc (anh mau mau don gian + session.json) trong ho so cach ly
# <profile>\Local\darkshot\steps\current\. Khi darkshot khoi dong, thanh phien hien ngay voi "3 buoc".
# Chup them buoc: chup vung bang Ctrl+Alt+Shift+F10 (hoac menu khay > Chup tung buoc).
# Phim F10 (vk 121, nhu demo-ocr.ps1): khong chay hai demo cung luc.
# Khong co khoa R2 va khong mang. Chu ghi chu trong demo la tieng Anh khong dau (tep .ps1 ASCII).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-steps')
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir $Lang
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$stepsDir = Join-Path $profileDir 'Local\darkshot\steps\current'
New-Item -ItemType Directory -Force $stepsDir | Out-Null
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
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, $utf8)

# Ba anh mau: nen mau khac nhau, khung trang va chu "Step k".
Add-Type -AssemblyName System.Drawing
$fills = @(
    [System.Drawing.Color]::FromArgb(76, 141, 255),
    [System.Drawing.Color]::FromArgb(46, 160, 67),
    [System.Drawing.Color]::FromArgb(210, 153, 34)
)
$captions = @(
    'Open the settings window',
    'Pick the region to capture',
    'Check the result and export'
)
for ($i = 0; $i -lt 3; $i++) {
    $bmp = [System.Drawing.Bitmap]::new(640, 400)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear($fills[$i])
    $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::White, 6)
    $g.DrawRectangle($pen, 40, 40, 560, 320)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $font = [System.Drawing.Font]::new('Segoe UI', 48, [System.Drawing.FontStyle]::Bold)
    $g.DrawString(('Step {0}' -f ($i + 1)), $font, $brush, 220, 160)
    $file = Join-Path $stepsDir ('{0:000}.png' -f ($i + 1))
    $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
    $font.Dispose(); $brush.Dispose(); $pen.Dispose(); $g.Dispose(); $bmp.Dispose()
}

# session.json, dinh dang phien ban 1 (steps.rs, to_json): {"version":1,"steps":[{"file","caption"}...]}
$items = for ($i = 0; $i -lt $captions.Count; $i++) {
    '{{"file":"{0:000}.png","caption":"{1}"}}' -f ($i + 1), $captions[$i]
}
$session = '{"version":1,"steps":[' + ($items -join ',') + ']}'
[System.IO.File]::WriteAllText((Join-Path $stepsDir 'session.json'), $session, $utf8)

$env:DARKSHOT_PROFILE_DIR = $profileDir
Write-Host "Demo: $profileDir"
Write-Host 'Thanh phien hien ngay voi 3 buoc. Chup them: Ctrl+Alt+Shift+F10, roi Ctrl+N trong lop phu.'
Start-Process -FilePath $Exe
