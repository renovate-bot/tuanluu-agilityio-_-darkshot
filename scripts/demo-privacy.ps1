# Chay thu dot 21 (Privacy Shield: quet thong tin nhay cam truoc khi xuat anh) trong moi truong
# cach ly voi du lieu GIA: khong dung cau hinh, khoa R2 hay thu muc that cua ban, khong mang. Vi du:
#   powershell -File scripts/demo-privacy.ps1 -Exe target\release\darkshot.exe
#   powershell -File scripts/demo-privacy.ps1 -Exe target\release\darkshot.exe -Lang en
#   powershell -File scripts/demo-privacy.ps1 -Exe target\release\darkshot.exe -Off    (Shield tat, de so)
#   powershell -File scripts/demo-privacy.ps1 -NoLaunch                                (chi tao ho so)
# Ho so tam: privacy_shield bat (tru -Off), history_enabled bat, khong R2 (Tai len -> luu du phong vao
# Pictures\darkshot trong thu muc demo), khong kiem tra cap nhat.
# Phim: chup vung Ctrl+Alt+Shift+F10 (vk 121), luu toan man hinh Ctrl+Alt+Shift+F9 (vk 120),
# tai toan man hinh Ctrl+Alt+Shift+F8 (vk 119), lap lai lan chup truoc Ctrl+Alt+Shift+F11 (vk 122).
# Khong chay cung luc voi demo khac (cung phim F10).
# Tep mau Notepad co du lieu GIA cua ca 7 loai; chuoi trong nhu khoa that duoc ghep luc chay.
# Phien Chup tung buoc tao san 2 buoc chua quet (anh mau co du lieu gia) de thu Xuat.
param(
    [string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-privacy'),
    [switch]$Off,
    [switch]$NoLaunch
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not $NoLaunch -and -not $Exe) {
    Write-Host 'Can -Exe <duong dan darkshot.exe> (hoac -NoLaunch de chi tao ho so).'
    exit 1
}
$name = if ($Off) { "$Lang-off" } else { $Lang }
$profileDir = Join-Path $Dir $name
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$stepsDir = Join-Path $profileDir 'Local\darkshot\steps\current'
New-Item -ItemType Directory -Force $stepsDir | Out-Null
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
$shield = if ($Off) { 'false' } else { 'true' }
$json = @"
{
  "hotkey": { "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 121 },
  "hotkey_enabled": true,
  "hotkey_save_full": { "enabled": true, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 120 },
  "hotkey_upload_full": { "enabled": true, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 119 },
  "hotkey_repeat_last": { "enabled": true, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 122 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": true,
  "privacy_shield": $shield,
  "language": "$Lang",
  "save_dir": "$save"
}
"@
# UTF-8 khong BOM: serde_json khong doc duoc tep co BOM.
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, $utf8)

# ---- Du lieu GIA (ghep luc chay de cong cu quet bi mat khong bao nham) ----
function ConvertTo-B64Url([string]$s) {
    [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($s)).TrimEnd('=').Replace('+', '-').Replace('/', '_')
}
$aws = 'AK' + 'IA' + 'EXAMPLEKEY012345'
$gh = 'gh' + 'p_' + 'EXAMPLEexample0123456789EXAMPLEexamp'
$jwt = (ConvertTo-B64Url '{"alg":"HS256","typ":"JWT"}') + '.' + (ConvertTo-B64Url '{"sub":"demo","name":"Example User"}') + '.' + 'ZmFrZXNpZ25hdHVyZUVYQU1QTEU'
$pk = 'PRIVATE' + ' KEY'
$pemLines = @(
    "-----BEGIN $pk-----",
    'MIIEexampleEXAMPLEexampleEXAMPLEnotARealKey0123456789',
    'fakeFAKEfakeFAKEfakeFAKE0123456789abcdefABCDEF+/demo',
    "-----END $pk-----"
)
$sampleLines = @(
    'darkshot Privacy Shield demo - ALL DATA BELOW IS FAKE',
    '',
    'Email: demo.user@example.com',
    'Phone: +84 912 345 678   Hotline 0987 654 321',
    'Card: 4111 1111 1111 1111   exp 12/30',
    "AWS_ACCESS_KEY_ID=$aws",
    "token $gh",
    "Authorization: Bearer $jwt",
    'password: Example-Pass-123',
    'DB_PASSWORD=NotARealPassword9'
) + $pemLines + @(
    '',
    'Normal text (should not be flagged): build 1.2.3, 2026-11-22 10:30, 10.0.0.1, $1,234.56',
    'PWD=/home/demo   SECRETARY=alice   KEYBOARD=us'
)
$sample = Join-Path $profileDir 'privacy-sample.txt'
[System.IO.File]::WriteAllText($sample, ($sampleLines -join "`r`n"), $utf8)

# ---- Phien Chup tung buoc: 2 buoc chua quet (thieu "scanned" = false), anh co du lieu gia ----
Add-Type -AssemblyName System.Drawing
$stepText = @(
    @('Step 1: sign in as demo.user@example.com', 'password: Example-Pass-123'),
    @("Step 2: set AWS_ACCESS_KEY_ID=$aws", 'Card 4111 1111 1111 1111')
)
for ($i = 0; $i -lt $stepText.Count; $i++) {
    $bmp = [System.Drawing.Bitmap]::new(1000, 240)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::White)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::Black)
    $font = [System.Drawing.Font]::new('Segoe UI', 22)
    $g.DrawString($stepText[$i][0], $font, $brush, 30, 50)
    $g.DrawString($stepText[$i][1], $font, $brush, 30, 130)
    $file = Join-Path $stepsDir ('{0:000}.png' -f ($i + 1))
    $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
    $font.Dispose(); $brush.Dispose(); $g.Dispose(); $bmp.Dispose()
}
$session = '{"version":1,"steps":[{"file":"001.png","caption":"Sign in"},{"file":"002.png","caption":"Configure"}]}'
[System.IO.File]::WriteAllText((Join-Path $stepsDir 'session.json'), $session, $utf8)

