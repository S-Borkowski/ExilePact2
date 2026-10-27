"""tools/quest_tracker/outside/audio: the WAV reader, the levels, the timeline and clips.

Nothing here plays a sound, opens an audio device or starts the recorder. The
recordings are synthetic WAV files written into temporary folders outside the
repository, with sidecars shaped like the ones `loopback.ps1` writes. The one
test that starts Windows PowerShell only compiles the recorder's C# and exits.

The baselines are what a reader must not get wrong on a plain file: a WAV
written and read back unchanged, a sized header that ends before trailing
chunks, a tone that is absent measuring as absent, and an out-of-band tone
staying out of the speech-band envelope. The targets are what the live session
needs: a header the recorder has not patched yet (placeholder sizes, a half
frame at the end) read to its end, two tones told apart by their Goertzel
levels, an envelope whose onset lands on the right 50 ms frame, a sample index
mapped to UTC across a zero-filled gap, and a clip cut at a UTC instant.
"""
import contextlib
import datetime
import io
import json
import math
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.quest_tracker.outside import audio, common  # noqa: E402

UTC = datetime.timezone.utc
RATE = 48000
T0 = datetime.datetime(2026, 10, 26, 10, 28, 0, tzinfo=UTC)


def sine(hz, seconds, amplitude, rate=RATE, phase=0.0):
    n = int(round(seconds * rate))
    w = 2 * math.pi * hz / rate
    return [amplitude * math.sin(w * i + phase) for i in range(n)]


def pcm16(mono, channels=2):
    """Mono floats in [-1, 1] as interleaved 16-bit PCM, the same value on every channel."""
    out = []
    for v in mono:
        s = max(-32768, min(32767, int(round(v * 32767))))
        out.extend([s] * channels)
    return struct.pack("<%dh" % len(out), *out)


def mix(*parts):
    n = max(len(p) for p in parts)
    return [sum(p[i] if i < len(p) else 0.0 for p in parts) for i in range(n)]


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.assertIsNone(common.git_tree_of(self.tmp), "the temp folder sits inside a git checkout")

    def tearDown(self):
        self._tmp.cleanup()


