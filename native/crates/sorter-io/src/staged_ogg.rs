//! Bounded Ogg Vorbis/Opus staged repair. No path access or publication authority.
//! Header packets are repaginated; audio page payloads, lacing, flags, serial and
//! granule positions are unchanged. Only audio sequence/checksum words may change.
use crate::{
    staged_tags::{StagedTagEvidence, TagWriteError},
    staged_xiph::{CommentKind, rewrite_comments, validate_repairs},
};
use sha2::{Digest, Sha256};
use sorter_core::{fix_plan::FieldRepair, metadata::LegacyField};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_HEADERS: usize = 64 * 1024 * 1024;
const MAX_HEADER_PAGES: u32 = 4096;
const MAX_AUDIO_PACKET: u64 = 64 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;
type Result<T> = std::result::Result<T, TagWriteError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Codec { Vorbis, Opus }

#[derive(Clone, Debug)]
pub struct OggStagedPlan {
    original_header: Vec<u8>,
    repaired_header: Vec<u8>,
    source_length: u64,
    source_sha256: String,
    output_length: u64,
    header_pages: u32,
    sequence_delta: u32,
    serial: u32,
    codec: Codec,
    changed_fields: Vec<LegacyField>,
}

fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) { Err(TagWriteError::Cancelled) } else { Ok(()) }
}
fn read_checked(input: &mut impl Read, bytes: &mut [u8], flag: &AtomicBool) -> Result<()> {
    for chunk in bytes.chunks_mut(CHUNK) {
        cancelled(flag)?;
        input.read_exact(chunk).map_err(|_| TagWriteError::Io)?;
    }
    cancelled(flag)
}
fn eof(input: &mut impl Read, flag: &AtomicBool) -> Result<()> {
    cancelled(flag)?;
    if input.read(&mut [0; 1]).map_err(|_| TagWriteError::Io)? != 0 {
        return Err(TagWriteError::Stale);
    }
    cancelled(flag)
}
fn word(bytes: &[u8]) -> u32 { u32::from_le_bytes(bytes.try_into().expect("fixed Ogg word")) }

// RFC 3533 non-reflected CRC, initial value zero, polynomial 0x04c11db7.
// The stored checksum field is treated as zero without changing input bytes.
fn checksum(bytes: &[u8], flag: &AtomicBool) -> Result<u32> {
    let mut crc = 0u32;
    for (i, b) in bytes.iter().enumerate() {
        if i.is_multiple_of(2048) { cancelled(flag)?; }
        let byte = if (22..26).contains(&i) { 0 } else { *b };
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = (crc << 1) ^ if crc & 0x8000_0000 != 0 { 0x04c1_1db7 } else { 0 };
        }
    }
    Ok(crc)
}
#[derive(Debug)]
struct Page { bytes: Vec<u8>, body: usize }
impl Page {
    fn read(input: &mut impl Read, remaining: u64, flag: &AtomicBool) -> Result<Self> {
        if remaining < 27 { return Err(TagWriteError::Unsupported("truncated Ogg page")); }
        let mut header = [0; 27];
        read_checked(input, &mut header, flag)?;
        if &header[..4] != b"OggS" || header[4] != 0 || header[5] & !7 != 0 {
            return Err(TagWriteError::Unsupported("invalid Ogg page header"));
        }
        let body = 27 + usize::from(header[26]);
        if header[26] == 0 || body as u64 > remaining {
            return Err(TagWriteError::Unsupported("empty or truncated Ogg segment table"));
        }
        let mut bytes = header.to_vec();
        bytes.resize(body, 0);
        read_checked(input, &mut bytes[27..], flag)?;
        let size = body + bytes[27..].iter().map(|b| usize::from(*b)).sum::<usize>();
        if size as u64 > remaining { return Err(TagWriteError::Unsupported("truncated Ogg page body")); }
        bytes.resize(size, 0);
        read_checked(input, &mut bytes[body..], flag)?;
        if checksum(&bytes, flag)? != word(&bytes[22..26]) {
            return Err(TagWriteError::Unsupported("Ogg page checksum mismatch"));
        }
        Ok(Self { bytes, body })
    }
    fn flags(&self) -> u8 { self.bytes[5] }
    fn serial(&self) -> u32 { word(&self.bytes[14..18]) }
    fn sequence(&self) -> u32 { word(&self.bytes[18..22]) }
    fn granule(&self) -> u64 { u64::from_le_bytes(self.bytes[6..14].try_into().expect("granule word")) }
    fn lacing(&self) -> &[u8] { &self.bytes[27..self.body] }
    fn payload(&self) -> &[u8] { &self.bytes[self.body..] }
    fn completed(&self) -> usize { self.lacing().iter().filter(|b| **b < 255).count() }
    fn continues(&self) -> bool { self.lacing().last() == Some(&255) }
    fn shift_sequence(&mut self, delta: u32, flag: &AtomicBool) -> Result<()> {
        if delta != 0 {
            let sequence = self.sequence().wrapping_add(delta);
            self.bytes[18..22].copy_from_slice(&sequence.to_le_bytes());
            let crc = checksum(&self.bytes, flag)?;
            self.bytes[22..26].copy_from_slice(&crc.to_le_bytes());
        }
        Ok(())
    }
}

