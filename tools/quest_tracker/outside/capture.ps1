<#
.SYNOPSIS
Captures two bands of one process's window to JPEG at a fixed rate, for OCR afterwards.

.DESCRIPTION
Developer-only, part of tools/quest_tracker/outside. It never opens the target
process: it finds the largest visible top-level window the process owns
(EnumWindows and GetWindowThreadProcessId), reads that window's client rect on
every frame (GetClientRect and ClientToScreen) and copies two bands of it from
the screen (Graphics.CopyFromScreen). A band is a fraction x0,y0,x1,y1 of the
client rect; "off" leaves it out.

Each band's frame is saved as a quality-90 JPEG,
OutDir\frames\<band>\<yyyyMMddTHHmmss.fffZ>.jpg, and gets one line in
OutDir\frames.jsonl: utc, band, path (relative to OutDir), rect and client
(screen pixels, left, top, right, bottom), foreground (the target process
owns the foreground window), and luma_mean and luma_std (0-255, from every
fourth pixel of every fourth row), so a black GDI capture shows in the index.
A frame skipped while the window is minimized, or one whose copy failed, gets
a line with path null and a "skipped" reason.

Frames follow a fixed schedule of 1/Hz seconds; a tick missed because the
machine was busy is skipped, never caught up. Prints "capture: ready ..."
after the first frame, and ends when StopFile exists, when the window is gone
or after MaxMinutes, printing "capture: stopped <reason> ...".

Per-monitor DPI awareness is set before any screen call. OutDir inside a git
checkout is refused. About 2 GB per 30 minutes at the defaults.

.EXAMPLE
powershell.exe -NoProfile -ExecutionPolicy Bypass -File capture.ps1 -TargetPid 1234 -OutDir L\i3 -StopFile L\stop-i3 -Hz 2
#>
param(
    [Parameter(Mandatory = $true)][int]$TargetPid,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [Parameter(Mandatory = $true)][string]$StopFile,
    [double]$Hz = 2,
    [string]$Top = "0.25,0,0.75,0.14",
    [string]$Bottom = "0,0.35,1,1",
    [double]$MaxMinutes = 90,
    # How long to wait for the target's window to appear before giving up.
    [double]$WaitSeconds = 10
)
$ErrorActionPreference = 'Stop'

function Fail([string]$message) {
    [Console]::Error.WriteLine("capture: error: " + $message)
    exit 1
}

# The nearest folder at or above $path holding a .git entry, as poe2db.git_tree_of.
function Get-GitTree([string]$path) {
    $d = [System.IO.Path]::GetFullPath($path)
    while ($d) {
        if (Test-Path -LiteralPath (Join-Path $d '.git')) { return $d }
        $parent = [System.IO.Path]::GetDirectoryName($d)
        if ($parent -eq $d) { break }
        $d = $parent
    }
    return $null
}

function ConvertTo-Band([string]$name, [string]$text) {
    if ($text -eq 'off' -or $text -eq '') { return $null }
    $parts = $text.Split(',')
    if ($parts.Count -ne 4) { Fail ("-" + $name + " needs x0,y0,x1,y1 or off, got '" + $text + "'") }
    $inv = [System.Globalization.CultureInfo]::InvariantCulture
    $v = New-Object 'double[]' 4
    for ($i = 0; $i -lt 4; $i++) {
        $x = 0.0
        if (-not [double]::TryParse($parts[$i].Trim(), [System.Globalization.NumberStyles]::Float, $inv, [ref]$x)) {
            Fail ("-" + $name + " has a value that is not a number: '" + $parts[$i] + "'")
        }
        if ($x -lt 0 -or $x -gt 1) { Fail ("-" + $name + " fractions lie in 0..1, got " + $parts[$i]) }
        $v[$i] = $x
    }
    if ($v[2] -le $v[0] -or $v[3] -le $v[1]) { Fail ("-" + $name + " needs x1 > x0 and y1 > y0, got '" + $text + "'") }
    return ,$v
}