Write-Host "Demo: $profileDir (privacy_shield=$shield, lang=$Lang)"
Write-Host "Tep mau: $sample"
if ($Off) {
    Write-Host 'Shield TAT (-Off): moi duong duoi day xuat ngay nhu truoc, KHONG co cua so Kiem tra (de so sanh):'
} else {
    Write-Host 'Thu tung duong (Shield bat: moi duong phai hien cua so Kiem tra truoc khi xuat):'
}
Write-Host '  1. Chup vung Ctrl+Alt+Shift+F10 quanh tep mau -> Luu / Chep / Tai len (khong R2: luu du phong) / Ghim.'
if (-not $Off) { Write-Host '  2. Huy trong cua so Kiem tra: khong co tep moi trong Pictures\darkshot, khong co muc Lich su.' }
Write-Host '  3. Lap lai lan chup truoc: Ctrl+Alt+Shift+F11. Toan man hinh: Ctrl+Alt+Shift+F9 (Luu), F8 (Tai len).'
Write-Host '  4. Chup cuon: trong lop phu bam Ctrl+L, cuon tep mau, roi Luu/Chep/Tai len/Ghim o cua so ket qua.'
Write-Host '  5. Chup tung buoc: phien co san 2 buoc chua quet -> Xuat (moi buoc co phat hien deu hoi); them buoc Ctrl+N trong lop phu.'
Write-Host '  6. Lich su (menu khay): Chep anh / Ghim mot muc -> luon quet lai.'
Write-Host '  7. Anh ghim: Chep/Luu tu menu anh ghim.'
if ($Off) {
    Write-Host '  So sanh: chay lai khong co -Off (Shield bat: cua so Kiem tra truoc moi lan xuat).'
} else {
    Write-Host '  So sanh: chay lai voi -Off (Shield tat: khong cua so, xuat ngay nhu truoc).'
}
if (-not $Off) { Write-Host '  Ghi GIF khong duoc quet (thanh ghi hien dong canh bao khi Shield bat).' }
if ($NoLaunch) { exit 0 }
$env:DARKSHOT_PROFILE_DIR = $profileDir
Start-Process -FilePath 'notepad.exe' -ArgumentList $sample
Start-Process -FilePath $Exe
