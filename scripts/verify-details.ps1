# Run in an interactive Windows terminal. Each case opens the actual TUI;
# perform the printed keys, then q. Exact index/worktree checks run on exit.
param(
    [Parameter(Mandatory = $true)][string]$Binary,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "../tmp/details-terminal"),
    [switch]$UseStagedDiscardKey
)
$ErrorActionPreference = 'Stop'
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$runDirectory = Join-Path $OutputDirectory (Get-Date -Format 'yyyyMMdd-HHmmss-fff')
[System.IO.Directory]::CreateDirectory($runDirectory) | Out-Null
$env:GIT_CONFIG_NOSYSTEM = '1'
$env:GIT_CONFIG_GLOBAL = Join-Path $runDirectory 'empty-global-config'
$encoding = [System.Text.UTF8Encoding]::new($false)

function Invoke-CaseGit([string[]]$GitArgs) {
    $result = & git @GitArgs
    if ($LASTEXITCODE -ne 0) { throw "git $GitArgs failed" }
    return $result
}
function Get-CaseContent([int[]]$ChangedLines) {
    return ((1..60 | ForEach-Object {
        if ($ChangedLines -contains $_) { 'CHANGED-{0:D2}' -f $_ }
        else { 'line-{0:D2}' -f $_ }
    }) -join "`n") + "`n"
}
function Write-CaseFile([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

$results = @()
foreach ($selection in @('single', 'multiple', 'visual')) {
    foreach ($operation in @('stage', 'unstage', 'discard')) {
        $case = "$selection-$operation"
        $repoDirectory = Join-Path $runDirectory $case
        [System.IO.Directory]::CreateDirectory($repoDirectory) | Out-Null
        Push-Location -LiteralPath $repoDirectory
        try {
            Invoke-CaseGit @('init', '-q', '-b', 'main') | Out-Null
            foreach ($setting in @(@('user.name', 'ratagit-tests'), @('user.email', 'tests@example.com'), @('core.autocrlf', 'false'), @('commit.gpgsign', 'false'))) {
                Invoke-CaseGit @('config', $setting[0], $setting[1]) | Out-Null
            }
            Write-CaseFile (Join-Path $repoDirectory 'a.txt') (Get-CaseContent @())
            Write-CaseFile (Join-Path $repoDirectory 'b.txt') "base`n"
            Invoke-CaseGit @('add', 'a.txt', 'b.txt') | Out-Null
            Invoke-CaseGit @('commit', '-qm', 'base') | Out-Null
            Write-CaseFile (Join-Path $repoDirectory 'a.txt') (Get-CaseContent @(5, 25, 45))
            if ($operation -eq 'unstage') { Invoke-CaseGit @('add', 'a.txt') | Out-Null }
            Write-CaseFile (Join-Path $repoDirectory 'b.txt') "staged sentinel`n"
            Invoke-CaseGit @('add', 'b.txt') | Out-Null
            $chosen = switch ($selection) {
                'single' { @(25) }
                'multiple' { @(5, 45) }
                'visual' { @(5, 25) }
            }
            $selectKeys = switch ($selection) { 'single' { 'j' } 'multiple' { 'x j j x' } 'visual' { 'v j' } }
            $applyKeys = if ($operation -eq 'discard') { 'd Enter' } elseif ($operation -eq 'unstage' -and $UseStagedDiscardKey) { 'd' } else { 'Space' }
            $keys = "Enter, $selectKeys, $applyKeys, q"
            Write-Host "CASE $case READY: $keys"
            & $Binary
            if ($LASTEXITCODE -ne 0) { throw "TUI exited with $LASTEXITCODE" }
            $rest = @(5, 25, 45) | Where-Object { $chosen -notcontains $_ }
            $expectedIndex = if ($operation -eq 'stage') { Get-CaseContent $chosen } elseif ($operation -eq 'unstage') { Get-CaseContent $rest } else { Get-CaseContent @() }
            $expectedWorktree = if ($operation -eq 'discard') { Get-CaseContent $rest } else { Get-CaseContent @(5, 25, 45) }
            $actualIndex = ((Invoke-CaseGit @('show', ':a.txt')) -join "`n") + "`n"
            $actualWorktree = [System.IO.File]::ReadAllText((Join-Path $repoDirectory 'a.txt'))
            if ($actualIndex -cne $expectedIndex) { throw "$case index mismatch" }
            if ($actualWorktree -cne $expectedWorktree) { throw "$case worktree mismatch" }
            if ((Invoke-CaseGit @('show', ':b.txt')) -cne 'staged sentinel') { throw "$case changed unrelated index entry" }
            $report = "PASS $case`nKeys: $keys`nIndex, worktree, unrelated staged sentinel: exact match`n"
            $report += (Invoke-CaseGit @('status', '--short')) -join "`n"
            $report += "`nINDEX DIFF`n" + ((Invoke-CaseGit @('diff', '--cached')) -join "`n")
            $report += "`nWORKTREE DIFF`n" + ((Invoke-CaseGit @('diff')) -join "`n")
            Write-CaseFile (Join-Path $repoDirectory 'verification.txt') $report
            $results += "PASS $case"
            Write-Host "PASS $case"
        } finally { Pop-Location }
    }
}
Write-CaseFile (Join-Path $runDirectory 'results.txt') ($results -join "`n")
Write-Host "ALL 9 CASES PASSED. Evidence: $runDirectory"
