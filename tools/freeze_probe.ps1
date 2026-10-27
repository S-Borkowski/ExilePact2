<#
.SYNOPSIS
    Records what the machine is doing while the game stops responding.

.DESCRIPTION
    Written in the sister project this tooling came from
    (hero-siege-offline-toolkit): there an in-plugin stall watchdog showed the
    game's frame thread blocked in kernel waits and GPU present calls -
    ZwWaitForSingleObject, NtDxgkSubmitPresentToHwQueue,
    NtGdiDdDDIGetDeviceState - with no sample ever landing in plugin code, and
    the freeze reproduced with the plugin removed entirely.  So the cause was
    outside the mod, and measuring it needed a probe outside the game process.
    Nothing here is specific to either game: point -ProcessName at the client.

    This samples once a second and marks every window where the game's main
    window stops responding, recording for that window:
      - the game's own disk reads/writes (a save-load or AV-scan storm)
      - system free RAM (paging pressure)
      - which processes burned the most CPU, Defender (MsMpEng) called out by
        name because a real-time scan of a freshly patched large executable
        is a textbook whole-machine stall

    Read the summary printed at the end: whatever is busy during the
    not-responding window is the thing to chase.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tools\freeze_probe.ps1
    Then launch the game and reproduce the freeze.  Ctrl+C to stop.
#>
[CmdletBinding()]
param(
    # Wildcards allowed: Path of Exile 2 ships as PathOfExile.exe,
    # PathOfExile_x64.exe or a *Steam.exe variant.
    [string] $ProcessName = 'PathOfExile*',
    [string] $LogPath     = "$PSScriptRoot\..\freeze_probe.csv",
    [int]    $IntervalMs  = 1000,
    # 0 = run until the game exits or Ctrl+C.  A bound is handy for a smoke test.
    [int]    $MaxSeconds  = 0
)

$ErrorActionPreference = 'Stop'