struct StreamState { serial: u32, next: u32, continued: bool, ended: bool }
impl StreamState {
    fn accept(&mut self, page: &Page) -> Result<()> {
        if self.ended || page.flags() & 2 != 0 || page.serial() != self.serial {
            return Err(TagWriteError::Unsupported("chained, multiplexed or repeated BOS Ogg stream"));
        }
        if page.sequence() != self.next || (page.flags() & 1 != 0) != self.continued {
            return Err(TagWriteError::Unsupported("Ogg sequence or continuation mismatch"));
        }
        self.next = self.next.wrapping_add(1);
        self.continued = page.continues();
        self.ended = page.flags() & 4 != 0;
        if self.ended && self.continued {
            return Err(TagWriteError::Unsupported("unfinished packet at Ogg EOS"));
        }
        Ok(())
    }
}

fn identify(packet: &[u8]) -> Result<Codec> {
    if packet.starts_with(b"\x01vorbis") {
        if packet.len() != 30 || word(&packet[7..11]) != 0 || packet[11] == 0
            || word(&packet[12..16]) == 0 || !(6..=13).contains(&(packet[28] & 15))
            || !(6..=13).contains(&(packet[28] >> 4)) || (packet[28] & 15) > (packet[28] >> 4)
            || packet[29] & 1 == 0 {
            return Err(TagWriteError::Unsupported("invalid Vorbis identification header"));
        }
        Ok(Codec::Vorbis)
    } else if packet.starts_with(b"OpusHead") {
        if packet.len() < 19 || packet[8] != 1 || packet[9] == 0 {
            return Err(TagWriteError::Unsupported("unsupported Opus identification version"));
        }
        let channels = usize::from(packet[9]);
        match packet[18] {
            0 if channels <= 2 && packet.len() == 19 => {}
            1 if channels <= 8 && packet.len() == 21 + channels => {
                let streams = u16::from(packet[19]);
                let coupled = u16::from(packet[20]);
                if streams == 0 || coupled > streams || streams + coupled > 255
                    || packet[21..].iter().any(|b| *b != 255 && u16::from(*b) >= streams + coupled) {
                    return Err(TagWriteError::Unsupported("invalid Opus channel mapping"));
                }
            }
            _ => return Err(TagWriteError::Unsupported("unsupported Opus mapping or header size")),
        }
        Ok(Codec::Opus)
    } else {
        Err(TagWriteError::Unsupported("only Ogg Vorbis and Opus are supported"))
    }
}

struct Headers { original: Vec<u8>, first_page: Vec<u8>, packets: Vec<Vec<u8>>, state: StreamState, codec: Codec, pages: u32 }
fn headers(input: &mut impl Read, source_length: u64, flag: &AtomicBool) -> Result<Headers> {
    let first = Page::read(input, source_length, flag)?;
    if first.flags() != 2 || first.sequence() != 0 || first.granule() != 0
        || first.completed() != 1 || first.continues() {
        return Err(TagWriteError::Unsupported("identification must be alone on first Ogg BOS page"));
    }
    let codec = identify(first.payload())?;
    let mut state = StreamState { serial: first.serial(), next: 1, continued: false, ended: false };
    let first_page = first.bytes;
    let mut original = first_page.clone();
    let mut packets = Vec::new();
    let mut packet = Vec::new();
    let required = if codec == Codec::Vorbis { 2 } else { 1 };
    let mut pages = 1;
    while packets.len() < required {
        cancelled(flag)?;
        if pages >= MAX_HEADER_PAGES { return Err(TagWriteError::Unsupported("Ogg header page budget")); }
        let page = Page::read(input, source_length - original.len() as u64, flag)?;
        state.accept(&page)?;
        if state.ended { return Err(TagWriteError::Unsupported("EOS before Ogg audio")); }
        let granule = page.granule();
        if (page.completed() > 0 && granule != 0)
            || (page.completed() == 0 && granule != u64::MAX && !(codec == Codec::Vorbis && granule == 0)) {
            return Err(TagWriteError::Unsupported("invalid Ogg header granule"));
        }
        if original.len().checked_add(page.bytes.len()).is_none_or(|n| n > MAX_HEADERS) {
            return Err(TagWriteError::Unsupported("Ogg header byte budget"));
        }
        let mut at = 0;
        for (i, segment) in page.lacing().iter().enumerate() {
            cancelled(flag)?;
            let end = at + usize::from(*segment);
            packet.extend_from_slice(&page.payload()[at..end]);
            at = end;
            if *segment < 255 {
                if packet.is_empty() { return Err(TagWriteError::Unsupported("empty Ogg header packet")); }
                packets.push(std::mem::take(&mut packet));
                if packets.len() == required && i + 1 != page.lacing().len() {
                    return Err(TagWriteError::Unsupported("audio shares Ogg header page"));
                }
            }
        }
        original.extend_from_slice(&page.bytes);
        pages += 1;
    }
    let signature: &[u8] = if codec == Codec::Vorbis { b"\x03vorbis" } else { b"OpusTags" };
    if !packets[0].starts_with(signature)
        || (codec == Codec::Vorbis && (!packets[1].starts_with(b"\x05vorbis") || packets[1].len() <= 7)) {
        return Err(TagWriteError::Unsupported("invalid Ogg codec header order"));
    }
    Ok(Headers { original, first_page, packets, state, codec, pages })
}