if ($Hz -le 0 -or $Hz -gt 30) { Fail ("-Hz must lie in (0, 30], got " + $Hz) }
$topBand = ConvertTo-Band 'Top' $Top
$bottomBand = ConvertTo-Band 'Bottom' $Bottom
if ($null -eq $topBand -and $null -eq $bottomBand) { Fail "both bands are off: nothing to capture" }

$OutDir = [System.IO.Path]::GetFullPath($OutDir)
$StopFile = [System.IO.Path]::GetFullPath($StopFile)
$tree = Get-GitTree $OutDir
if ($tree) { Fail ("refusing to write frames under " + $OutDir + ": it is inside the git checkout " + $tree + ". Captures stay outside every repository.") }
if (Test-Path -LiteralPath $StopFile) { Fail ("the stop file " + $StopFile + " already exists; remove it before starting") }
foreach ($band in @('top', 'bottom')) {
    [void][System.IO.Directory]::CreateDirectory((Join-Path $OutDir (Join-Path 'frames' $band)))
}

Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class QtCapture
{
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] static extern bool IsWindow(IntPtr hWnd);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] static extern IntPtr GetThreadDpiAwarenessContext();
    [DllImport("user32.dll")] static extern int GetAwarenessFromDpiAwarenessContext(IntPtr value);
    [DllImport("kernel32.dll")] static extern void GetSystemTimePreciseAsFileTime(out long fileTime);

    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
    static readonly IntPtr PerMonitorV2 = new IntPtr(-4);

    /// Per-monitor awareness, so every rect is in physical pixels; returns what the thread ended up with.
    public static string SetDpiAwareness()
    {
        try { SetProcessDpiAwarenessContext(PerMonitorV2); }
        catch (EntryPointNotFoundException) { SetProcessDPIAware(); }
        try
        {
            int a = GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext());
            if (a == 2) return "per-monitor";
            if (a == 1) return "system";
            if (a == 0) return "unaware";
            return "unknown";
        }
        catch (EntryPointNotFoundException) { return "unknown"; }
    }

    public static DateTime UtcNow()
    {
        long ft;
        GetSystemTimePreciseAsFileTime(out ft);
        return DateTime.FromFileTimeUtc(ft);
    }

    public static string Iso(DateTime utc)
    {
        return utc.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", CultureInfo.InvariantCulture);
    }

    /// The largest visible top-level window `pid` owns, or zero.
    public static IntPtr FindWindow(int pid)
    {
        IntPtr best = IntPtr.Zero;
        long bestArea = -1;
        EnumWindows(delegate (IntPtr h, IntPtr l)
        {
            uint owner;
            GetWindowThreadProcessId(h, out owner);
            if (owner != (uint)pid || !IsWindowVisible(h)) return true;
            RECT r;
            if (!GetWindowRect(h, out r)) return true;
            long area = (long)Math.Max(0, r.Right - r.Left) * Math.Max(0, r.Bottom - r.Top);
            if (area > bestArea) { bestArea = area; best = h; }
            return true;
        }, IntPtr.Zero);
        return best;
    }

    static bool ClientOnScreen(IntPtr h, out RECT screen)
    {
        screen = new RECT();
        RECT c;
        if (!GetClientRect(h, out c)) return false;
        POINT p = new POINT();
        if (!ClientToScreen(h, ref p)) return false;
        screen.Left = p.X; screen.Top = p.Y;
        screen.Right = p.X + c.Right; screen.Bottom = p.Y + c.Bottom;
        return true;
    }

    static RECT BandRect(RECT client, double[] f)
    {
        int w = client.Right - client.Left, h = client.Bottom - client.Top;
        RECT r = new RECT();
        r.Left = client.Left + (int)Math.Round(f[0] * w);
        r.Top = client.Top + (int)Math.Round(f[1] * h);
        r.Right = client.Left + (int)Math.Round(f[2] * w);
        r.Bottom = client.Top + (int)Math.Round(f[3] * h);
        return r;
    }

    /// Mean and standard deviation of luma (0-255) over every fourth pixel of every fourth row.
    public static double[] Luma(Bitmap bmp)
    {
        Rectangle all = new Rectangle(0, 0, bmp.Width, bmp.Height);
        BitmapData data = bmp.LockBits(all, ImageLockMode.ReadOnly, PixelFormat.Format24bppRgb);
        try
        {
            byte[] row = new byte[bmp.Width * 3];
            double sum = 0, sumSq = 0;
            long n = 0;
            for (int y = 0; y < bmp.Height; y += 4)
            {
                Marshal.Copy(new IntPtr(data.Scan0.ToInt64() + (long)y * data.Stride), row, 0, row.Length);
                for (int x = 0; x < bmp.Width; x += 4)
                {
                    int i = x * 3;
                    double l = 0.114 * row[i] + 0.587 * row[i + 1] + 0.299 * row[i + 2];
                    sum += l; sumSq += l * l; n++;
                }
            }
            if (n == 0) return new double[] { 0, 0 };
            double mean = sum / n;
            return new double[] { mean, Math.Sqrt(Math.Max(0, sumSq / n - mean * mean)) };
        }
        finally { bmp.UnlockBits(data); }
    }

    static string Json(string s)
    {
        if (s == null) return "null";
        StringBuilder b = new StringBuilder("\"");
        foreach (char c in s)
        {
            if (c == '"') b.Append("\\\"");
            else if (c == '\\') b.Append("\\\\");
            else if (c < 0x20) b.Append("\\u").Append(((int)c).ToString("x4"));
            else b.Append(c);
        }
        return b.Append('"').ToString();
    }

    static string Json(RECT r)
    {
        return string.Format(CultureInfo.InvariantCulture, "[{0},{1},{2},{3}]", r.Left, r.Top, r.Right, r.Bottom);
    }

    static string Num(double v)
    {
        return Math.Round(v, 2).ToString("0.##", CultureInfo.InvariantCulture);
    }

    static ImageCodecInfo Jpeg()
    {
        foreach (ImageCodecInfo c in ImageCodecInfo.GetImageEncoders())
            if (c.MimeType == "image/jpeg") return c;
        throw new InvalidOperationException("no JPEG encoder");
    }

    static void Say(string line)
    {
        Console.Out.WriteLine(line);
        Console.Out.Flush();
    }

    public static int Run(int pid, string outDir, string stopFile, double hz, double[] top, double[] bottom,
                          double maxMinutes, double waitSeconds, string dpi)
    {
        DateTime started = UtcNow();
        IntPtr hwnd = IntPtr.Zero;
        while (true)
        {
            hwnd = FindWindow(pid);
            if (hwnd != IntPtr.Zero) break;
            if (File.Exists(stopFile)) { Say("capture: stopped stop-file frames 0 skipped 0"); return 0; }
            if ((UtcNow() - started).TotalSeconds > waitSeconds)
            {
                Console.Error.WriteLine("capture: error: process " + pid + " has no visible top-level window after " + waitSeconds + " s");
                return 2;
            }
            Thread.Sleep(100);
        }

        string[] names = new string[] { "top", "bottom" };
        double[][] bands = new double[][] { top, bottom };
        ImageCodecInfo jpeg = Jpeg();
        EncoderParameters quality = new EncoderParameters(1);
        quality.Param[0] = new EncoderParameter(System.Drawing.Imaging.Encoder.Quality, 90L);
        string index = Path.Combine(outDir, "frames.jsonl");
        double periodMs = 1000.0 / hz;
        int frames = 0, skipped = 0;
        bool ready = false;
        string reason = "max-minutes";
        DateTime t0 = UtcNow();

        using (StreamWriter log = new StreamWriter(new FileStream(index, FileMode.Append, FileAccess.Write, FileShare.Read), new UTF8Encoding(false)))
        {
            log.NewLine = "\n";
            log.AutoFlush = true;
            for (long tick = 0; ; tick++)
            {
                DateTime due = t0.AddMilliseconds(tick * periodMs);
                double wait = (due - UtcNow()).TotalMilliseconds;
                if (wait < -periodMs)
                {
                    // Behind by more than one period: drop the missed ticks rather than bursting.
                    tick = (long)Math.Floor((UtcNow() - t0).TotalMilliseconds / periodMs);
                    continue;
                }
                if (wait > 0) Thread.Sleep((int)Math.Ceiling(wait));

                if (File.Exists(stopFile)) { reason = "stop-file"; break; }
                if ((UtcNow() - t0).TotalMinutes >= maxMinutes) { reason = "max-minutes"; break; }
                if (!IsWindow(hwnd))
                {
                    hwnd = FindWindow(pid);
                    if (hwnd == IntPtr.Zero) { reason = "window-gone"; break; }
                }
                IntPtr fg = GetForegroundWindow();
                uint fgPid = 0;
                if (fg != IntPtr.Zero) GetWindowThreadProcessId(fg, out fgPid);
                bool foreground = fgPid == (uint)pid;
                string fgText = foreground ? "true" : "false";

                RECT client = new RECT();
                bool minimized = IsIconic(hwnd);
                bool haveClient = !minimized && ClientOnScreen(hwnd, out client);

                for (int b = 0; b < 2; b++)
                {
                    if (bands[b] == null) continue;
                    DateTime now = UtcNow();
                    string when = Iso(now);
                    if (!haveClient)
                    {
                        skipped++;
                        log.WriteLine("{\"utc\":" + Json(when) + ",\"band\":" + Json(names[b]) + ",\"path\":null,\"rect\":null,\"client\":null,\"foreground\":" + fgText + ",\"skipped\":" + Json(minimized ? "minimized" : "no-client-rect") + "}");
                        continue;
                    }
                    RECT r = BandRect(client, bands[b]);
                    int w = r.Right - r.Left, h = r.Bottom - r.Top;
                    if (w <= 0 || h <= 0)
                    {
                        skipped++;
                        log.WriteLine("{\"utc\":" + Json(when) + ",\"band\":" + Json(names[b]) + ",\"path\":null,\"rect\":" + Json(r) + ",\"client\":" + Json(client) + ",\"foreground\":" + fgText + ",\"skipped\":\"empty-band\"}");
                        continue;
                    }
                    string rel = "frames/" + names[b] + "/" + now.ToString("yyyyMMdd'T'HHmmss.fff'Z'", CultureInfo.InvariantCulture) + ".jpg";
                    try
                    {
                        using (Bitmap bmp = new Bitmap(w, h, PixelFormat.Format24bppRgb))
                        {
                            using (Graphics g = Graphics.FromImage(bmp))
                                g.CopyFromScreen(r.Left, r.Top, 0, 0, new Size(w, h), CopyPixelOperation.SourceCopy);
                            double[] luma = Luma(bmp);
                            bmp.Save(Path.Combine(outDir, rel.Replace('/', Path.DirectorySeparatorChar)), jpeg, quality);
                            frames++;
                            log.WriteLine("{\"utc\":" + Json(when) + ",\"band\":" + Json(names[b]) + ",\"path\":" + Json(rel) + ",\"rect\":" + Json(r) + ",\"client\":" + Json(client) + ",\"foreground\":" + fgText + ",\"luma_mean\":" + Num(luma[0]) + ",\"luma_std\":" + Num(luma[1]) + "}");
                        }
                    }
                    catch (Exception e)
                    {
                        // A locked or secure desktop makes the copy throw; log it and keep the schedule.
                        skipped++;
                        log.WriteLine("{\"utc\":" + Json(when) + ",\"band\":" + Json(names[b]) + ",\"path\":null,\"rect\":" + Json(r) + ",\"client\":" + Json(client) + ",\"foreground\":" + fgText + ",\"skipped\":" + Json("error: " + e.Message) + "}");
                    }
                }
                if (!ready && frames > 0)
                {
                    ready = true;
                    RECT c2;
                    ClientOnScreen(hwnd, out c2);
                    Say("capture: ready pid " + pid + " hwnd 0x" + hwnd.ToInt64().ToString("x") + " client " + Json(c2) + " dpi " + dpi + " hz " + Num(hz) + " first " + Iso(UtcNow()));
                }
            }
        }
        Say("capture: stopped " + reason + " frames " + frames + " skipped " + skipped + " at " + Iso(UtcNow()));
        return 0;
    }
}
'@

$dpi = [QtCapture]::SetDpiAwareness()
exit [QtCapture]::Run($TargetPid, $OutDir, $StopFile, $Hz, $topBand, $bottomBand, $MaxMinutes, $WaitSeconds, $dpi)
