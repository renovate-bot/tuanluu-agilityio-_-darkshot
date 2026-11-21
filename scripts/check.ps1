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
# Dung khi dong $i bat dau bang #[cfg(test)] (khong phai chu thich "// #[cfg(test)]" hay "/// ...")
# va gan vao "mod ..." hoac "pub(crate) mod ..." (dong khong rong ke tiep).
$modLine = '^(\s*)(pub(\([a-z]+\))?\s+)?mod\s'
function Test-TestMod($lines, [int]$i) {
    if ($lines[$i] -notmatch '^\s*#\[cfg\(test\)\]') { return $false }
    $j = $i + 1
    while ($j -lt $lines.Count -and $lines[$j].Trim() -eq '') { $j++ }
    return ($j -lt $lines.Count -and $lines[$j] -match $modLine)
}
# Tach mot file Rust thanh hai ban moi dong, mang trang thai chuoi qua cac dong:
#   Code: bo chu thich // (ca /// va //!) nhung GIU noi dung chuoi ("https://..." khong bi cat);
#   Bare: bo noi dung chuoi truoc ("..." -> "", 'x' -> ''), roi moi bo chu thich.
# Nhan ra chuoi thuong (co \ thoat), chuoi tho r"..", r#".."#, b"..", br"..", ky tu 'x', '\n', '\u{..}'
# va lifetime 'a. Chu thich khoi /* ... */ (nhieu dong, long nhau) bi bo khoi ca hai ban.
# Da biet (chua xu ly): chuoi C cr".." / c".." va macro tu viet (macro_rules!) co the lach cac luat grep.
function Split-Rust([string[]]$lines) {
    $code = New-Object string[] $lines.Count
    $bare = New-Object string[] $lines.Count
    $q = [char]34
    $ap = [char]39
    $bs = [char]92
    $inStr = $false
    $hashes = -1
    $block = 0
    $special = [char[]]@($q, $ap, [char]47)
    for ($n = 0; $n -lt $lines.Count; $n++) {
        $line = $lines[$n]
        if (-not $inStr -and $block -eq 0 -and $line.IndexOfAny($special) -lt 0) {
            $code[$n] = $line
            $bare[$n] = $line
            continue
        }
        $c = New-Object System.Text.StringBuilder
        $b = New-Object System.Text.StringBuilder
        $len = $line.Length
        $i = 0
        while ($i -lt $len) {
            $ch = $line[$i]
            $two = if (($i + 1) -lt $len) { $line.Substring($i, 2) } else { '' }
            if ($block -gt 0) {
                if ($two -eq '*/') { $block--; $i += 2 }
                elseif ($two -eq '/*') { $block++; $i += 2 }
                else { $i++ }
                continue
            }
            if ($inStr) {
                if ($hashes -lt 0 -and $ch -eq $bs) {
                    [void]$c.Append($line, $i, [Math]::Min(2, $len - $i))
                    $i += 2
                    continue
                }
                if ($ch -eq $q) {
                    $k = 0
                    while ($k -lt $hashes -and ($i + 1 + $k) -lt $len -and $line[$i + 1 + $k] -eq '#') { $k++ }
                    if ($hashes -le 0 -or $k -eq $hashes) {
                        $close = $line.Substring($i, 1 + [Math]::Max(0, $hashes))
                        [void]$c.Append($close)
                        [void]$b.Append($close)
                        $i += $close.Length
                        $inStr = $false
                        continue
                    }
                }
                [void]$c.Append($ch)
                $i++
                continue
            }
            if ($two -eq '//') { break }
            if ($two -eq '/*') {
                $block = 1
                [void]$c.Append(' ')
                [void]$b.Append(' ')
                $i += 2
                continue
            }
            # Chuoi tho r"..", r#".."#, br"..": 'r' dung dau tu (hoac sau 'b' dung dau tu).
            if ($ch -ceq 'r') {
                $startOk = ($i -eq 0) -or ($line[$i - 1] -notmatch '[A-Za-z0-9_]') -or
                    ($line[$i - 1] -ceq 'b' -and ($i -eq 1 -or $line[$i - 2] -notmatch '[A-Za-z0-9_]'))
                $j = $i + 1
                while ($j -lt $len -and $line[$j] -eq '#') { $j++ }
                if ($startOk -and $j -lt $len -and $line[$j] -eq $q) {
                    $open = $line.Substring($i, $j - $i + 1)
                    [void]$c.Append($open)
                    [void]$b.Append($open)
                    $hashes = $j - $i - 1
                    $inStr = $true
                    $i = $j + 1
                    continue
                }
            }
            if ($ch -eq $q) {
                [void]$c.Append($ch)
                [void]$b.Append($ch)
                $hashes = -1
                $inStr = $true
                $i++
                continue
            }
            if ($ch -eq $ap) {
                $end = -1
                if (($i + 1) -lt $len -and $line[$i + 1] -eq $bs) {
                    $end = $line.IndexOf($ap, [Math]::Min($i + 3, $len))
                } elseif (($i + 2) -lt $len -and $line[$i + 2] -eq $ap) {
                    $end = $i + 2
                } elseif (($i + 3) -lt $len -and [char]::IsHighSurrogate($line[$i + 1]) -and $line[$i + 3] -eq $ap) {
                    $end = $i + 3
                }
                if ($end -gt $i) {
                    [void]$c.Append($line, $i, $end - $i + 1)
                    [void]$b.Append([string]$ap + [string]$ap)
                    $i = $end + 1
                    continue
                }
            }
            [void]$c.Append($ch)
            [void]$b.Append($ch)
            $i++
        }
        $code[$n] = $c.ToString()
        $bare[$n] = $b.ToString()
    }
    return @{ Code = $code; Bare = $bare }
}
# Moi file nguon doc va tach mot lan, dung chung cho cac luat duoi.
$parsed = @{}
function Get-Parsed($file) {
    $key = $file.FullName
    if (-not $parsed.ContainsKey($key)) {
        $lines = @(Get-Content -Encoding UTF8 -LiteralPath $key)
        $sp = Split-Rust $lines
        $parsed[$key] = @{ Lines = $lines; Code = $sp.Code; Bare = $sp.Bare }
    }
    return $parsed[$key]
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
    $p = Get-Parsed $file
    $lines = $p.Lines
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $lineNo = $i + 1
        if (Test-TestMod $lines $i) { break }
        if ($p.Code[$i] -cmatch $viPattern) {
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
        if ($lines[$j] -notmatch '^(\s*)(pub(\([a-z]+\))?\s+)?mod\s+\w+\s*\{\s*$') { continue }
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
# Ngoai privacy\ (ke ca privacy_gate.rs): khong ten khac cho kieu (type X = PreparedOutput), khong
# impl trait cho no (impl From<Image> for PreparedOutput se la ham tao vong qua cong).
$prepAlias = '\btype\s+\w+(\s*<[^=]*>)?\s*=\s*(\w+\s*::\s*)*PreparedOutput\b|\bimpl\b.*\bfor\s+(\w+\s*::\s*)*PreparedOutput\b'
$leak = $false
foreach ($file in $srcFiles) {
    $rel = Get-RelPath $file
    if ($rel.StartsWith($privDir)) { continue }
    $p = Get-Parsed $file
    $mask = Get-TestMask $p.Lines
    for ($i = 0; $i -lt $p.Lines.Count; $i++) {
        if ($mask[$i]) { continue }
        $code = $p.Bare[$i]
        if ($rel -ne $gateFile -and $code -match $prepPattern) {
            Write-Host "CHECK FAILED: PreparedOutput constructor outside privacy gate in ${rel}:$($i + 1)"
            $leak = $true
        }
        if ($code -match $prepAlias) {
            Write-Host "CHECK FAILED: PreparedOutput alias or trait impl outside privacy in ${rel}:$($i + 1)"
            $leak = $true
        }
    }
}
if ($leak) { exit 1 }

Write-Host '== raw image sinks'
# Loi goi dua anh ra ngoai (clipboard, tep, R2, Lich su) chi o cac tep da ra soat; o do anh lay tu
# PreparedOutput (main.rs: run_on_image / gate_output / save_pending_uploads; in ra duoi day de
# nguoi ra soat doc). history_window.rs chi trong fn copy_prepared (nhanh dispatch cua cong) va
# kiem thu. Tep khac: cam ca trong kiem thu.
# Ca loi goi dang ham (Clipboard::set_image(c, i)) va ten dung khong kem "(" (let f = run_action;).
$sinkPattern = '\b(set_image|run_action|spawn_upload|record_outcome(_arc)?|encode_for_upload)\b'
$sinkDefinition = '\bfn\s+(set_image|run_action|spawn_upload|record_outcome(_arc)?|encode_for_upload)\b'
# Dau mot ham: pub / pub(crate) / pub(in path), const, async, unsafe, extern "abi", roi fn ten.
$fnHead = '^\s*(pub(\s*\([^)]*\))?\s+)?((const|async|unsafe)\s+|extern\s+("[^"]*"\s+)?)*fn\s+(\w+)'
$sinkFiles = @(
    'crates\darkshot-core\src\actions.rs',
    'crates\darkshot-core\src\image.rs',
    'crates\darkshot-windows\src\clipboard.rs',
    'crates\darkshot-windows\src\app.rs',
    'crates\darkshot-windows\src\history_service.rs',
    'crates\darkshot-windows\src\main.rs'
)
$historyWindow = 'crates\darkshot-windows\src\history_window.rs'
$mainFile = 'crates\darkshot-windows\src\main.rs'
$sinkBad = $false
foreach ($file in $srcFiles) {
    $rel = Get-RelPath $file
    $p = Get-Parsed $file
    $mask = Get-TestMask $p.Lines
    $fn = ''
    for ($i = 0; $i -lt $p.Lines.Count; $i++) {
        $code = $p.Bare[$i]
        if ($code -match $fnHead) { $fn = $Matches[6] }
        if ($code -notmatch $sinkPattern -or $code -match $sinkDefinition) { continue }
        if ($sinkFiles -contains $rel) {
            if ($rel -eq $mainFile -and -not $mask[$i]) { Write-Host "  main.rs:$($i + 1) in fn ${fn}: $($code.Trim())" }
            continue
        }
        if ($rel -eq $historyWindow -and ($fn -eq 'copy_prepared' -or $mask[$i])) { continue }
        Write-Host "CHECK FAILED: raw image sink outside reviewed files in ${rel}:$($i + 1)"
        $sinkBad = $true
    }
}
if ($sinkBad) { exit 1 }

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
