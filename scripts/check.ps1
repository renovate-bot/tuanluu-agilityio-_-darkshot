# Cong kiem tra truoc khi gop nhanh. Thoat ma 0 va in "ALL CHECKS PASSED" khi moi thu qua.
$ErrorActionPreference = 'Continue'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Step([string]$name, [scriptblock]$cmd) {
    Write-Host "== $name"
    & $cmd
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CHECK FAILED: $name"
        exit 1
    }
}

Step 'cargo fmt --check' { cargo fmt --all --check }
Step 'cargo test' { cargo test --workspace }
Step 'cargo clippy' { cargo clippy --workspace --all-targets -- -D warnings }

Write-Host '== darkshot-core dependencies'
$tree = cargo tree -p darkshot-core -e normal --prefix none
if ($LASTEXITCODE -ne 0) {
    Write-Host 'CHECK FAILED: cargo tree'
    exit 1
}
$banned = $tree | Where-Object { $_ -match '^(windows|native-tls|schannel|rustls-platform-verifier) v' }
if ($banned) {
    Write-Host 'CHECK FAILED: darkshot-core depends on a banned crate:'
    $banned | Sort-Object -Unique | ForEach-Object { Write-Host "  $_" }
    exit 1
}

Write-Host '== no hard-coded Vietnamese strings'
# Dau tieng Viet viet bang \uXXXX de file nay giu thuan ASCII: chu co dau Latin (U+00C0-U+024F) va Latin mo rong (U+1E00-U+1EFF).
$viPattern = '"[^"]*[\u00C0-\u024F\u1E00-\u1EFF][^"]*"'
$hard = $false
$root = (Get-Location).Path
function Get-RelPath($f) { $f.FullName.Substring($root.Length + 1) }
# Dung khi #[cfg(test)] o dong $i gan vao "mod ..." (dong khong rong ke tiep).
function Test-TestMod($lines, [int]$i) {
    if ($lines[$i] -notmatch '#\[cfg\(test\)\]') { return $false }
    $j = $i + 1
    while ($j -lt $lines.Count -and $lines[$j].Trim() -eq '') { $j++ }
    return ($j -lt $lines.Count -and $lines[$j] -match '^\s*mod\s')
}
# Quet de quy moi file .rs trong crates\*\src (ke ca thu muc con).
$srcFiles = @(Get-ChildItem -Path 'crates' -Directory | ForEach-Object { Join-Path $_.FullName 'src' } |
        Where-Object { Test-Path -LiteralPath $_ } |
        ForEach-Object { Get-ChildItem -LiteralPath $_ -Recurse -File -Filter '*.rs' })
if ($srcFiles.Count -eq 0) {
    Write-Host 'CHECK FAILED: no source files found under crates\*\src'
    exit 1
}
$srcFiles | Where-Object { $_.Name -ne 'i18n.rs' } | ForEach-Object {
    $file = $_
    $lines = @(Get-Content -Encoding UTF8 -LiteralPath $file.FullName)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $lineNo = $i + 1
        $line = $lines[$i]
        if (Test-TestMod $lines $i) { break }
        $code = $line -replace '//.*$', ''
        if ($code -cmatch $viPattern) {
            Write-Host "CHECK FAILED: hard-coded string in $(Get-RelPath $file):$lineNo"
            $hard = $true
        }
    }
}
if ($hard) { exit 1 }

