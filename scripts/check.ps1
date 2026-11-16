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
Get-ChildItem -Path 'crates\*\src\*.rs' | Where-Object { $_.Name -ne 'i18n.rs' } | ForEach-Object {
    $file = $_
    $lines = @(Get-Content -Encoding UTF8 -LiteralPath $file.FullName)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $lineNo = $i + 1
        $line = $lines[$i]
        if ($line -match '#\[cfg\(test\)\]') {
            # Chi dung khi #[cfg(test)] gan vao "mod ..." (dong khong rong ke tiep).
            $j = $i + 1
            while ($j -lt $lines.Count -and $lines[$j].Trim() -eq '') { $j++ }
            if ($j -lt $lines.Count -and $lines[$j] -match '^\s*mod\s') { break }
            continue
        }
        $code = $line -replace '//.*$', ''
        if ($code -cmatch $viPattern) {
            Write-Host "CHECK FAILED: hard-coded string in $($file.Name):$lineNo"
            $hard = $true
        }
    }
}
if ($hard) { exit 1 }

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
