#!/usr/bin/env python3
"""Independent page/packet/decoder probe. This does NOT execute the Rust writer.
Only owned frozen and newly FFmpeg-generated synthetic recordings are used.
"""
import hashlib
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

NATIVE = Path(__file__).resolve().parents[2]
MEDIA = NATIVE / "tests/fixtures/media"
FIELDS = ("artist", "albumartist", "title", "album", "genre")


def crc(data):
    value = 0
    for i, byte in enumerate(data):
        if 22 <= i < 26:
            byte = 0
        value ^= byte << 24
        for _ in range(8):
            value = ((value << 1) ^ (0x04C11DB7 if value & 0x80000000 else 0)) & 0xFFFFFFFF
    return value


def page(flags, granule, serial, sequence, laces, data):
    assert len(laces) <= 255 and sum(laces) == len(data)
    out = bytearray(b"OggS\0" + struct.pack("<BQIIIB", flags, granule, serial, sequence, 0, len(laces)) + bytes(laces) + data)
    struct.pack_into("<I", out, 22, crc(out))
    return bytes(out)


def pages(data):
    at, out, serial, continued = 0, [], None, False
    while at < len(data):
        assert len(data) - at >= 27 and data[at:at + 5] == b"OggS\0"
        flags, granule, stream, seq, checksum, count = struct.unpack_from("<BQIIIB", data, at + 5)
        laces = data[at + 27:at + 27 + count]
        end = at + 27 + count + sum(laces)
        assert count and len(laces) == count and end <= len(data)
        raw = data[at:end]
        assert checksum == crc(raw) and flags & ~7 == 0
        if serial is None:
            serial = stream
        assert stream == serial and seq == len(out)
        assert bool(flags & 1) == continued
        assert bool(flags & 2) == (not out)
        assert bool(flags & 4) == (end == len(data))
        continued = laces[-1] == 255
        out.append((flags, granule, stream, seq, laces, raw[27 + count:], raw))
        at = end
    assert out and not continued
    return out


def packets(data):
    out, pending = [], b""
    for _, _, _, _, laces, body, _ in pages(data):
        at = 0
        for lace in laces:
            pending += body[at:at + lace]
            at += lace
            if lace < 255:
                out.append(pending)
                pending = b""
    assert not pending
    return out