Write-Host '== privacy boundary'
# Ham tao PreparedOutput chi duoc goi trong loi privacy\ va privacy_gate.rs.
# Chi mien phan than cua khoi "#[cfg(test)] mod x { ... }"; "mod x;" khong mien gi.
# Dau "}" dong khoi la dong chi co "}" cung thut le voi dong mod (rustfmt bao dam, cargo fmt --check chay truoc).
# Dem ngoac khong dung duoc vi chuoi va ky tu trong kiem thu chua ngoac.
function Get-TestMask($lines) {
    $mask = New-Object bool[] $lines.Count
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if (-not (Test-TestMod $lines $i)) { continue }
        $j = $i + 1
        while ($lines[$j].Trim() -eq '') { $j++ }
        if ($lines[$j] -notmatch '^(\s*)mod\s+\w+\s*\{\s*$') { continue }
        $close = $Matches[1] + '}'
        $k = $j + 1
        while ($k -lt $lines.Count -and $lines[$k].TrimEnd() -ne $close) { $k++ }
        for ($m = $i; $m -le [Math]::Min($k, $lines.Count - 1); $m++) { $mask[$m] = $true }
        $i = $k
    }
    return , $mask
}
# Goi ham tao, doi ten (as), goi qua <PreparedOutput>:: hoac xuong dong sau "PreparedOutput::".
$prepPattern = 'PreparedOutput::(unscanned|no_findings|reviewed_none|redacted|skipped_after_error)\b|PreparedOutput\s+as\b|<\s*PreparedOutput\s*>\s*::|PreparedOutput\s*::\s*$'
$privDir = 'crates\darkshot-core\src\privacy\'
$gateFile = 'crates\darkshot-windows\src\privacy_gate.rs'
$leak = $false
foreach ($file in $srcFiles) {
    $rel = Get-RelPath $file
    if ($rel.StartsWith($privDir) -or $rel -eq $gateFile) { continue }
    $lines = @(Get-Content -Encoding UTF8 -LiteralPath $file.FullName)
    $mask = Get-TestMask $lines
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($mask[$i]) { continue }
        $code = $lines[$i] -replace '//.*$', ''
        if ($code -match $prepPattern) {
            Write-Host "CHECK FAILED: PreparedOutput constructor outside privacy gate in ${rel}:$($i + 1)"
            $leak = $true
        }
    }
}
if ($leak) { exit 1 }

Write-Host '== no logging in privacy'
# Ke ca trong mod tests: khong (e)print!/(e)println!/dbg! trong privacy\, privacy_gate.rs, review_window.rs.
$logPattern = '\b(e?println|e?print|dbg)!'
$reviewFile = 'crates\darkshot-windows\src\review_window.rs'
$logged = $false
foreach ($file in $srcFiles) {
    $rel = Get-RelPath $file
    if (-not ($rel.StartsWith($privDir) -or $rel -eq $gateFile -or $rel -eq $reviewFile)) { continue }
    foreach ($h in @(Select-String -LiteralPath $file.FullName -Pattern $logPattern)) {
        Write-Host "CHECK FAILED: logging in ${rel}:$($h.LineNumber)"
        $logged = $true
    }
}
if ($logged) { exit 1 }

Write-Host '== no legacy name'
$legacy = 'shot' + 'me'
$skip = @('crates\darkshot-windows\src\legacy.rs', 'crates\darkshot-core\src\migrate.rs')
# Tieu de muc chuyen du lieu duoc mien. Chu co dau dung [char] de file giu thuan ASCII.
# Muc duoc mien tu tieu de den tieu de ## ke tiep; dong co dia chi repo hoac duong dan thu muc du an cung duoc mien.
$chE = [char]0x1EC3
$tU = [char]0x1EEB
$dD = [char]0x0110
$oO = [char]0x1EE3
$exemptHeading = @{
    'README.md' = '## Chuy' + $chE + 'n t' + $tU + " $legacy"
    'docs\testing\manual-checklist.md' = '## ' + $dD + $oO + 't 4: chuy' + $chE + 'n t' + $tU + " $legacy"
}
$found = $false
$files = @(Get-ChildItem -Path 'crates' -Recurse -File) + @(Get-ChildItem -Path 'scripts' -File) + @(Get-Item -LiteralPath 'README.md') + @(Get-ChildItem -Path 'docs\testing' -File)
foreach ($f in $files) {
    $rel = $f.FullName.Substring((Get-Location).Path.Length + 1)
    if ($rel -like '*\target\*' -or $skip -contains $rel -or $f.Extension -in '.ico', '.png') { continue }
    if ($rel -eq 'README.md' -or $rel -like 'docs\testing\*') {
        $lines = @(Get-Content -LiteralPath $f.FullName -Encoding UTF8)
        $exempt = $false
        for ($i = 0; $i -lt $lines.Count; $i++) {
            $line = $lines[$i]
            if ($line -match '^## ') { $exempt = ($line.TrimEnd() -eq $exemptHeading[$rel]) }
            if ($exempt -or $line -match 'tuanluu-agilityio/|2026 - Tuan') { continue }
            if ($line -match $legacy) {
                Write-Host "CHECK FAILED: legacy name in ${rel}:$($i + 1)"
                $found = $true
            }
        }
        continue
    }
    $hits = Select-String -LiteralPath $f.FullName -Pattern $legacy -SimpleMatch
    foreach ($h in $hits) {
        Write-Host "CHECK FAILED: legacy name in ${rel}:$($h.LineNumber)"
        $found = $true
    }
}
if ($found) { exit 1 }

Write-Host 'ALL CHECKS PASSED'
exit 0
