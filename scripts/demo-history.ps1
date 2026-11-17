# Chay thu Lich su anh chup trong moi truong cach ly voi du lieu tong hop (khong dung cau hinh,
# khoa R2 hay thu muc that cua ban). Vi du:
#   powershell -File scripts/demo-history.ps1 -Exe target\demo7\release\darkshot.exe
#   powershell -File scripts/demo-history.ps1 -Exe target\demo7\release\darkshot.exe -Lang en -State off
# -State: full (16 muc, bat), empty (bat, trong), off (tat, con muc cu), offempty (tat, trong).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [ValidateSet('vi', 'en')][string]$Lang = 'vi',
    [ValidateSet('full', 'empty', 'off', 'offempty')][string]$State = 'full',
    [string]$Dir = (Join-Path $env:TEMP 'darkshot-demo-history')
)
$ErrorActionPreference = 'Stop'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Set-Location (Split-Path $PSScriptRoot -Parent)
$profileDir = Join-Path $Dir "$Lang-$State"
if (Test-Path $profileDir) { Remove-Item -Recurse -Force $profileDir }
New-Item -ItemType Directory -Force (Join-Path $profileDir 'Roaming\darkshot') | Out-Null
$enabled = if ($State -in 'full', 'empty') { 'true' } else { 'false' }
# Phim tat lo de khong tranh voi ban dang chay; khong kiem tra cap nhat; thu muc luu nam trong thu muc demo.
$save = (Join-Path $profileDir 'Pictures\darkshot').Replace('\', '/')
$json = @"
{
  "hotkey_enabled": false,
  "hotkey_save_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 134 },
  "hotkey_upload_full": { "enabled": false, "ctrl": true, "alt": true, "shift": true, "win": false, "vk": 133 },
  "check_updates": false,
  "autostart": false,
  "history_enabled": $enabled,
  "language": "$Lang",
  "save_dir": "$save"
}
"@
# UTF-8 khong BOM: serde_json khong doc duoc tep co BOM.
[System.IO.File]::WriteAllText((Join-Path $profileDir 'Roaming\darkshot\config.json'), $json, (New-Object System.Text.UTF8Encoding($false)))
if ($State -in 'full', 'off') {
    $env:DARKSHOT_DEMO_DIR = $profileDir
    cargo test -p darkshot-windows -- --ignored make_demo_store | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Khong dung duoc du lieu demo' }
    Remove-Item Env:\DARKSHOT_DEMO_DIR
}
$env:DARKSHOT_PROFILE_DIR = $profileDir
Write-Host "Demo: $profileDir"
Start-Process -FilePath $Exe -ArgumentList '--history'
