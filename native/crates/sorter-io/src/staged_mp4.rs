//! Exact-extent staged repair of ordinary MP4 iTunes UTF-8 text metadata.
//! No live paths or mutation authority. Movie/media offsets and file length stay
//! unchanged; unrelated metadata atoms, artwork and encoded media are copied.
use crate::{metadata::{project, RawFamily, RawValue}, staged_tags::{StagedTagEvidence, TagWriteError}};
use sha2::{Digest, Sha256};
use sorter_core::{fix_plan::{FieldRepair, FIX_FIELDS}, metadata::{LegacyField, LegacyScalar}};
use std::{io::{Read, Seek, SeekFrom, Write}, sync::atomic::{AtomicBool, Ordering}};
const MAX_MOOV: usize = 16 * 1024 * 1024;
const MAX_BOXES: usize = 65536;
type Result<T> = std::result::Result<T, TagWriteError>;
fn unsupported(reason: &'static str) -> TagWriteError { TagWriteError::Unsupported(reason) }
fn check(cancel: &AtomicBool) -> Result<()> { if cancel.load(Ordering::Acquire) { Err(TagWriteError::Cancelled) } else { Ok(()) } }
fn read_exact(input: &mut impl Read, bytes: &mut [u8], cancel: &AtomicBool) -> Result<()> {
    for chunk in bytes.chunks_mut(65536) { check(cancel)?; let result = input.read_exact(chunk); check(cancel)?; result.map_err(|_| TagWriteError::Io)?; }
    Ok(())
}
#[derive(Clone, Copy, Debug)]
struct Atom { start: usize, end: usize, header: usize, kind: [u8; 4] }
fn atoms(bytes: &[u8], start: usize, end: usize, cancel: &AtomicBool) -> Result<Vec<Atom>> {
    let mut at = start; let mut out = Vec::new();
    while at < end {
        check(cancel)?;
        if out.len() >= MAX_BOXES || end > bytes.len() || end - at < 8 { return Err(unsupported("MP4 atom budget or truncated header")); }
        let size = u32::from_be_bytes(bytes[at..at+4].try_into().unwrap());
        let kind = bytes[at+4..at+8].try_into().unwrap();
        let (size, header) = if size == 1 {
            if end - at < 16 { return Err(unsupported("truncated extended MP4 atom")); }
            (usize::try_from(u64::from_be_bytes(bytes[at+8..at+16].try_into().unwrap())).map_err(|_| unsupported("MP4 atom overflow"))?, 16)
        } else if size == 0 { return Err(unsupported("zero-size nested MP4 atom")); } else { (size as usize, 8) };
        if size < header || size > end - at { return Err(unsupported("invalid MP4 atom extent")); }
        out.push(Atom { start: at, end: at+size, header, kind }); at += size;
    }
    Ok(out)
}
fn one(items: &[Atom], kind: &[u8; 4]) -> Result<Atom> {
    let mut matching = items.iter().filter(|atom| &atom.kind == kind);
    let atom = *matching.next().ok_or(unsupported("required MP4 atom absent"))?;
    if matching.next().is_some() { return Err(unsupported("duplicate structural MP4 atom")); }
    Ok(atom)
}
fn pack(kind: &[u8; 4], body: &[u8]) -> Result<Vec<u8>> {
    let size = u32::try_from(body.len().checked_add(8).ok_or(unsupported("MP4 atom overflow"))?).map_err(|_| unsupported("MP4 atom overflow"))?;
    let mut out = size.to_be_bytes().to_vec(); out.extend(kind); out.extend(body); Ok(out)
}
fn field_key(field: LegacyField) -> Result<[u8; 4]> {
    Ok(match field { LegacyField::Artist => *b"\xa9ART", LegacyField::AlbumArtist => *b"aART", LegacyField::Title => *b"\xa9nam", LegacyField::Album => *b"\xa9alb", LegacyField::Genre => *b"\xa9gen", _ => return Err(unsupported("MP4 repair field")) })
}
fn text_values(bytes: &[u8], atom: Atom, cancel: &AtomicBool) -> Result<Vec<RawValue>> {
    let mut values = Vec::new();
    for data in atoms(bytes, atom.start+atom.header, atom.end, cancel)? {
        if data.kind != *b"data" || data.header != 8 || data.end-data.start < 16 { return Err(unsupported("MP4 text data layout")); }
        let flags = u32::from_be_bytes(bytes[data.start+8..data.start+12].try_into().unwrap());
        if !matches!(flags, 0 | 1) { return Err(unsupported("MP4 text must be implicit or explicit UTF-8")); }
        let value = std::str::from_utf8(&bytes[data.start+16..data.end]).map_err(|_| unsupported("invalid MP4 UTF-8"))?;
        values.push(RawValue::Text(value.into()));
        if values.len() > 4096 { return Err(unsupported("MP4 value budget")); }
    }
    Ok(values)
}
// Movie/media extents stay fixed. Metadata siblings can move inside ilst.
// Refuse encrypted, fragmented,
// item-location and private/integrity layouts rather than promise semantics for
// an unknown structure that could authenticate the edited text.
fn validate_structure(bytes: &[u8], atom: Atom, depth: usize, budget: &mut usize, cancel: &AtomicBool) -> Result<()> {
    check(cancel)?;
    if depth > 12 || *budget == MAX_BOXES { return Err(unsupported("MP4 structural budget")); }
    *budget += 1;
    let container = matches!(&atom.kind, b"moov" | b"trak" | b"mdia" | b"minf" | b"dinf" | b"stbl" | b"edts" | b"udta" | b"meta");
    if container {
        let skip = if atom.kind == *b"meta" { 4 } else { 0 };
        let start = atom.start+atom.header;
        if atom.end-start < skip || (skip == 4 && bytes[start..start+4] != [0;4]) { return Err(unsupported("MP4 metadata version")); }
        for child in atoms(bytes, start+skip, atom.end, cancel)? { validate_structure(bytes, child, depth+1, budget, cancel)?; }
    } else if atom.kind == *b"ilst" {
        // Opaque ilst items/artwork are preserved byte-for-byte. Only the five
        // selected text items are parsed; no generic TagLib save is invoked.
        atoms(bytes, atom.start+atom.header, atom.end, cancel)?;
    } else if atom.kind == *b"stsd" {
        let body = atom.start+atom.header;
        if atom.end-body < 8 || bytes[body..body+4] != [0;4] { return Err(unsupported("MP4 sample-description version")); }
        let samples = atoms(bytes, body+8, atom.end, cancel)?;
        let count = u32::from_be_bytes(bytes[body+4..body+8].try_into().unwrap()) as usize;
        if samples.len() != count || samples.is_empty() { return Err(unsupported("MP4 sample-description count")); }
        for sample in samples {
            if !matches!(&sample.kind, b"mp4a" | b"alac") || sample.end-sample.start < sample.header+28
                || bytes[sample.start+sample.header+6..sample.start+sample.header+8] != [0,1]
                || bytes[sample.start+sample.header+8..sample.start+sample.header+10] != [0,0]
            { return Err(unsupported("only local unencrypted AAC/ALAC descriptions")); }
            for extension in atoms(bytes,sample.start+sample.header+28,sample.end,cancel)? {
                if !matches!(&extension.kind,b"esds" | b"alac" | b"btrt") { return Err(unsupported("unproven MP4 audio sample extension")); }
            }
        }
    } else if atom.kind == *b"dref" {
        let body = atom.start+atom.header;
        if atom.end-body < 8 || bytes[body..body+4] != [0;4] || bytes[body+4..body+8] != [0,0,0,1] { return Err(unsupported("MP4 data-reference layout")); }
        let entries = atoms(bytes, body+8, atom.end, cancel)?;
        if entries.len() != 1 || entries[0].kind != *b"url " || entries[0].end-entries[0].start != 12
            || bytes[entries[0].start+8..entries[0].end] != [0,0,0,1] { return Err(unsupported("external MP4 data reference")); }
    } else if matches!(&atom.kind,b"sgpd"|b"sbgp") {
        let body = &bytes[atom.start+atom.header..atom.end];
        if body.len()<8 || !matches!(&body[4..8],b"roll"|b"prol") {return Err(unsupported("unproven MP4 sample grouping"));}
    } else if !matches!(&atom.kind, b"mvhd" | b"tkhd" | b"mdhd" | b"hdlr" | b"smhd" | b"stts" | b"ctts" | b"stsc" | b"stsz" | b"stz2" | b"stco" | b"co64" | b"stss" | b"sdtp" | b"sgpd" | b"sbgp" | b"elst" | b"free" | b"skip") {
        return Err(unsupported("unproven MP4 structural atom"));
    }
    Ok(())
}
const MAX_SAMPLES: usize = 1_000_000;
fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(bytes.get(at..at.checked_add(4).ok_or(unsupported("MP4 table overflow"))?).ok_or(unsupported("truncated MP4 table"))?.try_into().unwrap()))
}
fn table_body(bytes: &[u8], atom: Atom) -> Result<&[u8]> {
    let body = &bytes[atom.start+atom.header..atom.end];
    if body.len()<4 || body[..4]!=[0;4] {return Err(unsupported("MP4 table version/flags"));}
    Ok(body)
}
fn validate_samples(bytes: &[u8], root: Atom, media: &[(u64,u64)], cancel: &AtomicBool) -> Result<()> {
    let movie = atoms(bytes,root.start+root.header,root.end,cancel)?;
    let mut total_samples = 0usize;
    for track in movie.iter().filter(|atom|atom.kind==*b"trak") {
        check(cancel)?;
        let mdia = one(&atoms(bytes,track.start+track.header,track.end,cancel)?,b"mdia")?;
        let mdia_children = atoms(bytes,mdia.start+mdia.header,mdia.end,cancel)?;
        let handler = one(&mdia_children,b"hdlr")?;
        let handler_body = table_body(bytes,handler)?;
        if handler_body.get(8..12)!=Some(b"soun".as_slice()) {return Err(unsupported("MP4 audio tracks only"));}
        let minf = one(&mdia_children,b"minf")?;
        let minf_children = atoms(bytes,minf.start+minf.header,minf.end,cancel)?;
        let dinf = one(&minf_children,b"dinf")?;
        one(&atoms(bytes,dinf.start+dinf.header,dinf.end,cancel)?,b"dref")?;
        let stbl = one(&minf_children,b"stbl")?;
        let tables = atoms(bytes,stbl.start+stbl.header,stbl.end,cancel)?;
        let descriptions = table_body(bytes,one(&tables,b"stsd")?)?;
        let description_count = u32_at(descriptions,4)?;
        let offsets = tables.iter().filter(|atom|matches!(&atom.kind,b"stco"|b"co64")).copied().collect::<Vec<_>>();
        if offsets.len()!=1 {return Err(unsupported("MP4 requires one chunk-offset table"));}
        let offsets_atom = offsets[0]; let offsets = table_body(bytes,offsets_atom)?;
        let chunk_count = u32_at(offsets,4)? as usize;
        let width = if offsets_atom.kind==*b"stco" {4}else{8};
        if chunk_count==0 || chunk_count>MAX_SAMPLES || offsets.len()!=8usize.checked_add(chunk_count.checked_mul(width).ok_or(unsupported("MP4 chunk overflow"))?).ok_or(unsupported("MP4 chunk overflow"))? {return Err(unsupported("MP4 chunk count/extent"));}
        let sizing = tables.iter().filter(|atom|matches!(&atom.kind,b"stsz"|b"stz2")).copied().collect::<Vec<_>>();
        if sizing.len()!=1 || sizing[0].kind!=*b"stsz" {return Err(unsupported("MP4 requires ordinary stsz sample sizes"));}
        let sizes = table_body(bytes,sizing[0])?;
        let common_size = u32_at(sizes,4)?;
        let sample_count = u32_at(sizes,8)? as usize;
        total_samples = total_samples.checked_add(sample_count).ok_or(unsupported("MP4 sample budget"))?;
        if sample_count==0 || total_samples>MAX_SAMPLES || sizes.len()!=12usize.checked_add(if common_size==0 {sample_count.checked_mul(4).ok_or(unsupported("MP4 sample overflow"))?}else{0}).ok_or(unsupported("MP4 sample overflow"))? {return Err(unsupported("MP4 sample count/extent"));}
        let schedule = table_body(bytes,one(&tables,b"stsc")?)?;
        let run_count = u32_at(schedule,4)? as usize;
        if run_count==0 || run_count>chunk_count || schedule.len()!=8usize.checked_add(run_count.checked_mul(12).ok_or(unsupported("MP4 schedule overflow"))?).ok_or(unsupported("MP4 schedule overflow"))? {return Err(unsupported("MP4 chunk schedule extent"));}
        let mut runs = Vec::with_capacity(run_count);
        for i in 0..run_count {
            check(cancel)?;
            let at=8+i*12; let first=u32_at(schedule,at)? as usize; let samples=u32_at(schedule,at+4)? as usize; let description=u32_at(schedule,at+8)?;
            if first==0 || first>chunk_count || samples==0 || description==0 || description>description_count || (i==0&&first!=1) || runs.last().is_some_and(|&(old,_)|old>=first) {return Err(unsupported("MP4 chunk schedule mapping"));}
            runs.push((first,samples));
        }
        let timing = table_body(bytes,one(&tables,b"stts")?)?;
        let timing_count=u32_at(timing,4)? as usize;
        if timing_count>MAX_SAMPLES || timing.len()!=8usize.checked_add(timing_count.checked_mul(8).ok_or(unsupported("MP4 timing overflow"))?).ok_or(unsupported("MP4 timing overflow"))? {return Err(unsupported("MP4 timing extent"));}
        let mut timed=0u64;
        for i in 0..timing_count {check(cancel)?;timed=timed.checked_add(u64::from(u32_at(timing,8+i*8)?)).ok_or(unsupported("MP4 timing overflow"))?;}
        if timed!=sample_count as u64 {return Err(unsupported("MP4 timing/sample count mismatch"));}
        let mut run=0; let mut sample=0usize;
        for chunk in 0..chunk_count {
            check(cancel)?;
            while run+1<runs.len() && runs[run+1].0<=chunk+1 {run+=1;}
            let end_sample=sample.checked_add(runs[run].1).ok_or(unsupported("MP4 sample range overflow"))?;
            if end_sample>sample_count {return Err(unsupported("MP4 chunk exceeds sample count"));}
            let length=if common_size!=0 {u64::from(common_size).checked_mul(runs[run].1 as u64).ok_or(unsupported("MP4 chunk length overflow"))?}else{
                let mut length=0u64;for index in sample..end_sample {check(cancel)?;length=length.checked_add(u64::from(u32_at(sizes,12+index*4)?)).ok_or(unsupported("MP4 chunk length overflow"))?;}length
            };
            let pos=8+chunk*width;
            let start=if width==4 {u64::from(u32_at(offsets,pos)?)}else{u64::from_be_bytes(offsets[pos..pos+8].try_into().unwrap())};
            let end=start.checked_add(length).ok_or(unsupported("MP4 media offset overflow"))?;
            let index=media.partition_point(|&(begin,_)|begin<=start);
            if index==0 || end>media[index-1].1 {return Err(unsupported("MP4 sample bytes outside local media"));}
            sample=end_sample;
        }
        if sample!=sample_count {return Err(unsupported("MP4 unmapped samples"));}
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Mp4StagedPlan {
    source_length: u64,
    moov_offset: u64,
    original_moov: Vec<u8>,
    repaired_moov: Vec<u8>,
    media: Vec<(u64,u64)>,
    changed_fields: Vec<LegacyField>,
}
impl Mp4StagedPlan {
    pub fn inspect(input: &mut (impl Read+Seek), source_length: u64, repairs: &[FieldRepair], cancel: &AtomicBool) -> Result<Self> {
        check(cancel)?;
        let mut at = 0u64; let mut count = 0; let mut moov = None; let mut media = Vec::new(); let mut ftyp = false;
        while at < source_length {
            check(cancel)?; count += 1;
            if count > MAX_BOXES || source_length-at < 8 { return Err(unsupported("MP4 top-level budget or extent")); }
            input.seek(SeekFrom::Start(at)).map_err(|_| TagWriteError::Io)?;
            let mut header = [0u8;16]; read_exact(input, &mut header[..8], cancel)?;
            let short = u32::from_be_bytes(header[..4].try_into().unwrap());
            let kind: [u8;4] = header[4..8].try_into().unwrap();
            let (size, head) = if short == 1 { read_exact(input, &mut header[8..], cancel)?; (u64::from_be_bytes(header[8..].try_into().unwrap()),16) }
                else if short == 0 { (source_length-at,8) } else { (u64::from(short),8) };
            if size < head || size > source_length-at { return Err(unsupported("MP4 top-level extent")); }
            match &kind {
                b"ftyp" if at == 0 && size >= head+8 && (size-head-8).is_multiple_of(4) => ftyp = true,
                b"moov" if moov.is_none() && size <= MAX_MOOV as u64 && head == 8 && short != 0 => {
                    input.seek(SeekFrom::Start(at)).map_err(|_| TagWriteError::Io)?;
                    let mut bytes = vec![0;size as usize]; read_exact(input,&mut bytes,cancel)?; moov = Some((at,bytes));
                }
                b"mdat" => media.push((at+head,at+size)),
                b"free" | b"skip" | b"wide" => {},
                _ => return Err(unsupported("unproven or duplicate MP4 top-level atom")),
            }
            at += size;
        }
        if !ftyp || media.is_empty() { return Err(unsupported("MP4 movie/media required")); }
        let (moov_offset,original_moov) = moov.ok_or(unsupported("MP4 movie required"))?;
        let root = one(&atoms(&original_moov,0,original_moov.len(),cancel)?, b"moov")?;
        validate_structure(&original_moov,root,0,&mut 0,cancel)?;
        validate_samples(&original_moov,root,&media,cancel)?;
        if !atoms(&original_moov,8,root.end,cancel)?.iter().any(|atom|atom.kind==*b"trak") { return Err(unsupported("MP4 audio track required")); }
        let udta = one(&atoms(&original_moov,8,root.end,cancel)?,b"udta")?;
        let meta = one(&atoms(&original_moov,udta.start+udta.header,udta.end,cancel)?,b"meta")?;
        let ilst = one(&atoms(&original_moov,meta.start+meta.header+4,meta.end,cancel)?,b"ilst")?;
        if ilst.header != 8 { return Err(unsupported("extended MP4 metadata-list header")); }
        let items = atoms(&original_moov,ilst.start+8,ilst.end,cancel)?;
        for field in FIX_FIELDS {
            let key = field_key(field)?;
            let matching: Vec<_> = items.iter().filter(|item|item.kind==key).collect();
            if matching.len()>1 {
                let mut count=0usize;
                for item in matching { count=count.checked_add(text_values(&original_moov,*item,cancel)?.len()).ok_or(unsupported("MP4 value budget"))?; if count>4096 { return Err(unsupported("MP4 value budget")); } }
            }
        }
        let mut replacements = Vec::new(); let mut previous = None;
        for repair in repairs {
            check(cancel)?;
            let index = FIX_FIELDS.iter().position(|field| *field == repair.field).ok_or(unsupported("MP4 repair field"))?;
            if previous.is_some_and(|old| old >= index) || repair.original == repair.replacement
                || sorter_core::fix_jibrish(&repair.original,sorter_core::RepairMode::Hebrew) != repair.replacement
                || repair.original.len() != repair.replacement.len() { return Err(unsupported("MP4 exact-width original repair policy")); }
            if repair.field == LegacyField::Genre && items.iter().any(|item|item.kind==*b"gnre") { return Err(unsupported("legacy numeric genre mirror")); }
            previous = Some(index); let key = field_key(repair.field)?; let mut values = Vec::new();
            for item in items.iter().filter(|item| item.kind == key) { values.extend(text_values(&original_moov,*item,cancel)?); if values.len()>4096 { return Err(unsupported("MP4 value budget")); } }
            let scalar = project(RawFamily::Mp4,repair.field,values).map_err(|_| TagWriteError::Stale)?.value.map_err(|_| TagWriteError::Stale)?;
            if scalar != LegacyScalar::Text(repair.original.clone()) { return Err(TagWriteError::Stale); }
            let mut data = vec![0,0,0,1,0,0,0,0]; data.extend(repair.replacement.as_bytes());
            replacements.push((key,pack(&key,&pack(b"data",&data)?)?));
        }
        let mut rebuilt = Vec::new(); let mut seen = vec![false;replacements.len()];
        for item in &items {
            if let Some(index) = replacements.iter().position(|(key,_)| *key == item.kind) {
                if !seen[index] { rebuilt.extend(&replacements[index].1); seen[index] = true; }
            } else { rebuilt.extend_from_slice(&original_moov[item.start..item.end]); }
        }
        if seen.iter().any(|seen| !seen) { return Err(TagWriteError::Stale); }
        let mut replacement = pack(b"ilst",&rebuilt)?;
        let old_length = ilst.end-ilst.start;
        let slack = old_length.checked_sub(replacement.len()).ok_or(unsupported("MP4 rewrite unexpectedly grows"))?;
        if slack > 0 {
            if slack < 8 { return Err(unsupported("MP4 padding remainder below atom header")); }
            replacement.extend(pack(b"free",&vec![0;slack-8])?);
        }
        let mut repaired_moov = original_moov.clone(); repaired_moov[ilst.start..ilst.end].copy_from_slice(&replacement);
        Ok(Self { source_length,moov_offset,original_moov,repaired_moov,media,changed_fields:repairs.iter().map(|repair|repair.field).collect() })
    }
    pub fn changes_bytes(&self) -> bool { self.original_moov != self.repaired_moov }
    pub fn staged_length(&self) -> u64 { self.source_length }
    pub fn write_staged(&self,input: &mut (impl Read+Seek),output: &mut impl Write,expected_source_sha256: &str,cancel: &AtomicBool) -> Result<StagedTagEvidence> {
        check(cancel)?; input.seek(SeekFrom::Start(0)).map_err(|_| TagWriteError::Io)?;
        let mut source = Sha256::new(); let mut staged = Sha256::new(); let mut media_hash = Sha256::new(); let mut media_length = 0u64; let mut at = 0u64; let mut media_index = 0; let mut buffer = [0u8;65536];
        while at < self.source_length {
            let count = usize::try_from((self.source_length-at).min(buffer.len() as u64)).unwrap(); read_exact(input,&mut buffer[..count],cancel)?;
            let bytes = &mut buffer[..count]; source.update(&*bytes);
            while media_index < self.media.len() && self.media[media_index].1 <= at { media_index += 1; }
            for &(start,end) in &self.media[media_index..] {
                if start >= at+count as u64 { break; }
                let left = at.max(start); let right = (at+count as u64).min(end);
                if left < right { media_hash.update(&bytes[(left-at) as usize..(right-at) as usize]); media_length = media_length.checked_add(right-left).ok_or(unsupported("MP4 media length overflow"))?; }
            }
            let left = at.max(self.moov_offset); let right = (at+count as u64).min(self.moov_offset+self.original_moov.len() as u64);
            if left < right {
                let local = (left-at) as usize..(right-at) as usize; let movie = (left-self.moov_offset) as usize..(right-self.moov_offset) as usize;
                if bytes[local.clone()] != self.original_moov[movie.clone()] { return Err(TagWriteError::Stale); }
                bytes[local].copy_from_slice(&self.repaired_moov[movie]);
            }
            staged.update(&*bytes); check(cancel)?; let result = output.write_all(bytes); check(cancel)?; result.map_err(|_| TagWriteError::Io)?; at += count as u64;
        }
        let mut extra = [0]; check(cancel)?; let result = input.read(&mut extra); check(cancel)?;
        if result.map_err(|_| TagWriteError::Io)? != 0 { return Err(TagWriteError::Stale); }
        let source_sha256 = format!("{:x}",source.finalize());
        if source_sha256 != expected_source_sha256 { return Err(TagWriteError::Stale); }
        Ok(StagedTagEvidence { source_sha256,staged_sha256:format!("{:x}",staged.finalize()),media_sha256:format!("{:x}",media_hash.finalize()),media_length,staged_length:self.source_length,changed_fields:self.changed_fields.clone() })
    }
}

/// Read-only correction for TagLib's first-outer-item MP4 projection. Only
/// duplicate occurrences of the five legacy text keys override the native
/// reader. Parsing uses the same held input; no path or mutation authority.
#[cfg(native_metadata_bridge)]
pub(crate) type DuplicatePreviewFields = Vec<(LegacyField, std::result::Result<Vec<RawValue>, sorter_core::metadata::FieldError>)>;
#[cfg(native_metadata_bridge)]
pub(crate) fn duplicate_preview_fields(
    input: &mut (impl Read + Seek),
    source_length: u64,
) -> Result<DuplicatePreviewFields> {
    let cancel = AtomicBool::new(false);
    let mut at = 0u64;
    let mut count = 0usize;
    let mut movie = None;
    while at < source_length {
        count += 1;
        if count > MAX_BOXES || source_length-at < 8 { return Err(unsupported("MP4 preview atom budget or extent")); }
        input.seek(SeekFrom::Start(at)).map_err(|_| TagWriteError::Io)?;
        let mut header = [0u8;16];
        read_exact(input, &mut header[..8], &cancel)?;
        let short = u32::from_be_bytes(header[..4].try_into().unwrap());
        let (size, head) = if short == 1 {
            read_exact(input, &mut header[8..], &cancel)?;
            (u64::from_be_bytes(header[8..].try_into().unwrap()),16)
        } else if short == 0 { (source_length-at,8) } else { (u64::from(short),8) };
        if size < head || size > source_length-at { return Err(unsupported("MP4 preview atom extent")); }
        if &header[4..8] == b"moov" {
            if movie.is_some() || size > MAX_MOOV as u64 { return Err(unsupported("MP4 preview movie budget or duplicate")); }
            input.seek(SeekFrom::Start(at)).map_err(|_| TagWriteError::Io)?;
            let mut bytes = vec![0;size as usize];
            read_exact(input, &mut bytes, &cancel)?;
            movie = Some((bytes,head as usize));
        }
        at += size;
    }
    let Some((bytes, head)) = movie else { return Ok(Vec::new()); };
    let children = atoms(&bytes,head,bytes.len(),&cancel)?;
    if !children.iter().any(|a|a.kind==*b"udta") { return Ok(Vec::new()); }
    let udta = one(&children,b"udta")?;
    let children = atoms(&bytes,udta.start+udta.header,udta.end,&cancel)?;
    if !children.iter().any(|a|a.kind==*b"meta") { return Ok(Vec::new()); }
    let meta = one(&children,b"meta")?;
    let body = meta.start+meta.header;
    if meta.end-body < 4 || bytes[body..body+4] != [0;4] { return Err(unsupported("MP4 preview metadata version")); }
    let children = atoms(&bytes,body+4,meta.end,&cancel)?;
    if !children.iter().any(|a|a.kind==*b"ilst") { return Ok(Vec::new()); }
    let ilst = one(&children,b"ilst")?;
    let items = atoms(&bytes,ilst.start+ilst.header,ilst.end,&cancel)?;
    let mut result = Vec::new();
    for field in FIX_FIELDS {
        let key = field_key(field)?;
        let matching: Vec<_> = items.iter().filter(|item|item.kind==key).copied().collect();
        if matching.len() < 2 { continue; }
        let values = (|| {
            let mut values = Vec::new();
            for item in matching {
                values.extend(text_values(&bytes,item,&cancel)?);
                if values.len() > 4096 { return Err(unsupported("MP4 preview value budget")); }
            }
            Ok(values)
        })();
        result.push((field,values.map_err(|_| sorter_core::metadata::FieldError::get(
            "UnsupportedMp4TextRepresentation", "duplicate MP4 text projection requires bounded UTF-8 data atoms",
        ))));
    }
    Ok(result)
}