// Write each remaining codec header packet on its own bounded page sequence.
// Exact multiples of 255 include their mandatory terminating zero lace, even
// when this requires another continued page. No audio packet is repaginated.
fn paginate(packet: &[u8], serial: u32, sequence: &mut u32, output: &mut Vec<u8>, flag: &AtomicBool) -> Result<()> {
    let segments = packet.len() / 255 + 1;
    let mut offset = 0;
    for base in (0..segments).step_by(255) {
        cancelled(flag)?;
        let end = (base + 255).min(segments);
        let complete = end == segments;
        let mut page = b"OggS\0".to_vec();
        page.push(u8::from(base != 0));
        page.extend(if complete { 0u64 } else { u64::MAX }.to_le_bytes());
        page.extend(serial.to_le_bytes());
        page.extend(sequence.to_le_bytes());
        page.extend([0; 4]);
        page.push((end - base) as u8);
        let mut size = 0;
        for i in base..end {
            let lace = if i + 1 == segments { (packet.len() % 255) as u8 } else { 255 };
            page.push(lace);
            size += usize::from(lace);
        }
        page.extend_from_slice(&packet[offset..offset + size]);
        offset += size;
        let crc = checksum(&page, flag)?;
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        if output.len().checked_add(page.len()).is_none_or(|n| n > MAX_HEADERS) {
            return Err(TagWriteError::Unsupported("rewritten Ogg header budget"));
        }
        output.extend(page);
        *sequence = sequence.checked_add(1).ok_or(TagWriteError::Unsupported("Ogg header sequence overflow"))?;
    }
    Ok(())
}

#[derive(Default)]
struct AudioState { packet_length: u64, packets: u64, last_granule: u64 }
impl AudioState {
    fn accept(&mut self, page: &Page, codec: Codec, flag: &AtomicBool) -> Result<()> {
        let completed = page.completed();
        let granule = page.granule();
        if (completed == 0 && granule != u64::MAX)
            || (completed != 0 && (granule > i64::MAX as u64 || granule < self.last_granule)) {
            return Err(TagWriteError::Unsupported("invalid Ogg audio granule"));
        }
        if completed != 0 { self.last_granule = granule; }
        let mut at = 0;
        for segment in page.lacing() {
            cancelled(flag)?;
            if self.packet_length == 0 && *segment > 0 && codec == Codec::Vorbis && page.payload()[at] & 1 != 0 {
                return Err(TagWriteError::Unsupported("unexpected Vorbis header in audio"));
            }
            self.packet_length += u64::from(*segment);
            if self.packet_length > MAX_AUDIO_PACKET { return Err(TagWriteError::Unsupported("Ogg audio packet budget")); }
            at += usize::from(*segment);
            if *segment < 255 {
                if self.packet_length == 0 { return Err(TagWriteError::Unsupported("zero-length Ogg audio packet")); }
                self.packets += 1;
                self.packet_length = 0;
            }
        }
        Ok(())
    }
    fn finish(&self, state: &StreamState) -> Result<()> {
        if !state.ended || state.continued || self.packet_length != 0 || self.packets == 0 {
            return Err(TagWriteError::Unsupported("missing Ogg EOS or complete audio packets"));
        }
        Ok(())
    }
}

