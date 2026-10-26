<#
.SYNOPSIS
Stamps each Ctrl+Alt+M press in UTC to a markers file, with a short tone from its own process.

.DESCRIPTION
Developer-only, part of tools/quest_tracker/outside. It registers the system
hotkey Ctrl+Alt+M (RegisterHotKey with MOD_NOREPEAT, so holding the keys
counts once) on its own thread's message loop, which wakes on a message or
every 200 ms to check -StopFile.

When it starts it appends its header to -Out and prints the first line of it,
"# registered Ctrl+Alt+M ...", which is its ready line:

    # registered Ctrl+Alt+M (MOD_NOREPEAT) at <utc>
    # pid <this process>
    # tone 1760 Hz 150 ms

Each press appends "<n> <utc>" (n counts on from any presses already in the
file), flushes, prints "marker: press <n> <utc>" and plays a 1760 Hz, 150 ms
tone from this process (System.Media.SoundPlayer, from memory), so the game's
process-loopback recording can show that it does not hear other processes.

It ends when -StopFile exists, appending "# stopped <utc> presses <k>". If
the hotkey is already taken it prints the reason and exits 2. An -Out inside
a git checkout is refused.

.EXAMPLE
powershell.exe -NoProfile -ExecutionPolicy Bypass -File marker.ps1 -Out L\markers.txt -StopFile L\stop-marker
#>
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [Parameter(Mandatory = $true)][string]$StopFile
)
$ErrorActionPreference = 'Stop'

function Fail([string]$message, [int]$code = 1) {
    [Console]::Error.WriteLine("marker: error: " + $message)
    exit $code
}

$Out = [System.IO.Path]::GetFullPath($Out)
$StopFile = [System.IO.Path]::GetFullPath($StopFile)
for ($d = [System.IO.Path]::GetDirectoryName($Out); $d; ) {
    if (Test-Path -LiteralPath (Join-Path $d '.git')) {
        Fail ("refusing to write " + $Out + ": it is inside the git checkout " + $d)
    }
    $parent = [System.IO.Path]::GetDirectoryName($d)
    if ($parent -eq $d) { break }
    $d = $parent
}
if (Test-Path -LiteralPath $StopFile) { Fail ("the stop file " + $StopFile + " already exists; remove it before starting") }
$outDir = [System.IO.Path]::GetDirectoryName($Out)
if (-not (Test-Path -LiteralPath $outDir -PathType Container)) { Fail ("no folder " + $outDir) }

Add-Type -TypeDefinition @'
using System;
using System.Globalization;
using System.IO;
using System.Media;
using System.Runtime.InteropServices;
using System.Text;

public static class QtMarker
{
    [StructLayout(LayoutKind.Sequential)]
    public struct MSG
    {
        public IntPtr hwnd; public uint message; public IntPtr wParam; public IntPtr lParam;
        public uint time; public int ptX; public int ptY; public uint lPrivate;
    }

    [DllImport("user32.dll", SetLastError = true)] static extern bool RegisterHotKey(IntPtr hWnd, int id, uint modifiers, uint vk);
    [DllImport("user32.dll")] static extern bool UnregisterHotKey(IntPtr hWnd, int id);
    [DllImport("user32.dll")] static extern bool PeekMessage(out MSG msg, IntPtr hWnd, uint min, uint max, uint remove);
    [DllImport("user32.dll")] static extern uint MsgWaitForMultipleObjects(uint count, IntPtr[] handles, bool waitAll, uint ms, uint wakeMask);
    [DllImport("kernel32.dll")] static extern void GetSystemTimePreciseAsFileTime(out long fileTime);

    const uint MOD_ALT = 0x1, MOD_CONTROL = 0x2, MOD_NOREPEAT = 0x4000;
    const uint VK_M = 0x4D;
    const uint WM_HOTKEY = 0x0312;
    const uint PM_REMOVE = 0x1;
    const uint QS_ALLINPUT = 0x04FF;
    const int HOTKEY_ID = 0x5154;  // any id unique to this thread
    public const int ToneHz = 1760;
    public const int ToneMs = 150;
    const int Rate = 48000;

    static SoundPlayer player;

