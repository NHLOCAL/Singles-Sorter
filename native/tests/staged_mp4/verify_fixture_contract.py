"""Independent owned MP4 exact-extent fixture and FFmpeg checks; no user paths."""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

if not __debug__:
    raise RuntimeError("MP4 preservation checks require assertions; do not run with -O")

ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "media"
KEYS = [b"\xa9ART", b"aART", b"\xa9nam", b"\xa9alb", b"\xa9gen"]
ORIGINAL = ["à", "á", "â", "ã", "ä"]
REPAIRED = ["א", "ב", "ג", "ד", "ה"]

def atom(kind, body):
    return struct.pack(">I4s", len(body) + 8, kind) + body

def children(data):
    offset = 0
    result = []
    while offset < len(data):
        size, kind = struct.unpack_from(">I4s", data, offset)
        assert 8 <= size <= len(data) - offset
        result.append((kind, data[offset:offset + size]))
        offset += size
    return result

def text(key, values):
    return atom(key, b"".join(atom(b"data", b"\0\0\0\1\0\0\0\0" + value.encode()) for value in values))

def update(data, payload, exact=False):
    result = bytearray()
    for kind, box in children(data):
        if kind == b"ilst":
            replacement = atom(kind, payload)
            if exact:
                slack = len(box) - len(replacement)
                assert slack == 0 or slack >= 8
                if slack:
                    replacement += atom(b"free", bytes(slack - 8))
            result.extend(replacement)
        elif kind in (b"moov", b"udta"):
            result.extend(atom(kind, update(box[8:], payload, exact)))
        elif kind == b"meta":
            result.extend(atom(kind, box[8:12] + update(box[12:], payload, exact)))
        else:
            result.extend(box)
    return bytes(result)

def list_items(data):
    for key in (b"moov", b"udta", b"meta", b"ilst"):
        data = next(box for kind, box in children(data) if kind == key)
        data = data[12:] if key == b"meta" else data[8:]
    return children(data)

def fixture(alac=False, multiple=False):
    base = (ROOT / ("blank.alac" if alac else "blank.m4a")).read_bytes()
    assert children(base)[-1][0] == b"moov"
    fields = b"".join(text(key, [value]) for key, value in zip(KEYS, ORIGINAL))
    if multiple:
        fields = text(KEYS[0], ["à", "á", "â"])
    opaque = atom(b"zzzz", b"opaque unrecognized atom \0\xff")
    picture = next(box for key, box in list_items((ROOT / "tagged.m4a").read_bytes()) if key == b"covr")
    return update(base, fields + opaque + picture)

def repair(data, multiple=False):
    unknown = b"".join(box for key, box in list_items(data) if key not in KEYS)
    values = ["א, ב, ג"] if multiple else REPAIRED
    fields = b"".join(text(key, [value]) for key, value in zip(KEYS, values))
    return update(data, fields + unknown, exact=True)

def fast_start(data):
    top = children(data)
    movie = top[-1][1]
    def offsets(boxes):
        result = bytearray()
        for kind, box in children(boxes):
            if kind in (b"moov", b"trak", b"mdia", b"minf", b"stbl"):
                box = atom(kind, offsets(box[8:]))
            elif kind == b"stco":
                box = bytearray(box)
                for pos in range(16, len(box), 4):
                    old, = struct.unpack_from(">I", box, pos)
                    struct.pack_into(">I", box, pos, old + len(movie))
            result.extend(box)
        return bytes(result)
    return top[0][1] + offsets(movie) + b"".join(box for _, box in top[1:-1])

def digest(data):
    return hashlib.sha256(data).hexdigest()

def evidence(before, after):
    assert len(before) == len(after)
    old = children(before)
    new = children(after)
    for (kind, left), (other, right) in zip(old, new):
        assert kind == other and len(left) == len(right)
        if kind != b"moov":
            assert left == right
    for key in (b"zzzz", b"covr"):
        assert [box for kind, box in list_items(before) if kind == key] == [box for kind, box in list_items(after) if kind == key]
    with tempfile.TemporaryDirectory(prefix="owned-mp4-") as directory:
        a, b = Path(directory) / "before.m4a", Path(directory) / "after.m4a"
        a.write_bytes(before)
        b.write_bytes(after)
        def pcm(path):
            return subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(path), "-map", "0:a:0", "-f", "s16le", "-"])
        original, repaired = pcm(a), pcm(b)
        assert original == repaired and original
        tags = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format_tags", "-of", "json", str(b)]))["format"]["tags"]
    return {"before_sha256": digest(before), "after_sha256": digest(after), "pcm_sha256": digest(original), "length": len(before), "tags": tags}

class Contract(unittest.TestCase):
    def test_owned_aac_and_alac_all_fields(self):
        for alac in (False, True):
            before = fixture(alac)
            report = evidence(before, repair(before))
            for name, value in zip(("artist", "album_artist", "title", "album", "genre"), REPAIRED):
                self.assertEqual(report["tags"][name], value)
    def test_fast_start_keeps_track_tables_and_order(self):
        before = fast_start(fixture())
        after = repair(before)
        evidence(before, after)
        self.assertEqual(children(after)[1][0], b"moov")
        tracks = lambda data: [box for kind, box in children(children(data)[1][1][8:]) if kind == b"trak"]
        self.assertEqual(tracks(before), tracks(after))
    def test_duplicate_values_use_valid_padding(self):
        before = fixture(multiple=True)
        report = evidence(before, repair(before, multiple=True))
        self.assertEqual(report["tags"]["artist"], "א, ב, ג")

if __name__ == "__main__":
    unittest.main()