class WavRoundTrip(TempDir):
    def test_written_wav_reads_back_unchanged(self):
        mono = sine(440, 0.25, 0.1)
        data = pcm16(mono)
        path = self.tmp / "a.wav"
        audio.write_wav(path, data, RATE, 2, 16)
        reader = audio.WavReader(path)
        self.assertEqual((reader.sample_rate, reader.channels, reader.bits), (RATE, 2, 16))
        self.assertEqual(reader.frames, len(mono))
        self.assertTrue(reader.sized)
        self.assertEqual(reader.read_frames(0, reader.frames), data)
        back = reader.read_mono(100, 10)
        for got, want in zip(back, mono[100:110]):
            self.assertAlmostEqual(got, want, delta=1.0 / 32767)

    def test_sized_header_stops_before_a_trailing_chunk(self):
        # Baseline: a finished file's data size is honoured, so a chunk after
        # the data (a LIST tag here) is not read as samples.
        data = pcm16([0.5] * 100)
        path = self.tmp / "tail.wav"
        audio.write_wav(path, data, RATE, 2, 16)
        with open(path, "ab") as f:
            f.write(b"LIST" + struct.pack("<I", 8) + b"INFOabcd")
        reader = audio.WavReader(path)
        self.assertTrue(reader.sized)
        self.assertEqual(reader.frames, 100)

    def test_header_left_open_by_a_running_recorder_reads_to_eof(self):
        data = pcm16(sine(1000, 0.1, 0.2))
        frames = len(data) // 4
        for placeholder in (0xFFFFFFFF, 0):
            path = self.tmp / ("open-%x.wav" % placeholder)
            with open(path, "wb") as f:
                f.write(audio.wav_header(RATE, 2, 16, data_bytes=None, placeholder=placeholder))
                f.write(data)
                f.write(b"\x01\x02\x03")  # a frame the recorder is still writing
            reader = audio.WavReader(path)
            self.assertFalse(reader.sized, placeholder)
            self.assertEqual(reader.frames, frames, placeholder)
            self.assertEqual(reader.read_frames(0, frames), data)

    def test_extensible_header_and_extra_chunks_are_walked(self):
        # Media Foundation writes WAVE_FORMAT_EXTENSIBLE and may put chunks
        # before the data; the reader walks them.
        data = pcm16([0.25] * 50)
        sub = struct.pack("<IHH8s", 1, 0x0000, 0x0010, bytes.fromhex("800000aa00389b71"))
        fmt = struct.pack("<HHIIHHHHI", 0xFFFE, 2, RATE, RATE * 4, 4, 16, 22, 16, 3) + sub
        body = (b"WAVE" + b"fmt " + struct.pack("<I", len(fmt)) + fmt
                + b"junk" + struct.pack("<I", 3) + b"abc\x00"
                + b"data" + struct.pack("<I", len(data)) + data)
        path = self.tmp / "ext.wav"
        path.write_bytes(b"RIFF" + struct.pack("<I", len(body)) + body)
        reader = audio.WavReader(path)
        self.assertEqual((reader.channels, reader.bits, reader.frames), (2, 16, 50))
        self.assertAlmostEqual(reader.read_mono(0, 1)[0], 0.25, delta=1e-3)

    def test_trim_rewrites_an_overrun_decode_as_a_plain_wav(self):
        # The AudioGraph route writes silence past the input's end and may
        # leave extra chunks; the trim keeps the input's duration only.
        data = pcm16([0.25] * 300 + [0.0] * 700)
        path = self.tmp / "decoded.wav"
        with open(path, "wb") as f:
            f.write(audio.wav_header(RATE, 2, 16, data_bytes=None, placeholder=0))
            f.write(data)
        audio.trim_wav(path, 300)
        reader = audio.WavReader(path)
        self.assertTrue(reader.sized)
        self.assertEqual(reader.frames, 300)
        self.assertEqual(reader.read_frames(0, 300), data[:1200])

    def test_writer_refuses_a_git_checkout(self):
        tree = self.tmp / "checkout"
        (tree / ".git").mkdir(parents=True)
        with self.assertRaises(common.ToolError):
            audio.write_wav(tree / "x.wav", b"\x00" * 4, RATE, 2, 16)
        audio.write_wav(self.tmp / "x.wav", b"\x00" * 4, RATE, 2, 16)  # the control


