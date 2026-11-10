//! Owned fixtures and an independent Ogg parser; never uses production helpers.
#![allow(dead_code)]
use std::convert::TryInto;

pub const VORBIS: &[u8] = include_bytes!("../fixtures/media/blank.ogg");
pub const OPUS: &[u8] = include_bytes!("../fixtures/media/blank.opus");
pub const VENDOR: &[u8] = b"Owned Ogg vendor \xc2\xa9";

#[derive(Clone, Debug)]
pub struct Page {
    pub raw: Vec<u8>,
    pub flags: u8,
    pub granule: u64,
    pub serial: u32,
    pub sequence: u32,
    pub laces: Vec<u8>,
    pub data: Vec<u8>,
}
pub fn crc(bytes: &[u8]) -> u32 {
    let table: Vec<u32> = (0..256)
        .map(|n| {
            (0..8).fold(n << 24, |v, _| {
                if v & 0x8000_0000 == 0 {
                    v << 1
                } else {
                    (v << 1) ^ 0x04c1_1db7
                }
            })
        })
        .collect();
    bytes.iter().enumerate().fold(0u32, |v, (i, b)| {
        let b = if (22..26).contains(&i) { 0 } else { *b };
        (v << 8) ^ table[((v >> 24) as u8 ^ b) as usize]
    })
}
pub fn reseal(bytes: &mut [u8]) {
    let sum = crc(bytes);
    bytes[22..26].copy_from_slice(&sum.to_le_bytes());
}
pub fn pages(bytes: &[u8]) -> Vec<Page> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        assert_eq!(&bytes[at..at + 4], b"OggS");
        assert_eq!(bytes[at + 4], 0);
        let body = at + 27 + usize::from(bytes[at + 26]);
        let laces = bytes[at + 27..body].to_vec();
        let end = body + laces.iter().map(|b| usize::from(*b)).sum::<usize>();
        let raw = bytes[at..end].to_vec();
        assert_eq!(
            crc(&raw),
            u32::from_le_bytes(raw[22..26].try_into().unwrap())
        );
        out.push(Page {
            flags: raw[5],
            granule: u64::from_le_bytes(raw[6..14].try_into().unwrap()),
            serial: u32::from_le_bytes(raw[14..18].try_into().unwrap()),
            sequence: u32::from_le_bytes(raw[18..22].try_into().unwrap()),
            raw,
            laces,
            data: bytes[body..end].to_vec(),
        });
        at = end;
    }
    out
}
pub fn packets(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut packet = Vec::new();
    let mut continued = false;
    let parsed = pages(bytes);
    let serial = parsed[0].serial;
    for (i, p) in parsed.iter().enumerate() {
        assert_eq!(p.serial, serial);
        assert_eq!(p.sequence, i as u32);
        assert_eq!(p.flags & 1 != 0, continued);
        assert_eq!(p.flags & 2 != 0, i == 0);
        assert_eq!(p.flags & 4 != 0, i + 1 == parsed.len());
        let mut at = 0;
        for lace in &p.laces {
            packet.extend(&p.data[at..at + usize::from(*lace)]);
            at += usize::from(*lace);
            continued = *lace == 255;
            if !continued {
                out.push(std::mem::take(&mut packet));
            }
        }
    }
    assert!(!continued);
    out
}
pub fn page(
    flags: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    laces: &[u8],
    data: &[u8],
) -> Vec<u8> {
    assert!(laces.len() <= 255);
    assert_eq!(
        laces.iter().map(|b| usize::from(*b)).sum::<usize>(),
        data.len()
    );
    let mut out = b"OggS\0".to_vec();
    out.push(flags);
    out.extend(granule.to_le_bytes());
    out.extend(serial.to_le_bytes());
    out.extend(sequence.to_le_bytes());
    out.extend([0; 4]);
    out.push(laces.len() as u8);
    out.extend(laces);
    out.extend(data);
    reseal(&mut out);
    out
}
pub fn vector(out: &mut Vec<u8>, data: &[u8]) {
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
}
pub fn comment_packet(opus: bool, comments: &[&str], suffix: &[u8]) -> Vec<u8> {
    let mut out = if opus {
        b"OpusTags".to_vec()
    } else {
        b"\x03vorbis".to_vec()
    };
    vector(&mut out, VENDOR);
    out.extend((comments.len() as u32).to_le_bytes());
    for text in comments {
        vector(&mut out, text.as_bytes());
    }
    out.extend(suffix);
    out
}
pub fn paginate(packets: &[Vec<u8>], serial: u32, first: u32, max_segments: usize) -> Vec<Vec<u8>> {
    assert!((1..=255).contains(&max_segments));
    let mut laces = Vec::new();
    let mut data = Vec::new();
    for packet in packets {
        laces.extend(std::iter::repeat_n(255, packet.len() / 255));
        laces.push((packet.len() % 255) as u8);
        data.extend(packet);
    }
    let mut out = Vec::new();
    let mut at = 0;
    let mut continued = false;
    for (i, chunk) in laces.chunks(max_segments).enumerate() {
        let size = chunk.iter().map(|b| usize::from(*b)).sum::<usize>();
        let granule = if chunk.iter().any(|b| *b < 255) {
            0
        } else {
            u64::MAX
        };
        out.push(page(
            u8::from(continued),
            granule,
            serial,
            first + i as u32,
            chunk,
            &data[at..at + size],
        ));
        at += size;
        continued = chunk.last() == Some(&255);
    }
    assert!(!continued);
    out
}
pub fn build(opus: bool, comments: &[&str], suffix: &[u8], max_segments: usize) -> Vec<u8> {
    let base = if opus { OPUS } else { VORBIS };
    let source = pages(base);
    let content = packets(base);
    let mut headers = vec![comment_packet(opus, comments, suffix)];
    if !opus {
        headers.push(content[2].clone());
    }
    let mut out = source[0].raw.clone();
    let rendered = paginate(&headers, source[0].serial, 1, max_segments);
    let mut next = rendered.len() as u32 + 1;
    for p in rendered {
        out.extend(p);
    }
    // Frozen tiny ordinary FFmpeg recording has one audio page, seven packets.
    for p in &source[2..] {
        out.extend(page(p.flags, p.granule, p.serial, next, &p.laces, &p.data));
        next += 1;
    }
    out
}
pub fn comments(bytes: &[u8]) -> (Vec<u8>, Vec<String>, Vec<u8>) {
    let all = packets(bytes);
    let packet = &all[1];
    let mut at = if packet.starts_with(b"OpusTags") {
        8
    } else {
        7
    };
    fn word(bytes: &[u8], at: &mut usize) -> usize {
        let n = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
        *at += 4;
        n
    }
    let n = word(packet, &mut at);
    let vendor = packet[at..at + n].to_vec();
    at += n;
    let count = word(packet, &mut at);
    let mut comments = Vec::new();
    for _ in 0..count {
        let n = word(packet, &mut at);
        comments.push(String::from_utf8(packet[at..at + n].to_vec()).unwrap());
        at += n;
    }
    (vendor, comments, packet[at..].to_vec())
}
pub fn audio_pages(bytes: &[u8]) -> Vec<Page> {
    let mut done = 0;
    let mut out = Vec::new();
    let parsed = pages(bytes);
    let required = if parsed[0].data.starts_with(b"OpusHead") {
        2
    } else {
        3
    };
    for p in parsed {
        if done >= required {
            out.push(p);
        } else {
            done += p.laces.iter().filter(|b| **b < 255).count();
        }
    }
    out
}
pub fn assert_preserved(before: &[u8], after: &[u8]) {
    let old = packets(before);
    let new = packets(after);
    assert_eq!(old.len(), new.len());
    assert_eq!(old[0], new[0]);
    assert_eq!(old[2..], new[2..]);
    let old = audio_pages(before);
    let new = audio_pages(after);
    assert_eq!(old.len(), new.len());
    for (a, b) in old.iter().zip(new) {
        assert_eq!(
            (a.flags, a.granule, a.serial, &a.laces, &a.data),
            (b.flags, b.granule, b.serial, &b.laces, &b.data)
        );
        let mut masked_a = a.raw.clone();
        let mut masked_b = b.raw;
        masked_a[18..26].fill(0);
        masked_b[18..26].fill(0);
        assert_eq!(masked_a, masked_b);
    }
}

// Owned generated one-pixel black PNG in a FLAC picture comment.
pub const PICTURE: &str = "METADATA_BLOCK_PICTURE=AAAAAwAAAAlpbWFnZS9wbmcAAAAAAAAAAQAAAAEAAAAYAAAAAAAAAEWJUE5HDQoaCgAAAA1JSERSAAAAAQAAAAEIAgAAAJB3U94AAAAMSURBVHicY2BgYAAAAAQAAfYXOFUAAAAASUVORK5CYII=";
