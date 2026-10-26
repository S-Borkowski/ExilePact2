<#
.SYNOPSIS
Record one process tree's audio through WASAPI process loopback into a WAV
whose sample index stays linear in UTC. Developer-only.

.DESCRIPTION
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File loopback.ps1 `
        -TargetPid P -Out L\i2\game.wav -StopFile L\stop-i2 [-MaxMinutes 90]
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File loopback.ps1 -CompileOnly

It activates the virtual device VAD\Process_Loopback for the process P and
its children (PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE), so only that
tree's audio is recorded, not the rest of the desktop. Nothing here opens the
target's process, reads its memory or calls its code; the recorder only asks
Windows' audio engine for the mix of that tree's streams, and checks once a
second whether a process with that pid still exists.

The WAV is 48 kHz, 2-channel, 16-bit PCM. Its header carries placeholder
sizes (0xFFFFFFFF) while it records and is flushed at least once a second,
so it can be read while it grows (readers take the data to run to the end of
the file); the sizes are patched when it stops.

The timeline: sample 0 is the instant the stream started, and each packet's
first frame belongs at the sample its QPC timestamp gives. When a packet
starts more than 10 ms after the samples written so far, zeros are written up
to it and the gap is logged. Packets flagged silent are written as zeros.
While no packet arrives for over a second, zeros are written up to half a
second ago. Anchors, `[sample, utc]`, are taken at the start, every 10 s, on
any packet that lands early, and at the end. The sidecar `<Out>.json` holds
pid, sample_rate, channels, bits, first_sample_utc, anchors, gaps, end_utc and
end_reason, and is rewritten at each anchor.

It prints `ready <utc>` once the first packet arrives, or 2 s after the
start. It stops on its stop file (`stop-file`), when the target exits
(`target-exited`), when the device is invalidated (`device-invalidated`) or
at -MaxMinutes (`max-minutes`), prints `stopped <utc> <reason>` and exits 0.
On an activation or format failure, an existing -Out, or a path inside a git
checkout, it prints one `error <reason>` line and exits 1.

Windows PowerShell 5.1 only compiles C# 5 through Add-Type, so the C# below
avoids string interpolation, nameof and expression-bodied members.
#>
param(
    [int]$TargetPid = 0,
    [string]$Out = '',
    [string]$StopFile = '',
    [double]$MaxMinutes = 90,
    [switch]$CompileOnly
)

$ErrorActionPreference = 'Stop'

$source = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace QuestTrackerOutside
{
    [ComImport, Guid("1CB9AD4C-DBFA-4c32-B178-C2F568A703B2"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IAudioClient
    {
        [PreserveSig] int Initialize(int shareMode, uint streamFlags, long bufferDuration, long periodicity, IntPtr format, IntPtr sessionGuid);
        [PreserveSig] int GetBufferSize(out uint frames);
        [PreserveSig] int GetStreamLatency(out long latency);
        [PreserveSig] int GetCurrentPadding(out uint padding);
        [PreserveSig] int IsFormatSupported(int shareMode, IntPtr format, out IntPtr closest);
        [PreserveSig] int GetMixFormat(out IntPtr format);
        [PreserveSig] int GetDevicePeriod(out long defaultPeriod, out long minimumPeriod);
        [PreserveSig] int Start();
        [PreserveSig] int Stop();
        [PreserveSig] int Reset();
        [PreserveSig] int SetEventHandle(IntPtr eventHandle);
        [PreserveSig] int GetService(ref Guid riid, [MarshalAs(UnmanagedType.IUnknown)] out object service);
    }

    [ComImport, Guid("C8ADBD64-E71E-48a0-A4DE-185C395CD317"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IAudioCaptureClient
    {
        [PreserveSig] int GetBuffer(out IntPtr data, out uint frames, out uint flags, out ulong devicePosition, out ulong qpcPosition);
        [PreserveSig] int ReleaseBuffer(uint frames);
        [PreserveSig] int GetNextPacketSize(out uint frames);
    }

    [ComImport, Guid("72A22D78-CDE4-431D-B8CC-843A71199B6D"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IActivateAudioInterfaceAsyncOperation
    {
        [PreserveSig] int GetActivateResult(out int activateResult, [MarshalAs(UnmanagedType.IUnknown)] out object activatedInterface);
    }

    [ComImport, Guid("41D949AB-9862-444A-80F6-C261334DA5EB"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IActivateAudioInterfaceCompletionHandler
    {
        [PreserveSig] int ActivateCompleted(IActivateAudioInterfaceAsyncOperation operation);
    }

    // The marker that tells COM the handler may be called from any thread.
    [ComImport, Guid("94ea2b94-e9cc-49e0-c0ff-ee64ca8f5b90"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IAgileObject
    {
    }

    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    public sealed class ActivationHandler : IActivateAudioInterfaceCompletionHandler, IAgileObject
    {
        public readonly ManualResetEvent Done = new ManualResetEvent(false);
        public int Result = unchecked((int)0x80004005);
        public object Client;

        public int ActivateCompleted(IActivateAudioInterfaceAsyncOperation operation)
        {
            try
            {
                int activateResult;
                object unknown;
                int hr = operation.GetActivateResult(out activateResult, out unknown);
                Result = hr < 0 ? hr : activateResult;
                if (Result >= 0) Client = unknown;
            }
            catch (Exception e)
            {
                Result = Marshal.GetHRForException(e);
            }
            finally
            {
                Done.Set();
            }
            return 0;
        }
    }

    public sealed class RecorderFailure : Exception
    {
        public RecorderFailure(string message) : base(message) { }
    }

    public sealed class Loopback
    {
        [DllImport("Mmdevapi.dll", ExactSpelling = true)]
        static extern int ActivateAudioInterfaceAsync(
            [MarshalAs(UnmanagedType.LPWStr)] string deviceInterfacePath, [In] ref Guid riid,
            IntPtr activationParams, IActivateAudioInterfaceCompletionHandler completionHandler,
            out IActivateAudioInterfaceAsyncOperation activationOperation);

        [DllImport("kernel32.dll")] static extern bool QueryPerformanceCounter(out long count);
        [DllImport("kernel32.dll")] static extern bool QueryPerformanceFrequency(out long frequency);
        [DllImport("kernel32.dll")] static extern void GetSystemTimePreciseAsFileTime(out long fileTime);

        const string ProcessLoopbackDevice = "VAD\\Process_Loopback";
        const int SampleRate = 48000;
        const int Channels = 2;
        const int Bits = 16;
        const int BlockAlign = 4;
        const uint StreamLoopback = 0x00020000;
        const uint StreamEventCallback = 0x00040000;
        const uint StreamAutoConvertPcm = 0x80000000;
        const uint StreamSrcDefaultQuality = 0x08000000;
        const int DeviceInvalidated = unchecked((int)0x88890004);
        const int BufferEmpty = 0x08890001;
        const uint FlagSilent = 0x2;
        const uint FlagTimestampError = 0x4;
        const long GapToleranceFrames = SampleRate / 100;  // 10 ms
        const long Hns = 10000000;                         // 100 ns units per second

        readonly int pid;
        readonly string outPath;
        readonly string stopFile;
        readonly double maxMinutes;
        FileStream wav;
        long written;
        long originHns;
        long offsetHns;  // UTC FILETIME minus QPC, both in 100 ns units
        string firstUtc;
        readonly List<string> anchors = new List<string>();
        readonly List<string> gaps = new List<string>();
        byte[] buffer = new byte[BlockAlign * SampleRate];
        long qpcFrequency;

        Loopback(int pid, string outPath, string stopFile, double maxMinutes)
        {
            this.pid = pid;
            this.outPath = outPath;
            this.stopFile = stopFile;
            this.maxMinutes = maxMinutes;
        }

        public static int Run(int pid, string outPath, string stopFile, double maxMinutes)
        {
            int code = 1;
            Thread thread = new Thread(delegate()
            {
                Loopback recorder = new Loopback(pid, outPath, stopFile, maxMinutes);
                try
                {
                    code = recorder.Record();
                }
                catch (RecorderFailure e)
                {
                    Say("error " + e.Message);
                    code = 1;
                }
                catch (Exception e)
                {
                    Say("error " + e.GetType().Name + ": " + e.Message.Replace("\r", " ").Replace("\n", " "));
                    code = 1;
                }
            });
            thread.SetApartmentState(ApartmentState.MTA);
            thread.Start();
            thread.Join();
            return code;
        }

        public static string CompileCheck()
        {
            return "compiled";
        }

        static void Say(string line)
        {
            Console.Out.WriteLine(line);
            Console.Out.Flush();
        }

        static string Hex(int hr)
        {
            return "0x" + hr.ToString("X8", CultureInfo.InvariantCulture);
        }

        // ---------------------------------------------------------------- clock

        long ToHns(long qpc)
        {
            return (qpc / qpcFrequency) * Hns + (qpc % qpcFrequency) * Hns / qpcFrequency;
        }

        // The QPC-to-UTC offset from the narrowest of five paired reads.
        void SampleClock()
        {
            long best = long.MaxValue;
            for (int i = 0; i < 5; i++)
            {
                long a, b, ft;
                QueryPerformanceCounter(out a);
                GetSystemTimePreciseAsFileTime(out ft);
                QueryPerformanceCounter(out b);
                if (b - a < best)
                {
                    best = b - a;
                    offsetHns = ft - ToHns(a + (b - a) / 2);
                }
            }
        }

        long NowHns()
        {
            long q;
            QueryPerformanceCounter(out q);
            return ToHns(q);
        }

        string UtcOf(long hns)
        {
            DateTime t = DateTime.FromFileTimeUtc(hns + offsetHns);
            return t.ToString("yyyy-MM-dd'T'HH:mm:ss.fff", CultureInfo.InvariantCulture) + "Z";
        }

        long FrameOf(long hns)
        {
            return (long)Math.Round((hns - originHns) * (double)SampleRate / Hns);
        }

        // ---------------------------------------------------------------- files

        static byte[] Header(uint dataBytes, uint riffBytes)
        {
            MemoryStream m = new MemoryStream(44);
            BinaryWriter w = new BinaryWriter(m);
            w.Write(Encoding.ASCII.GetBytes("RIFF")); w.Write(riffBytes); w.Write(Encoding.ASCII.GetBytes("WAVE"));
            w.Write(Encoding.ASCII.GetBytes("fmt ")); w.Write(16u);
            w.Write((ushort)1); w.Write((ushort)Channels); w.Write((uint)SampleRate);
            w.Write((uint)(SampleRate * BlockAlign)); w.Write((ushort)BlockAlign); w.Write((ushort)Bits);
            w.Write(Encoding.ASCII.GetBytes("data")); w.Write(dataBytes);
            w.Flush();
            return m.ToArray();
        }

        void WriteZeros(long frames)
        {
            Array.Clear(buffer, 0, buffer.Length);
            long left = frames * BlockAlign;
            while (left > 0)
            {
                int n = (int)Math.Min(left, buffer.Length);
                wav.Write(buffer, 0, n);
                left -= n;
            }
        }

        void Gap(long frames)
        {
            gaps.Add("[" + written + ", " + frames + "]");
            WriteZeros(frames);
            written += frames;
        }

        void Anchor(string utc)
        {
            anchors.Add("[" + written + ", \"" + utc + "\"]");
            WriteSidecar(null, null);
        }

        void WriteSidecar(string endUtc, string endReason)
        {
            StringBuilder s = new StringBuilder();
            s.Append("{\n");
            s.Append("  \"pid\": ").Append(pid).Append(",\n");
            s.Append("  \"sample_rate\": ").Append(SampleRate).Append(",\n");
            s.Append("  \"channels\": ").Append(Channels).Append(",\n");
            s.Append("  \"bits\": ").Append(Bits).Append(",\n");
            s.Append("  \"first_sample_utc\": \"").Append(firstUtc).Append("\",\n");
            s.Append("  \"anchors\": [").Append(string.Join(", ", anchors.ToArray())).Append("],\n");
            s.Append("  \"gaps\": [").Append(string.Join(", ", gaps.ToArray())).Append("],\n");
            s.Append("  \"frames\": ").Append(written).Append(",\n");
            s.Append("  \"end_utc\": ").Append(endUtc == null ? "null" : "\"" + endUtc + "\"").Append(",\n");
            s.Append("  \"end_reason\": ").Append(endReason == null ? "null" : "\"" + endReason + "\"").Append("\n");
            s.Append("}\n");
            string target = outPath + ".json";
            string tmp = target + ".tmp";
            File.WriteAllText(tmp, s.ToString(), new UTF8Encoding(false));
            for (int attempt = 0; ; attempt++)
            {
                try
                {
                    if (File.Exists(target)) File.Replace(tmp, target, null);
                    else File.Move(tmp, target);
                    return;
                }
                catch (IOException)
                {
                    // A reader holds the sidecar open; the next anchor rewrites it.
                    if (endReason == null) return;
                    if (attempt >= 40)
                    {
                        File.Copy(tmp, target, true);
                        File.Delete(tmp);
                        return;
                    }
                    Thread.Sleep(50);
                }
            }
        }

        // ---------------------------------------------------------------- activation

        IAudioClient Activate()
        {
            IntPtr blob = Marshal.AllocHGlobal(12);
            IntPtr variant = Marshal.AllocHGlobal(24);
            try
            {
                // AUDIOCLIENT_ACTIVATION_PARAMS: PROCESS_LOOPBACK, the pid, include the tree.
                Marshal.WriteInt32(blob, 0, 1);
                Marshal.WriteInt32(blob, 4, pid);
                Marshal.WriteInt32(blob, 8, 0);
                // PROPVARIANT holding a VT_BLOB.
                for (int i = 0; i < 24; i++) Marshal.WriteByte(variant, i, 0);
                Marshal.WriteInt16(variant, 0, 65);
                Marshal.WriteInt32(variant, 8, 12);
                Marshal.WriteIntPtr(variant, 8 + IntPtr.Size, blob);

                Guid iid = typeof(IAudioClient).GUID;
                ActivationHandler handler = new ActivationHandler();
                IActivateAudioInterfaceAsyncOperation operation;
                int hr = ActivateAudioInterfaceAsync(ProcessLoopbackDevice, ref iid, variant, handler, out operation);
                if (hr < 0) throw new RecorderFailure("activation failed: ActivateAudioInterfaceAsync " + Hex(hr));
                if (!handler.Done.WaitOne(10000)) throw new RecorderFailure("activation failed: no completion within 10 s");
                if (handler.Result < 0 || handler.Client == null)
                    throw new RecorderFailure("activation failed: " + Hex(handler.Result) + " for pid " + pid);
                GC.KeepAlive(operation);
                return (IAudioClient)handler.Client;
            }
            finally
            {
                Marshal.FreeHGlobal(variant);
                Marshal.FreeHGlobal(blob);
            }
        }

        IAudioClient Initialize()
        {
            IntPtr format = Marshal.AllocHGlobal(20);
            try
            {
                // WAVEFORMATEX: PCM, 2 channels, 48 kHz, 16-bit. GetMixFormat is not
                // supported on the process loopback device, so the format is given.
                Marshal.WriteInt16(format, 0, 1);
                Marshal.WriteInt16(format, 2, Channels);
                Marshal.WriteInt32(format, 4, SampleRate);
                Marshal.WriteInt32(format, 8, SampleRate * BlockAlign);
                Marshal.WriteInt16(format, 12, BlockAlign);
                Marshal.WriteInt16(format, 14, Bits);
                Marshal.WriteInt16(format, 16, 0);
                IAudioClient client = Activate();
                int hr = client.Initialize(0, StreamLoopback | StreamEventCallback, 2000000, 0, format, IntPtr.Zero);
                if (hr >= 0) return client;
                // Some engines want to convert the format themselves; try once more on a fresh client.
                Marshal.ReleaseComObject(client);
                client = Activate();
                int hr2 = client.Initialize(0, StreamLoopback | StreamEventCallback | StreamAutoConvertPcm | StreamSrcDefaultQuality,
                                            2000000, 0, format, IntPtr.Zero);
                if (hr2 >= 0) return client;
                throw new RecorderFailure("format failed: IAudioClient.Initialize " + Hex(hr) + ", with conversion " + Hex(hr2));
            }
            finally
            {
                Marshal.FreeHGlobal(format);
            }
        }

        bool TargetAlive()
        {
            try
            {
                Process.GetProcessById(pid).Dispose();
                return true;
            }
            catch (ArgumentException)
            {
                return false;
            }
            catch (InvalidOperationException)
            {
                return false;
            }
        }

        // ---------------------------------------------------------------- recording

        int Record()
        {
            if (!TargetAlive()) throw new RecorderFailure("no process with pid " + pid);
            if (File.Exists(outPath)) throw new RecorderFailure("refusing to overwrite the recording " + outPath);
            QueryPerformanceFrequency(out qpcFrequency);

            IAudioClient client = Initialize();
            Guid captureIid = typeof(IAudioCaptureClient).GUID;
            object service;
            int hr = client.GetService(ref captureIid, out service);
            if (hr < 0) throw new RecorderFailure("format failed: GetService(IAudioCaptureClient) " + Hex(hr));
            IAudioCaptureClient capture = (IAudioCaptureClient)service;
            AutoResetEvent packetReady = new AutoResetEvent(false);
            hr = client.SetEventHandle(packetReady.SafeWaitHandle.DangerousGetHandle());
            if (hr < 0) throw new RecorderFailure("format failed: SetEventHandle " + Hex(hr));

            wav = new FileStream(outPath, FileMode.CreateNew, FileAccess.Write, FileShare.Read | FileShare.Delete);
            wav.Write(Header(0xFFFFFFFF, 0xFFFFFFFF), 0, 44);

            SampleClock();
            originHns = NowHns();
            firstUtc = UtcOf(originHns);
            Anchor(firstUtc);
            hr = client.Start();
            if (hr < 0) throw new RecorderFailure("activation failed: IAudioClient.Start " + Hex(hr));

            Stopwatch clock = Stopwatch.StartNew();
            long lastFlush = 0, lastAnchor = 0, lastPacket = 0, lastAliveCheck = 0;
            long maxMs = (long)(maxMinutes * 60000);
            bool ready = false, anchorDue = false, sawPacket = false;
            string reason = null;
            int exitCode = 0;

            while (reason == null)
            {
                packetReady.WaitOne(100);
                while (reason == null)
                {
                    uint next;
                    hr = capture.GetNextPacketSize(out next);
                    if (hr < 0) { reason = hr == DeviceInvalidated ? "device-invalidated" : "capture-error " + Hex(hr); break; }
                    if (next == 0) break;
                    IntPtr data;
                    uint frames, flags;
                    ulong devicePosition, qpcPosition;
                    hr = capture.GetBuffer(out data, out frames, out flags, out devicePosition, out qpcPosition);
                    if (hr < 0) { reason = hr == DeviceInvalidated ? "device-invalidated" : "capture-error " + Hex(hr); break; }
                    if (hr == BufferEmpty) break;

                    if ((flags & FlagTimestampError) == 0 && qpcPosition != 0)
                    {
                        long slot = FrameOf((long)qpcPosition);
                        long ahead = slot - written;
                        if (ahead > GapToleranceFrames) Gap(ahead);
                        else if (ahead < -GapToleranceFrames) anchorDue = true;
                        if (anchorDue)
                        {
                            Anchor(UtcOf((long)qpcPosition));
                            anchorDue = false;
                            lastAnchor = clock.ElapsedMilliseconds;
                        }
                    }
                    int bytes = (int)frames * BlockAlign;
                    if (bytes > buffer.Length) buffer = new byte[bytes];
                    if ((flags & FlagSilent) != 0 || data == IntPtr.Zero) Array.Clear(buffer, 0, bytes);
                    else Marshal.Copy(data, buffer, 0, bytes);
                    wav.Write(buffer, 0, bytes);
                    written += frames;
                    capture.ReleaseBuffer(frames);
                    sawPacket = true;
                    lastPacket = clock.ElapsedMilliseconds;
                }

                long now = clock.ElapsedMilliseconds;
                if (!ready && (sawPacket || now >= 2000))
                {
                    Say("ready " + UtcOf(NowHns()));
                    ready = true;
                }
                if (now - lastPacket > 1000)
                {
                    long slot = FrameOf(NowHns() - Hns / 2);
                    if (slot - written > GapToleranceFrames) Gap(slot - written);
                }
                if (now - lastFlush >= 1000)
                {
                    wav.Flush();
                    lastFlush = now;
                }
                if (now - lastAnchor >= 10000)
                {
                    SampleClock();
                    anchorDue = true;
                    lastAnchor = now;
                    if (now - lastPacket > 1000)
                    {
                        // No packets to anchor on: the zero fill above sits at its slot.
                        Anchor(UtcOf(originHns + written * Hns / SampleRate));
                        anchorDue = false;
                    }
                }
                if (reason != null) break;
                if (File.Exists(stopFile)) reason = "stop-file";
                else if (now >= maxMs) reason = "max-minutes";
                else if (now - lastAliveCheck >= 1000)
                {
                    lastAliveCheck = now;
                    if (!TargetAlive()) reason = "target-exited";
                }
            }
            if (reason.StartsWith("capture-error")) exitCode = 1;

            client.Stop();
            long endHns = NowHns();
            long endSlot = FrameOf(endHns);
            if (endSlot - written > GapToleranceFrames) Gap(endSlot - written);
            string endUtc = UtcOf(endHns);
            anchors.Add("[" + written + ", \"" + endUtc + "\"]");
            long dataBytes = written * BlockAlign;
            wav.Flush();
            wav.Seek(0, SeekOrigin.Begin);
            uint dataField = dataBytes > 0xFFFFFFF0L ? 0xFFFFFFFFu : (uint)dataBytes;
            uint riffField = dataBytes > 0xFFFFFFF0L - 36 ? 0xFFFFFFFFu : (uint)(dataBytes + 36);
            wav.Write(Header(dataField, riffField), 0, 44);
            wav.Flush();
            wav.Dispose();
            WriteSidecar(endUtc, reason);
            Marshal.ReleaseComObject(capture);
            Marshal.ReleaseComObject(client);
            if (exitCode != 0) Say("error " + reason);
            else Say("stopped " + endUtc + " " + reason);
            return exitCode;
        }
    }
}
'@

function Get-GitTree([string]$path) {
    $dir = [IO.Path]::GetFullPath($path)
    while ($dir) {
        if (Test-Path -LiteralPath (Join-Path $dir '.git')) { return $dir }
        $parent = [IO.Path]::GetDirectoryName($dir)
        if (-not $parent -or $parent -eq $dir) { break }
        $dir = $parent
    }
    return $null
}

function Say([string]$line) {
    [Console]::Out.WriteLine($line)
    [Console]::Out.Flush()
}

try {
    Add-Type -TypeDefinition $source -Language CSharp
    if ($CompileOnly) {
        Say ([QuestTrackerOutside.Loopback]::CompileCheck())
        exit 0
    }
    if ($TargetPid -le 0) { Say 'error -TargetPid is required'; exit 1 }
    if (-not $Out) { Say 'error -Out is required'; exit 1 }
    if (-not $StopFile) { Say 'error -StopFile is required'; exit 1 }
    $outFull = [IO.Path]::GetFullPath($Out)
    $tree = Get-GitTree $outFull
    if ($tree) {
        Say ("error refusing to write the recording at $outFull`: it is inside the git checkout $tree. " +
             'Recordings stay outside every repository; pass a folder elsewhere.')
        exit 1
    }
    $parent = [IO.Path]::GetDirectoryName($outFull)
    if (-not (Test-Path -LiteralPath $parent)) { $null = New-Item -ItemType Directory -Path $parent }
    $code = [QuestTrackerOutside.Loopback]::Run($TargetPid, $outFull, [IO.Path]::GetFullPath($StopFile), $MaxMinutes)
    exit $code
} catch {
    Say ('error ' + ($_.Exception.Message -replace '\s+', ' '))
    exit 1
}
