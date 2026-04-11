[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Paths,

    [string]$BaseRef = "",

    [switch]$Staged
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required"
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if (-not $repoRoot) {
    throw "unable to determine repository root"
}

Push-Location $repoRoot
try {
    $modeArgs = @()
    $label = "worktree"

    if ($Staged) {
        $modeArgs += "--cached"
        $label = "staged"
    } elseif ($BaseRef) {
        $modeArgs += $BaseRef
        $label = "compare:$BaseRef"
    }

    Write-Host "Scoped change report ($label)" -ForegroundColor Cyan
    Write-Host "Paths:" -ForegroundColor Cyan
    foreach ($path in $Paths) {
        Write-Host "  $path"
    }
    Write-Host ""

    $nameOnly = @("diff") + $modeArgs + @("--name-only", "--") + $Paths
    $statArgs = @("diff") + $modeArgs + @("--stat", "--") + $Paths
    $summaryArgs = @("diff") + $modeArgs + @("--summary", "--") + $Paths
    $untrackedArgs = @("ls-files", "--others", "--exclude-standard", "--") + $Paths

    Write-Host "Changed files" -ForegroundColor Yellow
    & git @nameOnly
    Write-Host ""

    Write-Host "Untracked files" -ForegroundColor Yellow
    & git @untrackedArgs
    Write-Host ""

    Write-Host "Diff stat" -ForegroundColor Yellow
    & git @statArgs
    Write-Host ""

    Write-Host "Change summary" -ForegroundColor Yellow
    & git @summaryArgs
}
finally {
    Pop-Location
}
