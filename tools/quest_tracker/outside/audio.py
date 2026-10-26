"""The game's audio, recorded from outside: the tone probe, clips, the reference decode.

Developer-only. Usage:

    py -3 -m tools.quest_tracker.outside.audio tone --hz F --seconds S [--dbfs D]
          [--start-after T] [--log J] [--go-file G]
    py -3 -m tools.quest_tracker.outside.audio selftest [--out DIR]
    py -3 -m tools.quest_tracker.outside.audio clip --wav W --at <utc> [--before S]
          [--after S] --out O
    py -3 -m tools.quest_tracker.outside.audio decode-ref --ogg O --out W

The recorder itself is `loopback.ps1` beside this file: WASAPI process
loopback of one process tree into a 48 kHz, 2-channel, 16-bit WAV whose sample
index stays linear in UTC, with a sidecar `<wav>.json` (`pid`, `sample_rate`,
`channels`, `bits`, `first_sample_utc`, `anchors` as `[sample, utc]` pairs,
`gaps` as `[sample, zeros written]` pairs, `end_utc`, `end_reason`).

- `tone` plays a sine through `winsound.PlaySound` from memory and logs its
  pid and the UTC it started at to the JSON-lines file `J`. With `--go-file`
  it first waits for that file to appear, so a caller can start the recorder
  on its pid before the tone's `--start-after` begins to count.
- `selftest` is the recorder's positive and negative control in one run: two
  tone processes, A at 440 Hz and B at 1000 Hz, both at -20 dBFS for 3 s,
  starting 1.5 s after their go signal, and the recorder on A's process tree
  for 5 s. It passes when the 440 Hz level is at least 20 dB above the
  1000 Hz level and A's onset in the recording is within 150 ms of A's logged
  start. With `--out` the files stay there and the `i2-rehearsal` capture
  line is printed. Exit 0 on a pass, 1 otherwise. Measured on this machine
  (2026-10-26, five runs): A at -33.5 dBFS in the recording against -20
  played, so the recorded level follows an output volume; B at -129 to
  -133 dBFS; A's onset +95 to +115 ms after its logged start. That delay
  most likely belongs to `PlaySound`'s own start-up rather than to the
  recorder's timestamps (the two are not separated), and it leaves about
  35 ms of the 150 ms rule.
- `clip` cuts `[at - before, at + after]` out of a recording, through its
  sidecar's sample-to-UTC mapping, into a WAV with its own sidecar.
- `decode-ref` turns a reference `.ogg` into 48 kHz, 2-channel, 16-bit PCM
  through Windows' own Media Foundation, from Windows PowerShell's WinRT:
  `MediaTranscoder` first, then an `AudioGraph` from a file input node to a
  file output node (on this machine only the second opens Ogg Vorbis). It
  downloads and installs nothing; when neither route can decode the file it
  exits 2 with `no decoder` and both reasons.

The library half (`WavReader`, `goertzel_db`, `BandEnvelope` and
`band_envelope_db`, `Timeline`) is what the session analysis imports. All of
it is stdlib (there is no `audioop` in this Python). Every writer refuses a
path inside a git checkout: recordings, clips and decoded references stay in
the live folder.
"""
from __future__ import annotations

import argparse
import base64
import bisect
import datetime as _dt
import json
import math
import os
import queue
import struct
import subprocess
import sys
import tempfile
import threading
import time
from array import array
from operator import add, mul
from pathlib import Path

from tools.quest_tracker.outside import common
from tools.quest_tracker.outside.common import ToolError

ROOT = Path(__file__).resolve().parents[3]
MODULE = "tools.quest_tracker.outside.audio"
LOOPBACK_PS1 = Path(__file__).resolve().with_name("loopback.ps1")
POWERSHELL = "powershell.exe"
PS_ARGS = ("-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass")

# The recorder's format (the live folder contract: `i2\game.wav`).
SAMPLE_RATE = 48000
CHANNELS = 2
BITS = 16
# A header the recorder has not patched yet carries this in both size fields.
OPEN_SIZE = 0xFFFFFFFF

# The speech-band envelope: 300-3400 Hz energy in 50 ms frames, in dB.
BAND_LO_HZ = 300.0
BAND_HI_HZ = 3400.0
ENVELOPE_FRAME_S = 0.05
FLOOR_DB = -120.0       # an envelope frame of silence
LEVEL_FLOOR_DB = -160.0  # a Goertzel level of silence

# The selftest (the workorder's "Loopback recorder").
TONE_A_HZ = 440.0
TONE_B_HZ = 1000.0
TONE_DBFS = -20.0
TONE_SECONDS = 3.0
TONE_START_AFTER_S = 1.5
RECORD_SECONDS = 5.0
SEPARATION_DB = 20.0
ONSET_TOLERANCE_S = 0.150
ONSET_FRAME_S = 0.010
GO_TIMEOUT_S = 60.0
READY_TIMEOUT_S = 60.0


class NoDecoder(ToolError):
    """`decode-ref` could not decode: exit 2, never a fallback download."""


# --------------------------------------------------------------------------
# WAV writing

