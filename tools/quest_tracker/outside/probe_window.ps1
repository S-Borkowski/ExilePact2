<#
.SYNOPSIS
Shows known lines of text in a borderless topmost window, the screen capture's positive control.

.DESCRIPTION
Developer-only, part of tools/quest_tracker/outside. It shows a borderless,
topmost WinForms window with the lines of -Text ("|" separates lines) in a
plain font, light on a dark background, centred on the primary screen, for
-Seconds seconds from when it is shown. When shown it prints
"probe: pid <pid> shown <utc> rect [l,t,r,b]"; the capture takes its window
by that pid.

With -RenderTo <folder> it opens no window: it draws each line the same way
to <folder>\line-<n>.png, prints each path and exits. That is what
`screen.py selftest-ocr` reads.

Per-monitor DPI awareness is set first, as in capture.ps1.

.EXAMPLE
powershell.exe -NoProfile -ExecutionPolicy Bypass -File probe_window.ps1 -Text "Beira of the Rotten Pack|What happened here...?" -Seconds 3
#>
param(
    [string]$Text = "Beira of the Rotten Pack|What happened here...?",
    [double]$Seconds = 3,
    [string]$RenderTo = ""
)
$ErrorActionPreference = 'Stop'

$lines = @($Text.Split('|') | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
if ($lines.Count -eq 0) {
    [Console]::Error.WriteLine("probe: error: -Text holds no line")
    exit 1
}

Add-Type -ReferencedAssemblies System.Drawing, System.Windows.Forms -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Drawing.Text;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public static class QtProbe
{
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    [DllImport("kernel32.dll")] static extern void GetSystemTimePreciseAsFileTime(out long fileTime);

    public const int FontPx = 32;
    public const int Pad = 28;
    static readonly Color Back = Color.FromArgb(18, 18, 22);
    static readonly Color Fore = Color.FromArgb(236, 236, 236);

    public static void SetDpiAwareness()
    {
        try { SetProcessDpiAwarenessContext(new IntPtr(-4)); }
        catch (EntryPointNotFoundException) { SetProcessDPIAware(); }
    }

    static string Iso()
    {
        long ft;
        GetSystemTimePreciseAsFileTime(out ft);
        return DateTime.FromFileTimeUtc(ft).ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", CultureInfo.InvariantCulture);
    }

    /// The lines drawn one under another, light on dark, with a margin all round.
    public static Bitmap Render(string[] lines)
    {
        using (Font font = new Font("Segoe UI", FontPx, FontStyle.Regular, GraphicsUnit.Pixel))
        using (Bitmap probe = new Bitmap(1, 1))
        using (Graphics m = Graphics.FromImage(probe))
        {
            int width = 0;
            int lineH = (int)Math.Ceiling(font.GetHeight(m)) + 8;
            foreach (string l in lines)
                width = Math.Max(width, (int)Math.Ceiling(m.MeasureString(l, font).Width));
            Bitmap bmp = new Bitmap(width + 2 * Pad, lineH * lines.Length + 2 * Pad, PixelFormat.Format24bppRgb);
            using (Graphics g = Graphics.FromImage(bmp))
            using (Brush brush = new SolidBrush(Fore))
            {
                g.Clear(Back);
                g.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
                for (int i = 0; i < lines.Length; i++)
                    g.DrawString(lines[i], font, brush, Pad, Pad + i * lineH);
            }
            return bmp;
        }
    }

    public static void Show(string[] lines, double seconds)
    {
        Bitmap bmp = Render(lines);
        Form form = new Form();
        form.FormBorderStyle = FormBorderStyle.None;
        form.TopMost = true;
        form.ShowInTaskbar = false;
        form.StartPosition = FormStartPosition.Manual;
        form.BackColor = Back;
        form.BackgroundImage = bmp;
        form.BackgroundImageLayout = ImageLayout.None;
        form.ClientSize = bmp.Size;
        Rectangle area = Screen.PrimaryScreen.WorkingArea;
        form.Location = new Point(area.Left + (area.Width - bmp.Width) / 2, area.Top + (area.Height - bmp.Height) / 2);
        Timer timer = new Timer();
        timer.Interval = Math.Max(1, (int)Math.Round(seconds * 1000));
        timer.Tick += delegate { timer.Stop(); form.Close(); };
        form.Shown += delegate
        {
            Rectangle r = form.Bounds;
            Console.Out.WriteLine("probe: pid " + System.Diagnostics.Process.GetCurrentProcess().Id + " shown " + Iso()
                + " rect [" + r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom + "]");
            Console.Out.Flush();
            timer.Start();
        };
        Application.Run(form);
        Console.Out.WriteLine("probe: closed " + Iso());
        Console.Out.Flush();
    }
}
'@

[QtProbe]::SetDpiAwareness()
if ($RenderTo -ne '') {
    $dir = [System.IO.Path]::GetFullPath($RenderTo)
    for ($d = $dir; $d; ) {
        if (Test-Path -LiteralPath (Join-Path $d '.git')) {
            [Console]::Error.WriteLine("probe: error: refusing to render under " + $dir + ": it is inside the git checkout " + $d)
            exit 1
        }
        $parent = [System.IO.Path]::GetDirectoryName($d)
        if ($parent -eq $d) { break }
        $d = $parent
    }
    [void][System.IO.Directory]::CreateDirectory($dir)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $bmp = [QtProbe]::Render([string[]]@($lines[$i]))
        $path = Join-Path $dir ("line-" + ($i + 1) + ".png")
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        [Console]::Out.WriteLine("probe: rendered " + $path)
    }
    exit 0
}
if ($Seconds -le 0) {
    [Console]::Error.WriteLine("probe: error: -Seconds must be above 0")
    exit 1
}
[QtProbe]::Show([string[]]$lines, $Seconds)
exit 0