class Levels(unittest.TestCase):
    def test_tone_pcm_is_at_its_dbfs(self):
        data = audio.sine_pcm(440, 0.5, -20.0)
        values = struct.unpack("<%dh" % (len(data) // 2), data)
        left = [v / 32768.0 for v in values[0::2]]
        self.assertAlmostEqual(audio.goertzel_db(left[2400:-2400], RATE, 440), -20.0, delta=0.2)

    def test_goertzel_separates_two_tones(self):
        mono = mix(sine(440, 1.0, 0.1), sine(1000, 1.0, 0.001))  # -20 and -60 dBFS
        self.assertAlmostEqual(audio.goertzel_db(mono, RATE, 440), -20.0, delta=0.3)
        self.assertAlmostEqual(audio.goertzel_db(mono, RATE, 1000), -60.0, delta=1.0)

    def test_absent_tone_measures_absent(self):
        mono = sine(440, 1.0, 0.1)
        self.assertLess(audio.goertzel_db(mono, RATE, 1000), -80.0)
        self.assertLess(audio.goertzel_db([0.0] * 4800, RATE, 440), -140.0)

    def test_selftest_rule(self):
        ok = audio.selftest_verdict(-20.5, -90.0, 0.040)
        self.assertTrue(ok["pass"])
        self.assertFalse(audio.selftest_verdict(-20.5, -35.0, 0.040)["pass"])   # 14.5 dB apart
        self.assertFalse(audio.selftest_verdict(-20.5, -90.0, 0.200)["pass"])   # late onset
        self.assertFalse(audio.selftest_verdict(-20.5, -90.0, None)["pass"])    # no onset
        self.assertFalse(audio.selftest_verdict(-150.0, -150.0, None)["pass"])  # blind recorder
        line = audio.rehearsal_line(ok)
        self.assertTrue(line.startswith("- i2-rehearsal | expected: "), line)
        self.assertTrue(line.endswith("| pass"), line)


class Envelope(TempDir):
    def burst_wav(self, hz, start=0.8, length=0.5, total=2.0):
        mono = [0.0] * int(start * RATE) + sine(hz, length, 0.1)
        mono += [0.0] * (int(total * RATE) - len(mono))
        path = self.tmp / ("burst-%d.wav" % hz)
        audio.write_wav(path, pcm16(mono), RATE, 2, 16)
        return path

    def test_onset_lands_on_its_50ms_frame(self):
        env = audio.band_envelope_db(audio.WavReader(self.burst_wav(1000)))
        self.assertEqual(len(env), 40)
        loud = [i for i, db in enumerate(env) if db > -40.0]
        self.assertEqual(loud[0], 16)   # 0.80 s / 50 ms
        self.assertEqual(loud[-1], 25)  # the frame ending at 1.30 s
        self.assertAlmostEqual(env[20], -23.0, delta=1.5)  # a -20 dBFS sine's power
        self.assertLess(env[5], -100.0)

    def test_out_of_band_tone_stays_out_of_the_envelope(self):
        env = audio.band_envelope_db(audio.WavReader(self.burst_wav(6000)))
        self.assertLess(max(env), -45.0)

    def test_streaming_matches_one_block(self):
        reader = audio.WavReader(self.burst_wav(1000))
        whole = audio.band_envelope_db(reader)
        env = audio.BandEnvelope(RATE)
        parts = []
        for start in range(0, reader.frames, 7001):
            parts += env.feed(reader.read_mono(start, 7001))
        self.assertEqual(len(parts), len(whole))
        for a, b in zip(parts, whole):
            self.assertAlmostEqual(a, b, delta=1e-6)

    def test_window_of_a_recording(self):
        reader = audio.WavReader(self.burst_wav(1000))
        env = audio.band_envelope_db(reader, start=int(0.5 * RATE), count=RATE)
        self.assertEqual(len(env), 20)
        self.assertEqual([i for i, db in enumerate(env) if db > -40.0][0], 6)


def sidecar(**extra):
    data = {
        "pid": 4242, "sample_rate": RATE, "channels": 2, "bits": 16,
        "first_sample_utc": common.format_utc(T0),
        "anchors": [[0, common.format_utc(T0)]],
        "gaps": [], "end_utc": None, "end_reason": None,
    }
    data.update(extra)
    return data


def at(seconds):
    return T0 + datetime.timedelta(seconds=seconds)


class Timeline(unittest.TestCase):
    def test_filled_gap_keeps_the_index_linear(self):
        # 10 s recorded, a 0.25 s silence the recorder zero-filled, 10 s more.
        tl = audio.Timeline.from_sidecar(sidecar(
            anchors=[[0, common.format_utc(T0)], [480000, common.format_utc(at(10))],
                     [492000, common.format_utc(at(10.25))], [972000, common.format_utc(at(20.25))]],
            gaps=[[480000, 12000]]))
        self.assertEqual(tl.utc_of(486000), at(10.125))
        self.assertTrue(tl.in_gap(486000))
        self.assertFalse(tl.in_gap(492000))
        self.assertFalse(tl.in_gap(479999))
        self.assertAlmostEqual(tl.sample_of(at(15.25)), 732000, delta=0.5)
        self.assertAlmostEqual(tl.sample_of(at(10.125)), 486000, delta=0.5)

    def test_anchors_carry_a_drifting_clock(self):
        # The device ran 1 sample in 48000 fast against UTC.
        tl = audio.Timeline.from_sidecar(sidecar(
            anchors=[[0, common.format_utc(T0)], [480010, common.format_utc(at(10))]]))
        self.assertAlmostEqual(tl.sample_of(at(5)), 240005, delta=0.5)
        # Past the last anchor the nominal rate carries on.
        self.assertAlmostEqual(tl.sample_of(at(11)), 480010 + 48000, delta=0.5)
        self.assertEqual(tl.utc_of(480010 + 48000), at(11))

    def test_no_anchors_uses_the_first_sample(self):
        tl = audio.Timeline.from_sidecar(sidecar(anchors=[]))
        self.assertEqual(tl.utc_of(24000), at(0.5))
        self.assertAlmostEqual(tl.sample_of(at(2)), 96000, delta=0.5)


class Clip(TempDir):
    def recording(self):
        mono = [0.0] * (6 * RATE)
        for i, v in enumerate(sine(1000, 0.1, 0.5)):
            mono[3 * RATE + i] = v
        wav = self.tmp / "game.wav"
        with open(wav, "wb") as f:  # left open, like a recorder still running
            f.write(audio.wav_header(RATE, 2, 16, data_bytes=None))
            f.write(pcm16(mono))
        common.write_json(Path(str(wav) + ".json"), sidecar())
        return wav

    def test_clip_at_a_utc_instant(self):
        wav = self.recording()
        out = self.tmp / "kill-clip.wav"
        info = audio.clip(wav, at(3.0), 1.0, 1.0, out)
        reader = audio.WavReader(out)
        self.assertTrue(reader.sized)
        self.assertEqual(reader.frames, 2 * RATE)
        self.assertEqual(info["frames"], 2 * RATE)
        mono = reader.read_mono(0, reader.frames)
        first = next(i for i, v in enumerate(mono) if abs(v) > 0.01)
        self.assertLess(abs(first - RATE), 10)
        side = common.read_json(Path(str(out) + ".json"))
        self.assertEqual(side["first_sample_utc"], common.format_utc(at(2.0)))
        self.assertEqual(audio.Timeline.from_sidecar(side).utc_of(RATE), at(3.0))

    def test_clip_is_cut_short_at_the_recording_edge(self):
        wav = self.recording()
        info = audio.clip(wav, at(0.5), 5.0, 1.0, self.tmp / "edge.wav")
        self.assertEqual(info["frames"], int(1.5 * RATE))

    def test_clip_outside_the_recording_is_refused(self):
        wav = self.recording()
        with self.assertRaises(common.ToolError):
            audio.clip(wav, at(60), 1.0, 1.0, self.tmp / "none.wav")

    def test_clip_command_line(self):
        wav = self.recording()
        out = self.tmp / "bark-clip.wav"
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = audio.main(["clip", "--wav", str(wav), "--at", common.format_utc(at(3)),
                               "--before", "0.5", "--after", "0.5", "--out", str(out)])
        self.assertEqual(code, 0, buf.getvalue())
        self.assertEqual(audio.WavReader(out).frames, RATE)


class CommandLine(TempDir):
    def test_decode_ref_refuses_a_missing_input_without_a_decoder_run(self):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = audio.main(["decode-ref", "--ogg", str(self.tmp / "missing.ogg"),
                               "--out", str(self.tmp / "out.wav")])
        self.assertEqual(code, 1)
        self.assertIn("missing.ogg", err.getvalue())

    def test_tone_log_refuses_a_git_checkout(self):
        tree = self.tmp / "checkout"
        (tree / ".git").mkdir(parents=True)
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            code = audio.main(["tone", "--hz", "440", "--seconds", "0.1", "--dbfs", "-20",
                               "--log", str(tree / "tone.jsonl")])
        self.assertEqual(code, 1)
        self.assertIn("git checkout", err.getvalue())


@unittest.skipUnless(sys.platform == "win32", "the recorder is Windows PowerShell")
class RecorderCompiles(unittest.TestCase):
    def test_recorder_source_compiles(self):
        proc = subprocess.run(
            ["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
             "-File", str(audio.LOOPBACK_PS1), "-CompileOnly"],
            capture_output=True, text=True, timeout=120)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("compiled", proc.stdout)


if __name__ == "__main__":
    unittest.main()