def wav_header(rate: int, channels: int, bits: int, data_bytes: int | None,
               placeholder: int = OPEN_SIZE, fmt_tag: int = 1) -> bytes:
    """A 44-byte RIFF/WAVE header. `data_bytes=None` writes `placeholder` sizes,
    the shape of a file still being written."""
    block = channels * bits // 8
    fmt = struct.pack("<HHIIHH", fmt_tag, channels, rate, rate * block, block, bits)
    if data_bytes is None:
        riff_size = data_size = placeholder
    else:
        data_size = data_bytes
        riff_size = 4 + (8 + len(fmt)) + 8 + data_bytes + (data_bytes & 1)
    return (b"RIFF" + struct.pack("<I", riff_size) + b"WAVE"
            + b"fmt " + struct.pack("<I", len(fmt)) + fmt
            + b"data" + struct.pack("<I", data_size))


def write_wav(path: Path, pcm: bytes, rate: int, channels: int, bits: int) -> None:
    path = Path(path)
    common.refuse_inside_git(path, "WAV file")
    with open(path, "wb") as f:
        f.write(wav_header(rate, channels, bits, len(pcm)))
        f.write(pcm)
        if len(pcm) & 1:
            f.write(b"\x00")


def sine_pcm(hz: float, seconds: float, dbfs: float, rate: int = SAMPLE_RATE,
             channels: int = CHANNELS, ramp_s: float = 0.005) -> bytes:
    """A 16-bit sine at `dbfs` (peak, relative to full scale), with short linear
    ramps at both ends so it starts and stops without a click."""
    n = max(0, int(round(seconds * rate)))
    amp = 32767.0 * 10.0 ** (dbfs / 20.0)
    ramp = max(1, int(ramp_s * rate))
    w = 2.0 * math.pi * hz / rate
    out = array("h")
    for i in range(n):
        g = min(1.0, i / ramp, (n - 1 - i) / ramp)
        s = int(round(amp * g * math.sin(w * i)))
        for _ in range(channels):
            out.append(s)
    if sys.byteorder != "little":
        out.byteswap()
    return out.tobytes()


# --------------------------------------------------------------------------
# WAV reading

class WavReader:
    """A PCM or float WAV, read in place.

    The chunks are walked to `fmt ` and `data`. A data size the recorder has
    not patched yet (0 or 0xFFFFFFFF), or one larger than the file, means the
    data runs to the end of the file, whole frames only; `sized` says which.
    A sized data chunk is honoured, so a chunk after it is not read as samples.
    The frame count is fixed when the reader is made; make a new one to see a
    growing file's newer frames.
    """

    def __init__(self, path: Path):
        self.path = Path(path)
        size = self.path.stat().st_size
        with open(self.path, "rb") as f:
            head = f.read(12)
            if len(head) < 12 or head[:4] != b"RIFF" or head[8:12] != b"WAVE":
                raise ToolError("%s is not a RIFF/WAVE file" % self.path)
            fmt = None
            pos = 12
            while True:
                f.seek(pos)
                hdr = f.read(8)
                if len(hdr) < 8:
                    raise ToolError("%s has no data chunk" % self.path)
                cid, csize = hdr[:4], struct.unpack("<I", hdr[4:])[0]
                if cid == b"fmt ":
                    fmt = f.read(min(csize, 64))
                elif cid == b"data":
                    self.data_offset = pos + 8
                    data_size = csize
                    break
                if csize == OPEN_SIZE:
                    raise ToolError("%s has an open chunk %r before its data" % (self.path, cid))
                pos += 8 + csize + (csize & 1)
        if fmt is None or len(fmt) < 16:
            raise ToolError("%s has no fmt chunk before its data" % self.path)
        tag, self.channels, self.sample_rate, _rate, self.block_align, self.bits = \
            struct.unpack("<HHIIHH", fmt[:16])
        if tag == 0xFFFE and len(fmt) >= 28:
            tag = struct.unpack("<I", fmt[24:28])[0] & 0xFFFF
        self.fmt_tag = tag
        ok = (tag == 1 and self.bits in (8, 16, 24, 32)) or (tag == 3 and self.bits == 32)
        if not ok or self.channels < 1 or self.block_align != self.channels * self.bits // 8:
            raise ToolError("%s: unsupported format (tag %d, %d bits, %d channels)"
                            % (self.path, tag, self.bits, self.channels))
        remaining = max(0, size - self.data_offset)
        if data_size in (0, OPEN_SIZE) or data_size > remaining:
            self.sized = False
            nbytes = remaining
        else:
            self.sized = True
            nbytes = data_size
        self.frames = nbytes // self.block_align

    @property
    def seconds(self) -> float:
        return self.frames / float(self.sample_rate)

    def read_frames(self, start: int, count: int) -> bytes:
        """Raw interleaved bytes of frames `[start, start + count)`, clamped."""
        start = max(0, int(start))
        end = min(self.frames, start + max(0, int(count)))
        if end <= start:
            return b""
        with open(self.path, "rb") as f:
            f.seek(self.data_offset + start * self.block_align)
            return f.read((end - start) * self.block_align)

    def _values(self, raw: bytes):
        """Interleaved samples as numbers, and the full-scale value."""
        if self.fmt_tag == 3:
            a = array("f")
            a.frombytes(raw)
            if sys.byteorder != "little":
                a.byteswap()
            return a, 1.0
        if self.bits == 8:
            return [b - 128 for b in raw], 128.0
        if self.bits == 24:
            n = len(raw) // 3
            return [int.from_bytes(raw[3 * i:3 * i + 3], "little", signed=True)
                    for i in range(n)], 8388608.0
        a = array("h" if self.bits == 16 else "i")
        a.frombytes(raw)
        if sys.byteorder != "little":
            a.byteswap()
        return a, (32768.0 if self.bits == 16 else 2147483648.0)

    def read_mono(self, start: int, count: int) -> list[float]:
        """Frames `[start, start + count)` as the channels' mean, in [-1, 1]."""
        values, full = self._values(self.read_frames(start, count))
        ch = self.channels
        if ch == 1:
            scale = 1.0 / full
            return [v * scale for v in values]
        scale = 1.0 / (full * ch)
        if ch == 2:
            return [v * scale for v in map(add, values[0::2], values[1::2])]
        return [sum(t) * scale for t in zip(*(values[c::ch] for c in range(ch)))]


