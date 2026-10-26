#!/usr/bin/env python3
"""Independent FLAC fixture/spec sanity check, NOT execution of the Rust writer.

Uses Python stdlib, the owned frozen two-frame recording, and installed FFmpeg.
It never touches any live media library or installs dependencies.
"""
import hashlib
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib

NATIVE = Path(__file__).resolve().parents[2]
BASE = (NATIVE / "tests/fixtures/media/blank.flac").read_bytes()
AUDIO = BASE[46:]
VENDOR = "Owned FLAC fixture vendor ©".encode()
FIELDS = ("artist", "albumartist", "title", "album", "genre")


def block(kind, body, last=False):
    return bytes([kind | (128 if last else 0)]) + len(body).to_bytes(3, "big") + body


def vector(value):
    return struct.pack("<I", len(value)) + value


def comments(values):
    return vector(VENDOR) + struct.pack("<I", len(values)) + b"".join(vector(v.encode()) for v in values)


def cue():
    out = bytes(395) + b"\x02"
    for sample, number, has_index in [(0, 1, True), (5292, 255, False)]:
        out += struct.pack(">QB", sample, number) + bytes(26) + bytes([has_index])
        if has_index:
            out += bytes(8) + b"\x01" + bytes(3)
    return out


def picture():
    def chunk(key, data):
        return vector_be(data)[:4] + key + data + struct.pack(">I", zlib.crc32(key + data))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(4))) + chunk(b"IEND", b"")
    return struct.pack(">I", 3) + vector_be(b"image/png") + bytes(4) + struct.pack(">IIII", 1, 1, 24, 0) + vector_be(png)


def vector_be(value):
    return struct.pack(">I", len(value)) + value


SEEK = b"".join(struct.pack(">QQH", *point) for point in [(0, 0, 4608), (4608, 11, 684)])
EXTRAS = [(3, SEEK), (5, cue()), (6, picture())]


def fixture(values):
    blocks = [(0, BASE[8:42]), (4, comments(values)), *EXTRAS]
    return b"fLaC" + b"".join(block(kind, body, i + 1 == len(blocks)) for i, (kind, body) in enumerate(blocks)) + AUDIO


def split(data):
    assert data[:4] == b"fLaC"
    at, blocks = 4, []
    while True:
        size = int.from_bytes(data[at + 1:at + 4], "big")
        end = at + 4 + size
        blocks.append(data[at:end])
        at = end
        if blocks[-1][0] & 128:
            return blocks, data[at:]


def assign(values, repairs):
    # The released Mutagen operation: delete matching pairs, append assignment.
    result = values[:]
    for name, replacement in repairs:
        result = [entry for entry in result if entry.split("=", 1)[0].lower() != name]
        result.append(f"{name}={replacement}")
    return result


class FixtureContract(unittest.TestCase):
    def test_frozen_audio_and_seek_points(self):
        blocks, tail = split(BASE)
        self.assertEqual(tail, AUDIO)
        self.assertEqual(hashlib.sha256(BASE).hexdigest(), "626b7009add72ab6ab011656b07b8f7cf32a5e0ee24658ccd7caa8875b6b7332")
        self.assertEqual(len(tail), 24)
        self.assertEqual(hashlib.sha256(tail).hexdigest(), "97d269df10f6f06962ceb18c4cd0e676d261cebe809f34f8c01f33b2d1622518")
        self.assertEqual(len(blocks[0]), 38)
        for at in range(0, len(SEEK), 18):
            _, offset, _ = struct.unpack(">QQH", SEEK[at:at + 18])
            self.assertEqual(tail[offset:offset + 2], b"\xff\xf8")

    def test_metadata_shift_keeps_standard_blocks_and_audio(self):
        before = fixture(["ArTiSt=à", "X-Custom=before", "artist=á", "TITLE=kept"])
        after = fixture(assign(["ArTiSt=à", "X-Custom=before", "artist=á", "TITLE=kept"], [("artist", "א, ב")]))
        old_blocks, old_audio = split(before)
        new_blocks, new_audio = split(after)
        self.assertLess(len(after), len(before))
        self.assertEqual(old_audio, new_audio)
        self.assertEqual([b for b in old_blocks if b[0] & 127 != 4], [b for b in new_blocks if b[0] & 127 != 4])
        self.assertEqual(assign(["ARTIST=", "artist=à", "Artist="], [("artist", ", א, ")]), ["artist=, א, "])

    def test_released_mapping_does_not_need_growth(self):
        old = "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ"
        new = "אבגדהוזחטיכלמנסעפצקרשתךםןףץ"
        self.assertEqual(len(old), len(new))
        for a, b in zip(old, new):
            self.assertEqual(len(a.encode()), len(b.encode()))

    @unittest.skipUnless(shutil.which("ffmpeg"), "installed FFmpeg required for independent decode check")
    def test_real_decoder_accepts_shifted_audio(self):
        before = fixture(["ARTIST=à", "artist=á", "TITLE=kept"])
        after = fixture(["TITLE=kept", "artist=א, ב"])
        with tempfile.TemporaryDirectory(prefix="owned-flac-contract-") as owned:
            pcm = []
            for name, data in [("before", before), ("after", after)]:
                path = Path(owned) / f"{name}.flac"
                path.write_bytes(data)
                command = ["ffmpeg", "-v", "error", "-i", str(path), "-map", "0:a:0", "-f", "s16le", "-"]
                result = subprocess.run(command, check=True, capture_output=True)
                self.assertEqual(result.stderr, b"")
                pcm.append(result.stdout)
            self.assertEqual(pcm[0], pcm[1])
            self.assertEqual(len(pcm[0]), 5292 * 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
