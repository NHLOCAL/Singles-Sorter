#!/usr/bin/env python3
"""Freeze CPython 3.12 / Unicode 15 predicates used by the released policy.

This is a development-time generator, not a runtime dependency or a fixture
expectation generator. Refuse newer Unicode instead of silently refreshing.
"""
import hashlib
import json
from pathlib import Path
import platform
import re
import sys
import unicodedata

ROOT = Path(__file__).resolve().parents[1]


def ranges(bits):
    result = []
    start = None
    for codepoint, member in enumerate(bits):
        if member and start is None:
            start = codepoint
        elif not member and start is not None:
            result.append((start, codepoint - 1))
            start = None
    if start is not None:
        result.append((start, len(bits) - 1))
    return result


def main():
    if platform.python_implementation() != "CPython" or sys.version_info[:2] != (3, 12):
        raise SystemExit("CPython 3.12 is required; tables must not drift")
    if unicodedata.unidata_version != "15.0.0":
        raise SystemExit("Unicode 15.0.0 is required; tables must not drift")
    word_pattern = re.compile(r"\w")
    space_pattern = re.compile(r"\s")
    decimal_pattern = re.compile(r"\d")
    decimals = bytearray()
    decimal_zeroes = []
    words = bytearray()
    spaces = bytearray()
    for codepoint in range(0x110000):
        character = chr(codepoint)
        word = character == "_" or character.isalnum()
        space = character.isspace()
        # CPython source predicates and actual regex behavior must agree for
        # every code point, including unassigned characters and surrogates.
        assert bool(word_pattern.fullmatch(character)) == word
        assert bool(space_pattern.fullmatch(character)) == space
        words.append(word)
        spaces.append(space)
        decimal = unicodedata.decimal(character, None)
        assert bool(decimal_pattern.fullmatch(character)) == (decimal is not None)
        if decimal is not None:
            assert int(character) == decimal
            if decimal == 0:
                decimal_zeroes.append(codepoint)
        decimals.append(255 if decimal is None else decimal)
    output = [
        "//! Generated CPython 3.12 / Unicode 15.0.0 predicates; do not edit by hand.",
        "//! Reproduce with native/unicode/generate_unicode15.py. Provenance is beside it.",
        "", "pub fn is_word(character: char) -> bool {",
        "    contains(WORD_RANGES, character as u32)", "}", "",
        "pub fn is_space(character: char) -> bool {",
        "    contains(SPACE_RANGES, character as u32)", "}", "",
        "fn contains(ranges: &[(u32, u32)], codepoint: u32) -> bool {",
        "    let index = ranges.partition_point(|&(_, end)| end < codepoint);",
        "    ranges", "        .get(index)",
        "        .is_some_and(|&(start, _)| start <= codepoint)", "}", "",
    ]
    for name, bits in [("WORD_RANGES", words), ("SPACE_RANGES", spaces)]:
        output += ["#[rustfmt::skip]", f"const {name}: &[(u32, u32)] = &["]
        output += [f"    (0x{start:X}, 0x{end:X})," for start, end in ranges(bits)]
        output += ["];", ""]
    output += [
        "/// Unicode 15 Nd decimal values, matching CPython 3.12 regex digit rules.",
        "pub fn decimal_digit(character: char) -> Option<u8> {",
        "    let codepoint = character as u32;",
        "    let zero = DECIMAL_ZEROES",
        "        .iter()",
        "        .find(|&&zero| (zero..zero + 10).contains(&codepoint))?;",
        "    Some((codepoint - *zero) as u8)", "}", "",
        "#[rustfmt::skip]", "const DECIMAL_ZEROES: &[u32] = &[",
    ]
    output += [f"    0x{zero:X}," for zero in decimal_zeroes]
    output += ["];", ""]
    generated = "\n".join(output)
    (ROOT / "crates/sorter-core/src/unicode15.rs").write_text(generated, encoding="utf-8")
    provenance = {
        "purpose": "CPython word/space/decimal predicates for v14.0 matching, cleanup and album numbering",
        "generator": "native/unicode/generate_unicode15.py",
        "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "python_implementation": platform.python_implementation(),
        "python_version": platform.python_version(),
        "unicode_version": unicodedata.unidata_version,
        "runtime": sys.version,
        "oracle": "Exhaustive re.fullmatch(r'\\w', chr(cp)) and re.fullmatch(r'\\s', chr(cp)) checked against str.isalnum()/str.isspace() for cp=0..0x10FFFF",
        "word_semantics": "SRE_UNI_IS_WORD = Py_UNICODE_ISALNUM(ch) || ch == '_'",
        "space_semantics": "SRE_UNI_IS_SPACE = Py_UNICODE_ISSPACE(ch)",
        "cpython_source_revision": "v3.12.14",
        "source_references": [
            "https://github.com/python/cpython/blob/v3.12.14/Modules/_sre/sre.c",
            "https://github.com/python/cpython/blob/v3.12.14/Objects/unicodectype.c",
            "https://github.com/python/cpython/blob/v3.12.14/Objects/unicodetype_db.h",
            "https://github.com/python/cpython/blob/v3.12.14/Include/cpython/unicodeobject.h",
        ],
        "domain": "All 1,114,112 code points; surrogate membership is false",
        "word_codepoints": sum(words),
        "space_codepoints": sum(spaces),
        "word_ranges": len(ranges(words)),
        "space_ranges": len(ranges(spaces)),
        "word_membership_sha256": hashlib.sha256(words).hexdigest(),
        "space_membership_sha256": hashlib.sha256(spaces).hexdigest(),
        "decimal_values_sha256": hashlib.sha256(decimals).hexdigest(),
        "decimal_encoding": "One byte per codepoint: 0..9 for Nd decimal value, 255 otherwise; exhaustive regex \\d and int(character) agreement",
        "decimal_zeroes": len(decimal_zeroes),
        "generated_rust_sha256": hashlib.sha256(generated.encode()).hexdigest(),
        "normalization": "None; Rust standard Unicode classes and regex tables are not used",
    }
    (ROOT / "unicode/unicode15-provenance.json").write_text(json.dumps(provenance, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Unicode {unicodedata.unidata_version}: {sum(words)} word/{sum(spaces)} space codepoints; {len(ranges(words))}/{len(ranges(spaces))} ranges")


if __name__ == "__main__":
    main()
