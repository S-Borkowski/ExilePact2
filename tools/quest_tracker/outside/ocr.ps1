<#
.SYNOPSIS
Reads the text in a list of images with Windows.Media.Ocr, one process for the whole list.

.DESCRIPTION
Developer-only, part of tools/quest_tracker/outside. -List is a UTF-8 text
file with one image path per line. Each image is upscaled by -Scale first
(high-quality bicubic; the scale is lowered when it would push a side past
the engine's maximum image dimension, measured 10,000 on this machine), then
read by the Windows.Media.Ocr engine for -Language through WinRT in Windows
PowerShell 5.1.

For each image one JSON line is appended to -Out:
{"path": <as listed>, "scale": <used>, "lines": [{"text": ..., "box": [x0, y0, x1, y1]}]},
the box being the union of the line's word boxes in the original image's
pixels. An image that cannot be read gets "lines": [] and an "error". Prints
"ocr: done <n> images <e> errors". Exits 2 when no engine exists for the
language, 1 on a bad argument; an -Out inside a git checkout is refused.

.EXAMPLE
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ocr.ps1 -List frames.txt -Out L\i3\ocr.jsonl -Scale 2
#>
param(
    [Parameter(Mandatory = $true)][string]$List,
    [Parameter(Mandatory = $true)][string]$Out,
    [double]$Scale = 2,
    [string]$Language = "en-US"
)
$ErrorActionPreference = 'Stop'

function Fail([string]$message, [int]$code = 1) {
    [Console]::Error.WriteLine("ocr: error: " + $message)
    exit $code
}

$Out = [System.IO.Path]::GetFullPath($Out)
for ($d = [System.IO.Path]::GetDirectoryName($Out); $d; ) {
    if (Test-Path -LiteralPath (Join-Path $d '.git')) {
        Fail ("refusing to write " + $Out + ": it is inside the git checkout " + $d)
    }
    $parent = [System.IO.Path]::GetDirectoryName($d)
    if ($parent -eq $d) { break }
    $d = $parent
}
if (-not (Test-Path -LiteralPath $List -PathType Leaf)) { Fail ("no list file at " + $List) }
if ($Scale -lt 1 -or $Scale -gt 8) { Fail ("-Scale must lie in 1..8, got " + $Scale) }

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrResult, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Globalization, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.SoftwareBitmap, Windows.Graphics, ContentType = WindowsRuntime]

# WinRT's IAsyncOperation<T> awaited through the .NET bridge's AsTask<T>.
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($operation, [Type]$resultType) {
    $task = $asTaskGeneric.MakeGenericMethod($resultType).Invoke($null, @($operation))
    [void]$task.Wait(-1)
    return $task.Result
}

$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage((New-Object Windows.Globalization.Language $Language))
if ($null -eq $engine) { Fail ("Windows.Media.Ocr has no engine for " + $Language + "; install that OCR language") 2 }
$maxDim = [int][Windows.Media.Ocr.OcrEngine]::MaxImageDimension

function Read-Image([string]$path) {
    $src = [System.Drawing.Image]::FromFile($path)
    try {
        $scale = [double]$Scale
        $longest = [Math]::Max($src.Width, $src.Height)
        if ($longest * $scale -gt $maxDim) { $scale = [Math]::Max(1.0, [Math]::Floor(100.0 * $maxDim / $longest) / 100.0) }
        $w = [int][Math]::Round($src.Width * $scale)
        $h = [int][Math]::Round($src.Height * $scale)
        $bmp = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
        try {
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $g.DrawImage($src, 0, 0, $w, $h)
            $g.Dispose()
            $ms = New-Object System.IO.MemoryStream
            $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Bmp)
        }
        finally { $bmp.Dispose() }
    }
    finally { $src.Dispose() }
    $ms.Position = 0
    $stream = [System.IO.WindowsRuntimeStreamExtensions]::AsRandomAccessStream($ms)
    $decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $soft = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
    $result = Await ($engine.RecognizeAsync($soft)) ([Windows.Media.Ocr.OcrResult])
    $soft.Dispose()
    $ms.Dispose()

    $lines = New-Object System.Collections.ArrayList
    foreach ($line in $result.Lines) {
        $x0 = [double]::MaxValue; $y0 = [double]::MaxValue; $x1 = 0.0; $y1 = 0.0
        foreach ($word in $line.Words) {
            $r = $word.BoundingRect
            $x0 = [Math]::Min($x0, $r.X); $y0 = [Math]::Min($y0, $r.Y)
            $x1 = [Math]::Max($x1, $r.X + $r.Width); $y1 = [Math]::Max($y1, $r.Y + $r.Height)
        }
        if ($x0 -eq [double]::MaxValue) { $x0 = 0.0; $y0 = 0.0 }
        $box = @([int][Math]::Floor($x0 / $scale), [int][Math]::Floor($y0 / $scale),
            [int][Math]::Ceiling($x1 / $scale), [int][Math]::Ceiling($y1 / $scale))
        [void]$lines.Add([ordered]@{ text = [string]$line.Text; box = $box })
    }
    return @{ scale = $scale; lines = $lines }
}

$utf8 = New-Object System.Text.UTF8Encoding $false
$paths = @([System.IO.File]::ReadAllLines($List, $utf8) | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
$errors = 0
foreach ($path in $paths) {
    $record = [ordered]@{ path = $path }
    try {
        $read = Read-Image $path
        $record.scale = $read.scale
        $record.lines = @($read.lines.ToArray())
    }
    catch {
        $errors++
        $record.lines = @()
        $record.error = $_.Exception.Message
    }
    $json = ConvertTo-Json -InputObject $record -Compress -Depth 6
    [System.IO.File]::AppendAllText($Out, $json + "`n", $utf8)
}
[Console]::Out.WriteLine("ocr: done " + $paths.Count + " images " + $errors + " errors")
exit 0