# --------------------------------------------------------------------------
# Levels

def goertzel_db(samples, rate: float, hz: float) -> float:
    """The level of a sine at `hz` in `samples` (in [-1, 1]), in dB relative to
    a full-scale sine: a Hann-windowed Goertzel filter, any frequency."""
    n = len(samples)
    if n < 2:
        return LEVEL_FLOOR_DB
    w = 2.0 * math.pi * hz / rate
    coeff = 2.0 * math.cos(w)
    step = 2.0 * math.pi / (n - 1)
    s1 = s2 = 0.0
    wsum = 0.0
    for i, x in enumerate(samples):
        h = 0.5 - 0.5 * math.cos(step * i)
        wsum += h
        s = x * h + coeff * s1 - s2
        s2 = s1
        s1 = s
    power = s1 * s1 + s2 * s2 - coeff * s1 * s2
    amp = 2.0 * math.sqrt(max(power, 0.0)) / wsum if wsum > 0 else 0.0
    if amp <= 1e-8:
        return LEVEL_FLOOR_DB
    return 20.0 * math.log10(amp)


def _boxcar3_taps(d: int) -> list[float]:
    """Three boxcars of length `d` in a row, normalised to unit gain at DC."""
    taps = [1.0]
    for _ in range(3):
        nxt = [0.0] * (len(taps) + d - 1)
        for i, t in enumerate(taps):
            for j in range(d):
                nxt[i + j] += t
        taps = nxt
    g = float(d ** 3)
    return [t / g for t in taps]


def _biquad(kind: str, f0: float, fs: float, q: float = 1.0 / math.sqrt(2.0)):
    """RBJ cookbook second-order section, normalised: (b0, b1, b2, a1, a2)."""
    w0 = 2.0 * math.pi * f0 / fs
    cw, alpha = math.cos(w0), math.sin(w0) / (2.0 * q)
    if kind == "hp":
        b0, b1, b2 = (1 + cw) / 2, -(1 + cw), (1 + cw) / 2
    else:
        b0, b1, b2 = (1 - cw) / 2, 1 - cw, (1 - cw) / 2
    a0 = 1 + alpha
    return b0 / a0, b1 / a0, b2 / a0, (-2 * cw) / a0, (1 - alpha) / a0


class BandEnvelope:
    """The energy of one band in fixed frames, in dB, fed block by block.

    The mono signal is first decimated by the largest factor `d` (at most 8)
    that keeps the band under the new Nyquist with some margin; the
    anti-alias filter is three length-`d` boxcars, which leaves content above
    the new Nyquist about 16 to 30 dB down and softens the band's top edge by
    a few dB. A second-order high-pass at `lo` and low-pass at `hi` then shape
    the band. Frame `i` covers input samples `[i * frame, (i + 1) * frame)`,
    the filters are causal and start from silence, and the result does not
    depend on how the input is split into blocks.
    """

    def __init__(self, rate: int, lo: float = BAND_LO_HZ, hi: float = BAND_HI_HZ,
                 frame_s: float = ENVELOPE_FRAME_S):
        self.rate = int(rate)
        self.frame = int(round(frame_s * rate))
        if self.frame < 1:
            raise ValueError("frame of %r s is shorter than one sample" % frame_s)
        d = 1
        for cand in range(8, 0, -1):
            if rate % cand == 0 and self.frame % cand == 0 and rate / cand >= 2.2 * hi:
                d = cand
                break
        self.d = d
        self.taps = _boxcar3_taps(d) if d > 1 else [1.0]
        self.hist = [0.0] * (len(self.taps) - 1)
        self.pending: list[float] = []
        fs = rate / float(d)
        self.hp = _biquad("hp", lo, fs)
        self.lp = _biquad("lp", min(hi, 0.45 * fs), fs)
        self.state = [0.0] * 8
        self.dframe = self.frame // d
        self.partial: list[float] = []

    def _decimate(self, block) -> list[float]:
        k = len(self.hist)
        buf = self.hist + self.pending + list(block)
        m = (len(buf) - k) // self.d
        if m <= 0:
            self.pending = buf[k:]
            return []
        d = self.d
        acc = [0.0] * m
        for j, h in enumerate(self.taps):
            first = k + d - 1 - j
            acc = list(map(add, acc, map(h.__mul__, buf[first:first + d * (m - 1) + 1:d])))
        used = k + m * d
        self.hist = buf[used - k:used] if k else []
        self.pending = buf[used:]
        return acc

    def _filter(self, xs) -> list[float]:
        hb0, hb1, hb2, ha1, ha2 = self.hp
        lb0, lb1, lb2, la1, la2 = self.lp
        x1, x2, y1, y2, z1, z2, w1, w2 = self.state
        out = []
        push = out.append
        for x in xs:
            y = hb0 * x + hb1 * x1 + hb2 * x2 - ha1 * y1 - ha2 * y2
            x2, x1, y2, y1 = x1, x, y1, y
            w = lb0 * y + lb1 * z1 + lb2 * z2 - la1 * w1 - la2 * w2
            z2, z1, w2, w1 = z1, y, w1, w
            push(w)
        self.state = [x1, x2, y1, y2, z1, z2, w1, w2]
        return out

    def feed(self, mono) -> list[float]:
        """Feed mono samples in [-1, 1]; returns the frames completed, in dB."""
        ys = self.partial + self._filter(self._decimate(mono))
        n = self.dframe
        whole = len(ys) // n
        out = []
        for i in range(whole):
            fr = ys[i * n:(i + 1) * n]
            ms = sum(map(mul, fr, fr)) / n
            out.append(10.0 * math.log10(ms) if ms > 1e-12 else FLOOR_DB)
        self.partial = ys[whole * n:]
        return out


