pub fn fixture(version: u8, flags: u8) -> Vec<u8> {
    let original = include_bytes!("../fixtures/media/tagged.mp3");
    let size = original[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    let mut body = Vec::new();
    for (key, text) in [
        (b"TP1", "שלום עולם"),
        (b"TP2", "זמר אלבום"),
        (b"TT2", "שיר לבדיקה"),
        (b"TAL", "אלבום לדוגמה"),
        (b"TCO", "Test"),
        (b"TRK", "2/9"),
    ] {
        let mut value = vec![1, 255, 254];
        for unit in text.encode_utf16() {
            value.extend(unit.to_le_bytes());
        }
        value.extend([0, 0]);
        if version == 2 {
            body.extend(key);
            body.extend(&(value.len() as u32).to_be_bytes()[1..]);
        } else {
            let key = match key {
                b"TP1" => b"TPE1",
                b"TP2" => b"TPE2",
                b"TT2" => b"TIT2",
                b"TAL" => b"TALB",
                b"TCO" => b"TCON",
                b"TRK" => b"TRCK",
                _ => unreachable!(),
            };
            body.extend(key);
            body.extend((value.len() as u32).to_be_bytes());
            body.extend([0, 0]);
        }
        body.extend(value);
    }
    if flags & 0x40 != 0 {
        let mut extended = vec![0, 0, 0, 6, 0, 0, 0, 0, 0, 0];
        extended.extend(body);
        body = extended;
    }
    if flags & 0x80 != 0 {
        let mut escaped = Vec::new();
        for (i, b) in body.iter().enumerate() {
            escaped.push(*b);
            if *b == 255 && body.get(i + 1).is_none_or(|n| *n == 0 || *n >= 0xe0) {
                escaped.push(0);
            }
        }
        body = escaped;
    }
    let mut out = vec![b'I', b'D', b'3', version, 0, flags];
    for shift in [21, 14, 7, 0] {
        out.push(((body.len() >> shift) & 127) as u8);
    }
    out.extend(body);
    out.extend(&original[10 + size..]);
    out
}

pub fn mixed_library(source: &std::path::Path) {
    for dir in ["disc-a", "disc-b"] {
        std::fs::create_dir(source.join(dir)).unwrap();
        for index in 0..40 {
            let (version, flags) = match index % 3 {
                0 => (2, 0),
                1 => (3, 0x80),
                _ => (3, 0x40),
            };
            std::fs::write(
                source.join(dir).join(format!("{index:03}-track.mp3")),
                fixture(version, flags),
            )
            .unwrap();
        }
    }
    for index in 0..23 {
        std::fs::write(
            source.join(if index == 1 {
                "metadata-only-v1.mp3".into()
            } else {
                format!("Fallback Artist {index:03}.mp3")
            }),
            if index == 0 {
                vec![]
            } else if index == 1 {
                let mut bytes = id3v1(17, 1, 0);
                let at = bytes.len() - 128 + 33;
                bytes[at..at + 30].fill(0);
                // Latin-1 mojibake for "שלום עולם", matching the original map.
                bytes[at..at + 9]
                    .copy_from_slice(&[0xf9, 0xec, 0xe5, 0xed, b' ', 0xf2, 0xe5, 0xec, 0xed]);
                bytes
            } else {
                fixture(3, 0)
            },
        )
        .unwrap();
    }
}

pub fn id3v1(genre: u8, track: u8, comment_marker: u8) -> Vec<u8> {
    let original = include_bytes!("../fixtures/media/blank.mp3");
    let mut tag = vec![0; 128];
    tag[..3].copy_from_slice(b"TAG");
    for (offset, text) in [
        (3, b"  Synthetic title\t".as_slice()),
        (33, b" Artist ".as_slice()),
        (63, b"Album".as_slice()),
    ] {
        tag[offset..offset + text.len()].copy_from_slice(text);
    }
    tag[125] = comment_marker;
    tag[126] = track;
    tag[127] = genre;
    let mut bytes = original.to_vec();
    bytes.extend(tag);
    bytes
}