def paginate(content, serial, start=1, limit=255):
    laces, data = [], b""
    for packet in content:
        laces.extend([255] * (len(packet) // 255) + [len(packet) % 255])
        data += packet
    out, at, continued = [], 0, False
    for i in range(0, len(laces), limit):
        part = laces[i:i + limit]
        size = sum(part)
        granule = 0 if min(part) < 255 else 2**64 - 1
        out.append(page(int(continued), granule, serial, start + len(out), part, data[at:at + size]))
        at += size
        continued = part[-1] == 255
    assert not continued
    return out


def vector(data):
    return struct.pack("<I", len(data)) + data


def comment(opus, values, suffix):
    return (b"OpusTags" if opus else b"\x03vorbis") + vector(b"Owned Ogg probe vendor") + struct.pack("<I", len(values)) + b"".join(vector(v.encode()) for v in values) + suffix


def replace_headers(base, comment_packet, split=False, isolate=False):
    parsed, content = pages(base), packets(base)
    opus = content[0].startswith(b"OpusHead")
    required = 2 if opus else 3
    prefix = [comment_packet] + ([] if opus else [content[2]])
    rendered = [parsed[0][-1]]
    if isolate:
        for packet in prefix:
            rendered += paginate([packet], parsed[0][2], start=len(rendered))
    else:
        rendered += paginate(prefix, parsed[0][2], limit=1 if split else 255)
    completed = 0
    for flags, granule, serial, _, laces, body, _ in parsed:
        if completed >= required:
            rendered.append(page(flags, granule, serial, len(rendered), laces, body))
        else:
            completed += sum(x < 255 for x in laces)
    return b"".join(rendered)


def decode(path):
    result = subprocess.run(["ffmpeg", "-v", "error", "-nostdin", "-i", str(path), "-f", "s16le", "-acodec", "pcm_s16le", "-"], check=True, capture_output=True)
    assert result.stdout
    return result.stdout


class FixtureContract(unittest.TestCase):
    def test_frozen_identities_and_crc(self):
        hashes = dict(line.split()[::-1] for line in (MEDIA / "fixtures.sha256").read_text().splitlines())
        for name in ("blank.ogg", "blank.opus"):
            raw = (MEDIA / name).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), hashes["fixtures/" + name])
            self.assertEqual(len(pages(raw)), 3)
            self.assertEqual(len(packets(raw)), 9 if name.endswith("opus") else 10)

    def test_255_terminators_and_continuation(self):
        for opus in (False, True):
            base = (MEDIA / ("blank.opus" if opus else "blank.ogg")).read_bytes()
            suffix = b"\1opaque\0\xff" if opus else b"\x81padding\0"
            empty = comment(opus, ["ARTIST=à", "X="], suffix)
            for length in (255, 65025):
                packet = comment(opus, ["ARTIST=à", "X=" + "x" * (length - len(empty))], suffix)
                out = replace_headers(base, packet)
                self.assertEqual(packets(out)[1], packet)
                self.assertTrue(any(0 in p[4] for p in pages(out)))

    def test_assignment_collapse_and_raw_order(self):
        fields = ["X=first", "ArTiSt=à", "artist=á", "TITLE=à", "X=last", "ALBUM ARTIST=alias"]
        for name in ("artist", "title"):
            values = [v.split("=", 1)[1] for v in fields if v.split("=", 1)[0].lower() == name]
            replacement = ", ".join(values).translate(str.maketrans("àá", "אב"))
            fields = [v for v in fields if v.split("=", 1)[0].lower() != name] + [name + "=" + replacement]
        self.assertEqual(fields, ["X=first", "X=last", "ALBUM ARTIST=alias", "artist=א, ב", "title=א"])

    def test_frozen_repaged_headers_preserve_packet_bytes_and_decoded_pcm(self):
        with tempfile.TemporaryDirectory() as directory:
            for opus, extension in ((False, "ogg"), (True, "opus")):
                base = (MEDIA / ("blank." + extension)).read_bytes()
                suffix = b"\1private\0\xff" if opus else b"\x81padding\0"
                before = replace_headers(base, comment(opus, ["ArTiSt=à", "artist=á", "X=" + "x" * 70000], suffix), split=True)
                after = replace_headers(before, comment(opus, ["X=" + "x" * 70000, "artist=א, ב"], suffix), isolate=True)
                self.assertEqual(packets(before)[0], packets(after)[0])
                self.assertEqual(packets(before)[2:], packets(after)[2:])
                a, b = Path(directory) / ("a." + extension), Path(directory) / ("b." + extension)
                a.write_bytes(before); b.write_bytes(after)
                self.assertEqual(decode(a), decode(b))

    def test_new_ordinary_ffmpeg_files(self):
        with tempfile.TemporaryDirectory() as directory:
            for opus, extension, codec in ((False, "ogg", "libvorbis"), (True, "opus", "libopus")):
                source = Path(directory) / ("source." + extension)
                subprocess.run(["ffmpeg", "-v", "error", "-nostdin", "-f", "lavfi", "-i", "sine=frequency=440:duration=1.2:sample_rate=48000", "-c:a", codec, "-metadata", "artist=à", str(source)], check=True)
                before = source.read_bytes()
                after = replace_headers(before, comment(opus, ["artist=א"], b"" if opus else b"\1"), isolate=True)
                output = Path(directory) / ("output." + extension); output.write_bytes(after)
                self.assertEqual(packets(before)[2:], packets(after)[2:])
                self.assertEqual(decode(source), decode(output))

    def test_independent_parser_rejects_corruption_and_chaining(self):
        base = (MEDIA / "blank.opus").read_bytes()
        for bad in (base + base, base[:-1], base[:-1] + bytes([base[-1] ^ 1])):
            with self.assertRaises(AssertionError):
                pages(bad)


if __name__ == "__main__":
    unittest.main(verbosity=2)