def band_envelope_db(reader: WavReader, start: int = 0, count: int | None = None,
                     lo: float = BAND_LO_HZ, hi: float = BAND_HI_HZ,
                     frame_s: float = ENVELOPE_FRAME_S, block_s: float = 10.0) -> list[float]:
    """The band envelope of frames `[start, start + count)` of a WAV (to its end
    when `count` is None), read in blocks. Frame `i` starts at sample
    `start + i * round(frame_s * rate)`."""
    env = BandEnvelope(reader.sample_rate, lo, hi, frame_s)
    end = reader.frames if count is None else min(reader.frames, start + count)
    block = max(env.frame, int(block_s * reader.sample_rate) // env.frame * env.frame)
    out: list[float] = []
    pos = max(0, start)
    while pos < end:
        n = min(block, end - pos)
        out += env.feed(reader.read_mono(pos, n))
        pos += n
    return out


# --------------------------------------------------------------------------
# The sidecar's timeline

def sidecar_path(wav: Path) -> Path:
    return Path(str(wav) + ".json")


class Timeline:
    """A recording's sample index mapped to UTC through its sidecar.

    The points are `(0, first_sample_utc)` and every anchor; between two
    points the mapping is linear, and outside them it carries on at the
    nominal rate from the nearest point. Gaps the recorder zero-filled are
    part of the index (that is what keeps it linear); `in_gap` says whether
    a sample is one of those zeros.
    """

    def __init__(self, sample_rate: int, first_sample_utc: _dt.datetime, anchors=(), gaps=()):
        self.rate = float(sample_rate)
        self.origin = first_sample_utc
        points = {0: 0.0}
        for sample, when in anchors:
            when = common.parse_utc(when) if isinstance(when, str) else when
            points[int(sample)] = (when - first_sample_utc).total_seconds()
        self._s = sorted(points)
        self._t = [points[s] for s in self._s]
        self.gaps = sorted((int(s), int(n)) for s, n in gaps)

    @classmethod
    def from_sidecar(cls, data: dict) -> "Timeline":
        return cls(int(data["sample_rate"]), common.parse_utc(data["first_sample_utc"]),
                   data.get("anchors") or (), data.get("gaps") or ())

    @classmethod
    def of_wav(cls, wav: Path) -> "Timeline":
        path = sidecar_path(wav)
        if not path.is_file():
            raise ToolError("no sidecar %s beside the recording" % path)
        return cls.from_sidecar(common.read_json(path))

    def _seconds_of(self, sample: float) -> float:
        s, t = self._s, self._t
        i = bisect.bisect_right(s, sample) - 1
        if 0 <= i < len(s) - 1:
            return t[i] + (sample - s[i]) * (t[i + 1] - t[i]) / (s[i + 1] - s[i])
        j = 0 if i < 0 else len(s) - 1
        return t[j] + (sample - s[j]) / self.rate

    def utc_of(self, sample: float) -> _dt.datetime:
        return self.origin + _dt.timedelta(microseconds=round(self._seconds_of(sample) * 1e6))

    def sample_of(self, when: _dt.datetime) -> float:
        x = (when - self.origin).total_seconds()
        s, t = self._s, self._t
        i = bisect.bisect_right(t, x) - 1
        if 0 <= i < len(t) - 1 and t[i + 1] > t[i]:
            return s[i] + (x - t[i]) * (s[i + 1] - s[i]) / (t[i + 1] - t[i])
        j = 0 if i < 0 else len(t) - 1
        return s[j] + (x - t[j]) * self.rate

    def in_gap(self, sample: float) -> bool:
        return any(start <= sample < start + n for start, n in self.gaps)


# --------------------------------------------------------------------------
# Commands

def clip(wav: Path, at: _dt.datetime, before: float, after: float, out: Path) -> dict:
    """Cut `[at - before, at + after]` out of `wav` into `out`, with a sidecar."""
    wav, out = Path(wav), Path(out)
    common.refuse_inside_git(out, "clip")
    reader = WavReader(wav)
    tl = Timeline.of_wav(wav)
    s0 = int(round(tl.sample_of(at - _dt.timedelta(seconds=before))))
    s1 = int(round(tl.sample_of(at + _dt.timedelta(seconds=after))))
    a, b = max(0, s0), min(reader.frames, s1)
    if b <= a:
        raise ToolError("%s is outside the recording %s, which runs from %s to %s"
                        % (common.format_utc(at), wav, common.format_utc(tl.utc_of(0)),
                           common.format_utc(tl.utc_of(reader.frames))))
    write_wav(out, reader.read_frames(a, b - a), reader.sample_rate, reader.channels, reader.bits)
    gaps = [[max(s, a) - a, min(s + n, b) - max(s, a)] for s, n in tl.gaps if s < b and s + n > a]
    info = {
        "source": wav.name,
        "source_start_sample": a,
        "at_utc": common.format_utc(at),
        "sample_rate": reader.sample_rate,
        "channels": reader.channels,
        "bits": reader.bits,
        "first_sample_utc": common.format_utc(tl.utc_of(a)),
        "anchors": [[0, common.format_utc(tl.utc_of(a))], [b - a, common.format_utc(tl.utc_of(b))]],
        "gaps": gaps,
        "end_utc": common.format_utc(tl.utc_of(b)),
        "end_reason": "clip",
        "frames": b - a,
    }
    common.write_json(sidecar_path(out), info)
    return info


def play_memory(wav_bytes: bytes) -> None:
    import winsound
    winsound.PlaySound(wav_bytes, winsound.SND_MEMORY | winsound.SND_NODEFAULT)


def tone(hz: float, seconds: float, dbfs: float, start_after: float = 0.0,
         log: Path | None = None, go_file: Path | None = None, play=play_memory) -> dict:
    if log is not None:
        common.refuse_inside_git(Path(log), "tone log")
    pcm = sine_pcm(hz, seconds, dbfs)
    data = wav_header(SAMPLE_RATE, CHANNELS, BITS, len(pcm)) + pcm
    if go_file is not None:
        deadline = time.monotonic() + GO_TIMEOUT_S
        while not Path(go_file).exists():
            if time.monotonic() > deadline:
                raise ToolError("no go file %s within %d s" % (go_file, GO_TIMEOUT_S))
            time.sleep(0.005)
    if start_after > 0:
        time.sleep(start_after)
    record = {"pid": os.getpid(), "hz": hz, "seconds": seconds, "dbfs": dbfs,
              "start_utc": common.format_utc(common.now_utc())}
    if log is not None:
        common.append_jsonl(Path(log), record)
    play(data)
    return record


def selftest_verdict(level_a: float, level_b: float, onset_s: float | None) -> dict:
    """The selftest's rule: A's tone at least `SEPARATION_DB` above B's, and A's
    onset within `ONSET_TOLERANCE_S` of A's logged start."""
    sep = level_a - level_b
    onset_ok = onset_s is not None and abs(onset_s) <= ONSET_TOLERANCE_S
    return {"pass": sep >= SEPARATION_DB and onset_ok, "level_a": level_a, "level_b": level_b,
            "separation": sep, "onset_s": onset_s}


def rehearsal_line(v: dict) -> str:
    onset = "none" if v["onset_s"] is None else "%+d ms" % round(v["onset_s"] * 1000)
    return ("- i2-rehearsal | expected: %d Hz at least %d dB above %d Hz in A's recording, "
            "A's onset within %d ms of its logged start | observed: %d Hz %.1f dBFS, %d Hz "
            "%.1f dBFS (%.1f dB apart), onset %s | %s"
            % (TONE_A_HZ, SEPARATION_DB, TONE_B_HZ, ONSET_TOLERANCE_S * 1000,
               TONE_A_HZ, v["level_a"], TONE_B_HZ, v["level_b"], v["separation"], onset,
               "pass" if v["pass"] else "fail"))


def measure_selftest(wav: Path, start_a: _dt.datetime) -> dict:
    reader = WavReader(wav)
    tl = Timeline.of_wav(wav)
    rate = reader.sample_rate
    sa = int(round(tl.sample_of(start_a)))
    span = reader.read_mono(sa, int(TONE_SECONDS * rate)) if sa < reader.frames else []
    level_a = goertzel_db(span, rate, TONE_A_HZ)
    level_b = goertzel_db(span, rate, TONE_B_HZ)
    frame = int(ONSET_FRAME_S * rate)
    w0 = max(0, sa - rate)
    window = reader.read_mono(w0, 2 * rate)
    levels = [goertzel_db(window[i:i + frame], rate, TONE_A_HZ)
              for i in range(0, len(window) - frame + 1, frame)]
    onset_s = None
    if levels and max(levels) > -60.0:
        threshold = max(levels) - 10.0
        first = next(i for i, db in enumerate(levels) if db >= threshold)
        onset_s = (tl.utc_of(w0 + first * frame + frame // 2) - start_a).total_seconds()
    return selftest_verdict(level_a, level_b, onset_s)


def _pump(stream, lines: "queue.Queue[str | None]") -> None:
    for line in stream:
        lines.put(line.rstrip("\r\n"))
    lines.put(None)


def recorder_command(pid: int, out: Path, stop: Path, max_minutes: float) -> list[str]:
    return [POWERSHELL, *PS_ARGS, "-File", str(LOOPBACK_PS1), "-TargetPid", str(pid),
            "-Out", str(out), "-StopFile", str(stop), "-MaxMinutes", str(max_minutes)]


def _tone_command(hz: float, log: Path, go: Path) -> list[str]:
    return [sys.executable, "-m", MODULE, "tone", "--hz", str(hz), "--seconds", str(TONE_SECONDS),
            "--dbfs", str(TONE_DBFS), "--start-after", str(TONE_START_AFTER_S),
            "--log", str(log), "--go-file", str(go)]


def selftest(out: Path | None = None) -> int:
    if sys.platform != "win32":
        raise ToolError("the selftest needs Windows: process loopback and winsound")
    if out is None:
        with tempfile.TemporaryDirectory(prefix="qt-audio-selftest-") as tmp:
            return _selftest_in(Path(tmp), print_line=False)
    out = Path(out)
    common.refuse_inside_git(out, "selftest folder")
    out.mkdir(parents=True, exist_ok=True)
    return _selftest_in(out, print_line=True)


def _selftest_in(work: Path, print_line: bool) -> int:
    stamp = common.now_utc().strftime("%Y%m%dT%H%M%SZ")
    prefix = work / ("selftest-" + stamp)
    wav = Path(str(prefix) + ".wav")
    go, stop = Path(str(prefix) + "-go"), Path(str(prefix) + "-stop")
    log_a, log_b = Path(str(prefix) + "-a.jsonl"), Path(str(prefix) + "-b.jsonl")
    procs: list[subprocess.Popen] = []
    recorder = None
    lines: "queue.Queue[str | None]" = queue.Queue()
    said: list[str] = []
    try:
        with open(str(prefix) + "-tones.err", "w", encoding="utf-8") as err:
            a = subprocess.Popen(_tone_command(TONE_A_HZ, log_a, go), cwd=str(ROOT),
                                 stdout=subprocess.DEVNULL, stderr=err)
            procs.append(a)
            procs.append(subprocess.Popen(_tone_command(TONE_B_HZ, log_b, go), cwd=str(ROOT),
                                          stdout=subprocess.DEVNULL, stderr=err))
        recorder = subprocess.Popen(recorder_command(a.pid, wav, stop, 2), stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, encoding="utf-8",
                                    errors="replace")
        threading.Thread(target=_pump, args=(recorder.stdout, lines), daemon=True).start()
        deadline = time.monotonic() + READY_TIMEOUT_S
        while True:
            try:
                line = lines.get(timeout=max(0.1, deadline - time.monotonic()))
            except queue.Empty:
                raise ToolError("the recorder printed no ready line within %d s" % READY_TIMEOUT_S)
            if line is None:
                raise ToolError("the recorder ended before it was ready: %s"
                                % (" / ".join(said[-3:]) or "no output"))
            said.append(line)
            if line.startswith("ready "):
                break
            if line.startswith("error "):
                raise ToolError("the recorder failed: %s" % line[6:])
        go.touch()
        time.sleep(RECORD_SECONDS)
        stop.touch()
        try:
            recorder.wait(timeout=30)
        except subprocess.TimeoutExpired:
            raise ToolError("the recorder did not stop within 30 s of its stop file")
        while True:
            try:
                line = lines.get(timeout=5)
            except queue.Empty:
                break
            if line is None:
                break
            said.append(line)
        for p in procs:
            p.wait(timeout=15)
        if recorder.returncode != 0:
            raise ToolError("the recorder exited %d: %s" % (recorder.returncode, " / ".join(said[-3:])))
        starts = common.read_jsonl(log_a)
        if not starts:
            raise ToolError("tone A logged no start (%s)" % log_a)
        verdict = measure_selftest(wav, common.parse_utc(starts[0]["start_utc"]))
    finally:
        stop.touch(exist_ok=True)
        for p in procs + ([recorder] if recorder else []):
            if p.poll() is None:
                p.kill()
    stopped = next((s for s in said if s.startswith("stopped ")), "stopped: no line")
    onset = "none" if verdict["onset_s"] is None else "%+d ms" % round(verdict["onset_s"] * 1000)
    print("audio selftest: %d Hz %.1f dBFS, %d Hz %.1f dBFS, %.1f dB apart, onset %s; recorder %s"
          % (TONE_A_HZ, verdict["level_a"], TONE_B_HZ, verdict["level_b"], verdict["separation"],
             onset, stopped))
    if print_line:
        print("audio selftest: files %s.*" % prefix)
        print(rehearsal_line(verdict))
    print("audio selftest: %s" % ("pass" if verdict["pass"] else "fail"))
    return 0 if verdict["pass"] else 1


# Windows PowerShell 5.1 reaching Media Foundation through WinRT. Its inputs
# come in through the environment, so no path is ever spliced into the script.
#
# Two routes, both Media Foundation. `MediaTranscoder` decodes offline, but on
# this machine (measured 2026-10-26) it refuses Ogg Vorbis with reason
# `Unknown` while transcoding a WAV fine. `AudioGraph`'s file input node does
# open Ogg Vorbis (through the installed Web Media Extensions); with only file
# nodes the graph runs far faster than real time and keeps writing silence
# after the input ends, so it is stopped once the input's position stops
# moving and the caller trims the result to the duration printed here, in
# ticks of 100 ns (a culture-free number). Forced onto a WAV the transcoder
# also decodes, the graph's result matched it (normalized correlation 0.976,
# one sample apart) but about 13 dB quieter: its absolute level is not the
# file's, which a normalized correlation does not mind.
DECODE_PS = r"""
$ErrorActionPreference = 'Stop'
$why = @()
try {
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  $null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
  $null = [Windows.Storage.StorageFolder, Windows.Storage, ContentType = WindowsRuntime]
  $null = [Windows.Media.Transcoding.MediaTranscoder, Windows.Media.Transcoding, ContentType = WindowsRuntime]
  $null = [Windows.Media.MediaProperties.MediaEncodingProfile, Windows.Media.MediaProperties, ContentType = WindowsRuntime]
  $null = [Windows.Media.Audio.AudioGraph, Windows.Media.Audio, ContentType = WindowsRuntime]
  $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 }
  $asOp = $asTask | Where-Object { $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
  $asProg = $asTask | Where-Object { $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncActionWithProgress`1' } | Select-Object -First 1
  function Wait-Op($op, [Type]$type) { $t = $asOp.MakeGenericMethod($type).Invoke($null, @($op)); $null = $t.Wait(-1); $t.Result }
  function Wait-Prog($op, [Type]$type) { $t = $asProg.MakeGenericMethod($type).Invoke($null, @($op)); $null = $t.Wait(-1) }
  function New-Profile {
    $p = [Windows.Media.MediaProperties.MediaEncodingProfile]::CreateWav([Windows.Media.MediaProperties.AudioEncodingQuality]::High)
    $p.Audio = [Windows.Media.MediaProperties.AudioEncodingProperties]::CreatePcm(48000, 2, 16)
    $p
  }
  $src = Wait-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync($env:QT_DECODE_IN)) ([Windows.Storage.StorageFile])
  $dir = Wait-Op ([Windows.Storage.StorageFolder]::GetFolderFromPathAsync($env:QT_DECODE_DIR)) ([Windows.Storage.StorageFolder])
  $replace = [Windows.Storage.CreationCollisionOption]::ReplaceExisting
} catch {
  Write-Output ('no decoder: WinRT unavailable: ' + $_.Exception.GetBaseException().Message)
  exit 2
}
try {
  $dst = Wait-Op ($dir.CreateFileAsync($env:QT_DECODE_NAME, $replace)) ([Windows.Storage.StorageFile])
  $tc = New-Object Windows.Media.Transcoding.MediaTranscoder
  $prep = Wait-Op ($tc.PrepareFileTranscodeAsync($src, $dst, (New-Profile))) ([Windows.Media.Transcoding.PrepareTranscodeResult])
  if ($prep.CanTranscode) {
    Wait-Prog ($prep.TranscodeAsync()) ([double])
    Write-Output 'decoded transcoder'
    exit 0
  }
  $why += 'transcoder ' + $prep.FailureReason
} catch {
  $why += 'transcoder ' + $_.Exception.GetBaseException().Message
}
try {
  # Its own file: a transcode that prepared and then failed may still hold the first one open.
  $dst = Wait-Op ($dir.CreateFileAsync($env:QT_DECODE_NAME + '.graph.wav', $replace)) ([Windows.Storage.StorageFile])
  $settings =New-Object Windows.Media.Audio.AudioGraphSettings ([Windows.Media.Render.AudioRenderCategory]::Media)
  $made = Wait-Op ([Windows.Media.Audio.AudioGraph]::CreateAsync($settings)) ([Windows.Media.Audio.CreateAudioGraphResult])
  if ($made.Status -ne 'Success') { throw ('graph ' + $made.Status) }
  $graph = $made.Graph
  $fileIn = Wait-Op ($graph.CreateFileInputNodeAsync($src)) ([Windows.Media.Audio.CreateAudioFileInputNodeResult])
  if ($fileIn.Status -ne 'Success') { throw ('file input ' + $fileIn.Status) }
  $node = $fileIn.FileInputNode
  $ticks = $node.Duration.Ticks
  if ($ticks -le 0) { throw 'file input has no duration' }
  $output = Wait-Op ($graph.CreateFileOutputNodeAsync($dst, (New-Profile))) ([Windows.Media.Audio.CreateAudioFileOutputNodeResult])
  if ($output.Status -ne 'Success') { throw ('file output ' + $output.Status) }
  $node.AddOutgoingConnection($output.FileOutputNode)
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $limit = $node.Duration.TotalMilliseconds * 4 + 10000
  $last = [TimeSpan]::Zero
  $still = 0
  $graph.Start()
  while ($sw.ElapsedMilliseconds -lt $limit) {
    [Threading.Thread]::Sleep(2)
    $pos = $node.Position
    if ($pos -ge $node.Duration) { break }
    if ($pos -eq $last -and $pos -gt [TimeSpan]::Zero) { $still++; if ($still -ge 5) { break } } else { $still = 0 }
    $last = $pos
  }
  $graph.Stop()
  $reached = $node.Position.Ticks
  $done = Wait-Op ($output.FileOutputNode.FinalizeAsync()) ([Windows.Media.Transcoding.TranscodeFailureReason])
  $graph.Dispose()
  if ([string]$done -ne 'None') { throw ('finalize ' + $done) }
  if ($reached -lt $ticks * 0.99) { throw ('the input stopped at ' + $reached + ' of ' + $ticks + ' ticks') }
  Write-Output ('duration-ticks ' + $ticks)
  Write-Output 'decoded audiograph'
  exit 0
} catch {
  $why += 'audiograph ' + $_.Exception.GetBaseException().Message
}
Write-Output ('no decoder: ' + ($why -join '; '))
exit 2
"""


def decode_ref(ogg: Path, out: Path, runner=subprocess.run) -> dict:
    ogg, out = Path(ogg), Path(out)
    if not ogg.is_file():
        raise ToolError("no reference file at %s" % ogg)
    common.refuse_inside_git(out, "decoded reference")
    if sys.platform != "win32":
        raise NoDecoder("Media Foundation needs Windows")
    out.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, QT_DECODE_IN=str(ogg.resolve()), QT_DECODE_DIR=str(out.parent.resolve()),
               QT_DECODE_NAME=out.name)
    script = base64.b64encode(DECODE_PS.encode("utf-16-le")).decode("ascii")
    proc = runner([POWERSHELL, *PS_ARGS, "-EncodedCommand", script], env=env,
                  capture_output=True, text=True, timeout=300)
    said = [s.strip() for s in (proc.stdout or "").splitlines() + (proc.stderr or "").splitlines()
            if s.strip()]
    graph_out = out.with_name(out.name + ".graph.wav")
    if proc.returncode == 0 and "decoded audiograph" in said and graph_out.is_file():
        os.replace(graph_out, out)
    elif graph_out.is_file():
        graph_out.unlink()
    if proc.returncode != 0 or not out.is_file():
        reason = next((s for s in said if s.startswith("no decoder")), said[-1] if said else
                      "Windows PowerShell exited %d" % proc.returncode)
        if out.is_file() and out.stat().st_size == 0:
            out.unlink()
        raise NoDecoder(reason[len("no decoder: "):] if reason.startswith("no decoder: ") else reason)
    route = next((s.split()[1] for s in said if s.startswith("decoded ")), "unknown")
    ticks = next((int(s.split()[1]) for s in said if s.startswith("duration-ticks ")), None)
    try:
        reader = WavReader(out)
    except ToolError as err:
        out.unlink()
        raise NoDecoder("Media Foundation wrote no readable WAV: %s" % err)
    if reader.frames == 0:
        out.unlink()
        raise NoDecoder("Media Foundation wrote an empty WAV")
    if ticks:
        trim_wav(out, int(round(ticks * reader.sample_rate / 1e7)))
        reader = WavReader(out)
    return {"out": str(out), "sample_rate": reader.sample_rate, "channels": reader.channels,
            "bits": reader.bits, "seconds": reader.seconds, "route": route}


def trim_wav(path: Path, frames: int) -> None:
    """Rewrite `path` as a plain WAV of at most its first `frames` frames."""
    reader = WavReader(path)
    if reader.sized and reader.frames <= frames and reader.data_offset == 44:
        return
    data = reader.read_frames(0, frames)
    write_wav(path, data, reader.sample_rate, reader.channels, reader.bits)


# --------------------------------------------------------------------------
# Command line

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="audio", description="The tone probe, the recorder selftest, clips, the reference decode.")
    sub = parser.add_subparsers(dest="command", required=True)
    t = sub.add_parser("tone", help="play a sine and log its pid and start")
    t.add_argument("--hz", type=float, required=True)
    t.add_argument("--seconds", type=float, required=True)
    t.add_argument("--dbfs", type=float, default=TONE_DBFS)
    t.add_argument("--start-after", type=float, default=0.0)
    t.add_argument("--log", type=Path, default=None)
    t.add_argument("--go-file", type=Path, default=None,
                   help="wait for this file to appear before --start-after counts")
    s = sub.add_parser("selftest", help="two tones, the recorder on one; pass when it keeps only that one")
    s.add_argument("--out", type=Path, default=None)
    c = sub.add_parser("clip", help="cut a clip around a UTC instant")
    c.add_argument("--wav", type=Path, required=True)
    c.add_argument("--at", required=True, help="UTC, YYYY-MM-DDTHH:MM:SS.mmmZ")
    c.add_argument("--before", type=float, default=5.0)
    c.add_argument("--after", type=float, default=5.0)
    c.add_argument("--out", type=Path, required=True)
    d = sub.add_parser("decode-ref", help="decode a reference .ogg through Media Foundation")
    d.add_argument("--ogg", type=Path, required=True)
    d.add_argument("--out", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "tone":
            record = tone(args.hz, args.seconds, args.dbfs, args.start_after, args.log, args.go_file)
            print("audio: tone %g Hz from pid %d at %s" % (args.hz, record["pid"], record["start_utc"]))
        elif args.command == "selftest":
            return selftest(args.out)
        elif args.command == "clip":
            info = clip(args.wav, common.parse_utc(args.at), args.before, args.after, args.out)
            print("audio: clip %s from %s to %s (%d frames)"
                  % (args.out, info["first_sample_utc"], info["end_utc"], info["frames"]))
        else:
            info = decode_ref(args.ogg, args.out)
            print("audio: decoded %s through Media Foundation's %s, %d Hz, %d channels, %d bits, %.2f s"
                  % (info["out"], info["route"], info["sample_rate"], info["channels"], info["bits"],
                     info["seconds"]))
    except NoDecoder as err:
        print("audio: no decoder: %s" % err, file=sys.stderr)
        return 2
    except ToolError as err:
        print("audio: error: %s" % err, file=sys.stderr)
        return 1
    except (OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as err:
        print("audio: error: %s" % err, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
