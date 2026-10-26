<#
.SYNOPSIS
The one elevated script of the outside instruments: the file-read trace (I1).

.DESCRIPTION
Developer-only. Run it elevated, behind one admin prompt:

    Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','<repo>\tools\quest_tracker\outside\fileio_controller.ps1','-Live','L'

It uses only Windows' built-in Performance Recorder (wpr, profile FileIO, file
mode). It traces the operating system's file I/O log and never opens the
game's process. In order, writing each state to L\i1\status.txt as
"<utc> <state>":

  1. started. If `wpr -status` shows a recording already running, it writes
     "error wpr busy" and exits without touching it.
  2. rehearsal-trace-running: `wpr -start FileIO -filemode -recordtempto L\i1\tmp`.
  3. probe-done: `py -3 -m tools.quest_tracker.outside.fileio probe --live L`,
     run from this checkout's root.
  4. rehearsal-stopped: `wpr -stop L\i1\rehearsal\rehearsal.etl`.
  5. session-trace-running: the session trace, started the same way.
  6. session-trace-stopped: once L\stop-i1 exists, `wpr -stop L\i1\session.etl`.

Any failure writes "error <reason>", cancels a recording this script started,
and exits 1. A session trace still running after -MaxMinutes (default 180) is
stopped and saved as if L\stop-i1 had appeared, after an
"error session cap ..." line, so a forgotten trace cannot fill the disk.

The live folder must lie outside every git checkout: the ETL names every
process's files and stays in the live folder.
#>
param(
    [Parameter(Mandatory = $true)][string]$Live,
    [int]$MaxMinutes = 180
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$Live = [System.IO.Path]::GetFullPath($Live)
$i1 = Join-Path $Live 'i1'
$rehearsal = Join-Path $i1 'rehearsal'
$tmp = Join-Path $i1 'tmp'
$status = Join-Path $i1 'status.txt'
$stopFile = Join-Path $Live 'stop-i1'
$script:recording = $false

function Write-Status([string]$state) {
    $line = [DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'") + ' ' + $state
    [System.IO.File]::AppendAllText($status, $line + "`n", [System.Text.Encoding]::ASCII)
    Write-Host $line
}

function Get-GitTree([string]$path) {
    # The nearest folder at or above $path holding a .git entry, as common.py's refusal finds it.
    $d = $path
    while ($d) {
        if (Test-Path -LiteralPath (Join-Path $d '.git')) { return $d }
        $parent = Split-Path -Parent $d
        if ($parent -eq $d) { break }
        $d = $parent
    }
    return $null
}

function Invoke-Wpr([string[]]$wprArgs) {
    # Windows PowerShell 5.1 turns a native command's redirected stderr into
    # errors, which 'Stop' would throw on; wpr's exit code is the verdict here.
    $ErrorActionPreference = 'Continue'
    $out = & wpr.exe @wprArgs 2>&1 | Out-String
    return @{ Code = $LASTEXITCODE; Text = (($out -split "`r?`n") | Where-Object { $_.Trim() }) -join ' ' }
}

function Fail([string]$reason) {
    if ($script:recording) {
        $null = Invoke-Wpr @('-cancel')
        $script:recording = $false
    }
    Write-Status ('error ' + $reason)
    exit 1
}

function Start-Trace {
    $r = Invoke-Wpr @('-start', 'FileIO', '-filemode', '-recordtempto', $tmp)
    if ($r.Code -ne 0) { Fail ('wpr -start exited ' + $r.Code + ': ' + $r.Text) }
    $script:recording = $true
}

function Stop-Trace([string]$etl) {
    $r = Invoke-Wpr @('-stop', $etl)
    $script:recording = $false
    if ($r.Code -ne 0) { Fail ('wpr -stop exited ' + $r.Code + ': ' + $r.Text) }
}

$tree = Get-GitTree $Live
if ($tree) {
    Write-Host ("refusing the live folder " + $Live + ": it is inside the git checkout " + $tree)
    exit 1
}
foreach ($d in @($i1, $rehearsal, $tmp)) {
    if (-not (Test-Path -LiteralPath $d)) { $null = New-Item -ItemType Directory -Path $d }
}

try {
    Write-Status 'started'
    $st = Invoke-Wpr @('-status')
    if ($st.Code -ne 0) { Fail ('wpr -status exited ' + $st.Code + ': ' + $st.Text) }
    if ($st.Text -notmatch 'WPR is not recording') { Fail 'wpr busy' }

    Start-Trace
    Write-Status 'rehearsal-trace-running'

    Push-Location -LiteralPath $repo
    try {
        & py.exe -3 -m tools.quest_tracker.outside.fileio probe --live $Live
        $probeCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    if ($probeCode -ne 0) { Fail ('probe exited ' + $probeCode) }
    Write-Status 'probe-done'

    Stop-Trace (Join-Path $rehearsal 'rehearsal.etl')
    Write-Status 'rehearsal-stopped'

    Start-Trace
    Write-Status 'session-trace-running'
    $deadline = [DateTime]::UtcNow.AddMinutes($MaxMinutes)
    while (-not (Test-Path -LiteralPath $stopFile)) {
        if ([DateTime]::UtcNow -gt $deadline) {
            Write-Status ('error session cap of ' + $MaxMinutes + ' minutes reached; stopping the trace')
            break
        }
        Start-Sleep -Milliseconds 500
    }
    Stop-Trace (Join-Path $i1 'session.etl')
    Write-Status 'session-trace-stopped'
    exit 0
} catch {
    Fail ($_.Exception.Message -replace '\s+', ' ')
}