impl OggStagedPlan {
    /// Full sequential preflight before output exists. Buffers only bounded codec
    /// headers and one page; rejects other logical streams anywhere in the file.
    pub fn inspect(input: &mut (impl Read + Seek), source_length: u64, repairs: &[FieldRepair], cancel: &AtomicBool) -> Result<Self> {
        cancelled(cancel)?;
        validate_repairs(repairs, MAX_HEADERS, cancel)?;
        if input.seek(SeekFrom::End(0)).map_err(|_| TagWriteError::Io)? != source_length { return Err(TagWriteError::Stale); }
        input.seek(SeekFrom::Start(0)).map_err(|_| TagWriteError::Io)?;
        let mut h = headers(input, source_length, cancel)?;
        let (prefix, kind) = match h.codec { Codec::Vorbis => (7, CommentKind::Vorbis), Codec::Opus => (8, CommentKind::Opus) };
        let comment = rewrite_comments(&h.packets[0][prefix..], repairs, kind, MAX_HEADERS - prefix, cancel)?;
        let mut replacement = h.packets[0][..prefix].to_vec();
        replacement.extend(comment);
        h.packets[0] = replacement;
        let mut repaired = h.first_page;
        let mut sequence = 1;
        if repairs.is_empty() { repaired = h.original.clone(); sequence = h.pages; }
        else { for packet in &h.packets { paginate(packet, h.state.serial, &mut sequence, &mut repaired, cancel)?; } }
        let serial = h.state.serial;
        let mut source = Sha256::new();
        source.update(&h.original);
        let mut consumed = h.original.len() as u64;
        let mut audio = AudioState::default();
        while consumed < source_length {
            let page = Page::read(input, source_length - consumed, cancel)?;
            h.state.accept(&page)?;
            audio.accept(&page, h.codec, cancel)?;
            consumed += page.bytes.len() as u64;
            source.update(&page.bytes);
        }
        audio.finish(&h.state)?;
        eof(input, cancel)?;
        let output_length = (source_length - h.original.len() as u64).checked_add(repaired.len() as u64)
            .ok_or(TagWriteError::Unsupported("Ogg staged length overflow"))?;
        // One extra page can separate Vorbis comment/setup; policy text cannot grow.
        if output_length > source_length.saturating_add(27) {
            return Err(TagWriteError::Unsupported("Ogg output exceeds policy growth bound"));
        }
        Ok(Self { original_header: h.original, repaired_header: repaired, source_length,
            source_sha256: format!("{:x}", source.finalize()), output_length,
            header_pages: h.pages, sequence_delta: sequence.wrapping_sub(h.pages), serial, codec: h.codec,
            changed_fields: repairs.iter().map(|r| r.field).collect() })
    }
    pub fn changes_bytes(&self) -> bool { self.original_header != self.repaired_header }
    pub fn staged_length(&self) -> u64 { self.output_length }

    /// The caller owns an empty staging stream. Any failure makes its bytes
    /// untrusted; discard, never publish. Evidence hashes emitted bytes, not
    /// persistence. Independent readback, durable journal and publication remain
    /// the platform's responsibility.
    pub fn write_staged(&self, input: &mut (impl Read + Seek), output: &mut impl Write, expected_source_sha256: &str, cancel: &AtomicBool) -> Result<StagedTagEvidence> {
        cancelled(cancel)?;
        if expected_source_sha256 != self.source_sha256 || input.seek(SeekFrom::End(0)).map_err(|_| TagWriteError::Io)? != self.source_length {
            return Err(TagWriteError::Stale);
        }
        input.seek(SeekFrom::Start(0)).map_err(|_| TagWriteError::Io)?;
        let mut original = vec![0; self.original_header.len()];
        read_checked(input, &mut original, cancel)?;
        if original != self.original_header { return Err(TagWriteError::Stale); }
        let mut source = Sha256::new(); source.update(&original);
        let mut staged = Sha256::new();
        let mut media = Sha256::new();
        for chunk in self.repaired_header.chunks(CHUNK) {
            cancelled(cancel)?;
            output.write_all(chunk).map_err(|_| TagWriteError::Io)?;
            staged.update(chunk);
        }
        let mut consumed = original.len() as u64;
        let mut media_length = 0u64;
        let mut state = StreamState { serial: self.serial, next: self.header_pages, continued: false, ended: false };
        let mut audio = AudioState::default();
        while consumed < self.source_length {
            let mut page = Page::read(input, self.source_length - consumed, cancel)?;
            state.accept(&page)?;
            audio.accept(&page, self.codec, cancel)?;
            consumed += page.bytes.len() as u64;
            source.update(&page.bytes);
            media.update(page.payload());
            media_length += page.payload().len() as u64;
            page.shift_sequence(self.sequence_delta, cancel)?;
            cancelled(cancel)?;
            output.write_all(&page.bytes).map_err(|_| TagWriteError::Io)?;
            staged.update(&page.bytes);
        }
        audio.finish(&state)?;
        eof(input, cancel)?;
        let source_sha256 = format!("{:x}", source.finalize());
        if source_sha256 != expected_source_sha256 { return Err(TagWriteError::Stale); }
        cancelled(cancel)?;
        Ok(StagedTagEvidence { source_sha256, staged_sha256: format!("{:x}", staged.finalize()),
            media_sha256: format!("{:x}", media.finalize()), media_length,
            staged_length: self.output_length, changed_fields: self.changed_fields.clone() })
    }
}