    static string Iso()
    {
        long ft;
        GetSystemTimePreciseAsFileTime(out ft);
        return DateTime.FromFileTimeUtc(ft).ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", CultureInfo.InvariantCulture);
    }

    /// A mono 16-bit 48 kHz WAV of the tone, with 5 ms ramps so it does not click.
    static byte[] ToneWav()
    {
        int n = Rate * ToneMs / 1000, ramp = Rate * 5 / 1000;
        MemoryStream ms = new MemoryStream();
        BinaryWriter w = new BinaryWriter(ms);
        w.Write(Encoding.ASCII.GetBytes("RIFF")); w.Write(36 + n * 2); w.Write(Encoding.ASCII.GetBytes("WAVE"));
        w.Write(Encoding.ASCII.GetBytes("fmt ")); w.Write(16); w.Write((short)1); w.Write((short)1);
        w.Write(Rate); w.Write(Rate * 2); w.Write((short)2); w.Write((short)16);
        w.Write(Encoding.ASCII.GetBytes("data")); w.Write(n * 2);
        for (int i = 0; i < n; i++)
        {
            double env = Math.Min(1.0, Math.Min(i, n - 1 - i) / (double)ramp);
            w.Write((short)Math.Round(0.3 * 32767 * env * Math.Sin(2 * Math.PI * ToneHz * i / Rate)));
        }
        w.Flush();
        return ms.ToArray();
    }

    static void Append(string path, string line)
    {
        using (FileStream fs = new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite))
        {
            byte[] b = new UTF8Encoding(false).GetBytes(line + "\n");
            fs.Write(b, 0, b.Length);
            fs.Flush(true);
        }
    }

    static void Say(string line)
    {
        Console.Out.WriteLine(line);
        Console.Out.Flush();
    }

    /// Presses already in the file, so a restarted marker numbers on from them.
    static int ExistingPresses(string path)
    {
        if (!File.Exists(path)) return 0;
        int last = 0;
        foreach (string raw in File.ReadAllLines(path))
        {
            string line = raw.Trim();
            if (line.Length == 0 || line[0] == '#') continue;
            int n;
            if (int.TryParse(line.Split(' ')[0], NumberStyles.None, CultureInfo.InvariantCulture, out n)) last = Math.Max(last, n);
        }
        return last;
    }

    public static int Run(string outPath, string stopFile)
    {
        if (!RegisterHotKey(IntPtr.Zero, HOTKEY_ID, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_M))
        {
            int err = Marshal.GetLastWin32Error();
            string why = err == 1409 ? "it is already registered by another program" : "RegisterHotKey failed";
            Console.Error.WriteLine("marker: error: Ctrl+Alt+M could not be registered: " + why + " (Win32 error " + err + ")");
            return 2;
        }
        try
        {
            player = new SoundPlayer(new MemoryStream(ToneWav()));
            player.Load();
            int count = ExistingPresses(outPath), presses = 0;
            string registered = "# registered Ctrl+Alt+M (MOD_NOREPEAT) at " + Iso();
            Append(outPath, registered);
            Append(outPath, "# pid " + System.Diagnostics.Process.GetCurrentProcess().Id);
            Append(outPath, "# tone " + ToneHz + " Hz " + ToneMs + " ms");
            Say(registered);
            while (!File.Exists(stopFile))
            {
                MsgWaitForMultipleObjects(0, null, false, 200, QS_ALLINPUT);
                MSG m;
                while (PeekMessage(out m, IntPtr.Zero, 0, 0, PM_REMOVE))
                {
                    if (m.message != WM_HOTKEY || m.wParam.ToInt64() != HOTKEY_ID) continue;
                    string when = Iso();
                    count++; presses++;
                    Append(outPath, count + " " + when);
                    player.Play();
                    Say("marker: press " + count + " " + when);
                }
            }
            string stopped = "# stopped " + Iso() + " presses " + presses;
            Append(outPath, stopped);
            Say("marker: stopped stop-file presses " + presses);
            return 0;
        }
        finally { UnregisterHotKey(IntPtr.Zero, HOTKEY_ID); }
    }
}
'@

exit [QtMarker]::Run($Out, $StopFile)