# Get-CimInstance for IO counters and free memory costs seconds per sample, which
# is far too slow to see inside a freeze - and the probe would then be part of the
# load it is trying to measure.  These two calls are microseconds.
Add-Type -Namespace FreezeProbe -Name Native -MemberDefinition @'
    [StructLayout(LayoutKind.Sequential)]
    public struct IO_COUNTERS {
        public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount;
        public ulong ReadTransferCount,  WriteTransferCount,  OtherTransferCount;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct MEMORYSTATUSEX {
        public uint dwLength, dwMemoryLoad;
        public ulong ullTotalPhys, ullAvailPhys, ullTotalPageFile, ullAvailPageFile;
        public ulong ullTotalVirtual, ullAvailVirtual, ullAvailExtendedVirtual;
    }
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool GetProcessIoCounters(IntPtr hProcess, out IO_COUNTERS c);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool GlobalMemoryStatusEx(ref MEMORYSTATUSEX s);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool GetSystemTimes(out long idle, out long kernel, out long user);
'@

# System-wide busy/idle is the discriminator that matters: a frozen game on an
# otherwise IDLE machine is waiting on something (GPU, a lock); a frozen game on
# a BUSY machine is a victim of whatever is eating the box (AV scan, paging).
function Get-CpuSnapshot {
    $i = 0L; $k = 0L; $u = 0L
    if ([FreezeProbe.Native]::GetSystemTimes([ref]$i, [ref]$k, [ref]$u)) {
        [pscustomobject]@{ Idle = $i; Busy = ($k + $u) }   # kernel time already includes idle
    } else { $null }
}

function Get-SystemFreeMb {
    $m = New-Object FreezeProbe.Native+MEMORYSTATUSEX
    $m.dwLength = [System.Runtime.InteropServices.Marshal]::SizeOf($m)
    if ([FreezeProbe.Native]::GlobalMemoryStatusEx([ref]$m)) { [int]($m.ullAvailPhys / 1MB) } else { -1 }
}

Write-Host "Waiting for $ProcessName ..." -ForegroundColor Cyan
$proc = $null
while (-not $proc) {
    $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $proc) { Start-Sleep -Milliseconds 500 }
}
Write-Host "Attached to PID $($proc.Id). Reproduce the freeze; Ctrl+C when done." -ForegroundColor Green
$totalRam = New-Object FreezeProbe.Native+MEMORYSTATUSEX
$totalRam.dwLength = [System.Runtime.InteropServices.Marshal]::SizeOf($totalRam)
if ([FreezeProbe.Native]::GlobalMemoryStatusEx([ref]$totalRam)) {
    Write-Host ("Machine has {0} MB RAM total, {1} MB free right now." -f `
        [int]($totalRam.ullTotalPhys / 1MB), [int]($totalRam.ullAvailPhys / 1MB)) -ForegroundColor DarkGray
}
Write-Host "Logging to $LogPath" -ForegroundColor DarkGray

$prevRead  = 0L
$prevWrite = 0L
$prevCpu   = Get-CpuSnapshot
$rows      = New-Object System.Collections.Generic.List[object]
$wasFrozen = $false
$culprits  = New-Object System.Collections.Generic.List[string]

# Ranking every process costs ~1.5 s per sample - too slow to run continuously,
# and the probe would become part of the load.  So it runs ONCE per freeze, while
# the freeze is happening (they last tens of seconds; there is time).
function Get-TopCpuProcesses {
    try {
        (Get-Counter '\Process(*)\% Processor Time' -ErrorAction Stop).CounterSamples |
            Where-Object { $_.InstanceName -notin @('_total', 'idle') -and $_.CookedValue -gt 5 } |
            Sort-Object CookedValue -Descending | Select-Object -First 5 |
            ForEach-Object { "{0}:{1:N0}%" -f $_.InstanceName, $_.CookedValue }
    } catch { @() }
}

$deadline = if ($MaxSeconds -gt 0) { (Get-Date).AddSeconds($MaxSeconds) } else { [datetime]::MaxValue }

try {
    while ((Get-Date) -lt $deadline) {
        $proc.Refresh()
        if ($proc.HasExited) { Write-Host "Game exited." -ForegroundColor Yellow; break }

        # Responding is the same flag Explorer uses for "(Not Responding)": the
        # window stopped pumping messages, which is exactly the user-visible freeze.
        $responding = $true
        try { $responding = $proc.Responding } catch {}

        $read = 0L; $write = 0L
        $ioc = New-Object FreezeProbe.Native+IO_COUNTERS
        if ([FreezeProbe.Native]::GetProcessIoCounters($proc.Handle, [ref]$ioc)) {
            $read  = [int64]$ioc.ReadTransferCount
            $write = [int64]$ioc.WriteTransferCount
        }

        $cpu = Get-CpuSnapshot
        $sysBusyPct = -1
        if ($cpu -and $prevCpu) {
            $dIdle = $cpu.Idle - $prevCpu.Idle
            $dBusy = $cpu.Busy - $prevCpu.Busy      # kernel time includes idle
            if ($dBusy -gt 0) { $sysBusyPct = [int]((1 - ($dIdle / $dBusy)) * 100) }
        }
        $prevCpu = $cpu

        $row = [pscustomobject]@{
            Time        = (Get-Date).ToString('HH:mm:ss.fff')
            Responding  = $responding
            ReadKB      = if ($prevRead)  { [int](($read  - $prevRead)  / 1KB) } else { 0 }
            WriteKB     = if ($prevWrite) { [int](($write - $prevWrite) / 1KB) } else { 0 }
            GameWsMb    = [int]($proc.WorkingSet64 / 1MB)
            FreeRamMb   = Get-SystemFreeMb
            SysBusyPct  = $sysBusyPct
        }
        $rows.Add($row) | Out-Null
        $prevRead = $read; $prevWrite = $write

        if (-not $responding) {
            Write-Host ("[{0}] NOT RESPONDING  read {1} KB  write {2} KB  freeRAM {3} MB  cpu {4}%" -f `
                $row.Time, $row.ReadKB, $row.WriteKB, $row.FreeRamMb, $row.SysBusyPct) -ForegroundColor Red
            if (-not $wasFrozen) {
                # First sample of this freeze: pay for the process ranking once.
                $top = Get-TopCpuProcesses
                if ($top) {
                    $line = "busiest during freeze: " + ($top -join '  ')
                    Write-Host "    $line" -ForegroundColor Yellow
                    $culprits.Add("$($row.Time)  $line") | Out-Null
                }
            }
        }
        $wasFrozen = -not $responding

        Start-Sleep -Milliseconds $IntervalMs
    }
}
finally {
    if ($rows.Count -gt 0) {
        $rows | Export-Csv -Path $LogPath -NoTypeInformation -Encoding UTF8
        Write-Host "`nWrote $($rows.Count) samples to $LogPath" -ForegroundColor Green

        $frozen = $rows | Where-Object { -not $_.Responding }
        if ($frozen.Count -eq 0) {
            Write-Host "The game never stopped responding during this run." -ForegroundColor Yellow
        }
        else {
            $readMb  = [math]::Round((($frozen | Measure-Object ReadKB  -Sum).Sum) / 1024, 1)
            $writeMb = [math]::Round((($frozen | Measure-Object WriteKB -Sum).Sum) / 1024, 1)
            $busy    = ($frozen | Where-Object { $_.SysBusyPct -ge 0 } | Measure-Object SysBusyPct -Average).Average
            Write-Host "`n=== while not responding ($($frozen.Count) samples) ===" -ForegroundColor Cyan
            Write-Host ("  game disk      : {0} MB read, {1} MB written" -f $readMb, $writeMb)
            Write-Host ("  free RAM       : {0} -> {1} MB" -f $frozen[0].FreeRamMb, $frozen[-1].FreeRamMb)
            Write-Host ("  machine CPU    : {0:N0}% busy on average" -f $busy)
            foreach ($c in $culprits) { Write-Host "  $c" }
            Write-Host ""
            if ($busy -lt 25 -and $readMb -lt 50) {
                Write-Host "  Reading: the machine was mostly IDLE and the game was not reading much." -ForegroundColor Green
                Write-Host "  Nothing was competing for the box, so the game was WAITING - consistent with" -ForegroundColor Green
                Write-Host "  the GPU present stalls already captured. Chase the display driver." -ForegroundColor Green
            } else {
                Write-Host "  Reading: something WAS working during the freeze (see CPU/disk above)." -ForegroundColor Green
                Write-Host "  A big read or a busy machine points at disk/anti-virus/paging, not the GPU." -ForegroundColor Green
            }
        }
    }
}
