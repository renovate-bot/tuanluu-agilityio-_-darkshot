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
$found = $false
$files = @(Get-ChildItem -Path 'crates' -Recurse -File) + @(Get-ChildItem -Path 'scripts' -File)
foreach ($f in $files) {
    $rel = $f.FullName.Substring((Get-Location).Path.Length + 1)
    if ($rel -like '*\target\*' -or $skip -contains $rel -or $f.Extension -in '.ico', '.png') { continue }
    $hits = Select-String -LiteralPath $f.FullName -Pattern $legacy -SimpleMatch
    foreach ($h in $hits) {
        Write-Host "CHECK FAILED: legacy name in ${rel}:$($h.LineNumber)"
        $found = $true
    }
}
if ($found) { exit 1 }

Write-Host 'ALL CHECKS PASSED'
exit 0
