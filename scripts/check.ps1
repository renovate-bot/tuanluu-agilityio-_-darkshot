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

Write-Host '== shotme-core dependencies'
$tree = cargo tree -p shotme-core -e normal --prefix none
if ($LASTEXITCODE -ne 0) {
    Write-Host 'CHECK FAILED: cargo tree'
    exit 1
}
$banned = $tree | Where-Object { $_ -match '^(windows|native-tls|schannel|rustls-platform-verifier) v' }
if ($banned) {
    Write-Host 'CHECK FAILED: shotme-core depends on a banned crate:'
    $banned | Sort-Object -Unique | ForEach-Object { Write-Host "  $_" }
    exit 1
}

Write-Host 'ALL CHECKS PASSED'
exit 0
