//! Exact-extent ID3 repair in validated ordinary WAVE, AIFF and DSF containers.
//! Existing tag, chunk headers, padding and media offsets are retained.
use crate::staged_tags::{rewrite_id3_block,StagedTagEvidence,TagWriteError};
use sorter_core::{fix_plan::FieldRepair,metadata::LegacyField};
use sha2::{Digest,Sha256};
use std::{io::{Read,Seek,SeekFrom,Write},sync::atomic::{AtomicBool,Ordering}};
type Result<T> = std::result::Result<T,TagWriteError>;
fn bad(reason:&'static str)->TagWriteError{TagWriteError::Unsupported(reason)}
fn check(flag:&AtomicBool)->Result<()>{if flag.load(Ordering::Acquire){Err(TagWriteError::Cancelled)}else{Ok(())}}
fn read(input:&mut impl Read,bytes:&mut[u8],flag:&AtomicBool)->Result<()>{for chunk in bytes.chunks_mut(65536){check(flag)?;let result=input.read_exact(chunk);check(flag)?;result.map_err(|_|TagWriteError::Io)?;}Ok(())}
fn at(input:&mut(impl Read+Seek),offset:u64,bytes:&mut[u8],flag:&AtomicBool)->Result<()>{check(flag)?;let result=input.seek(SeekFrom::Start(offset));check(flag)?;result.map_err(|_|TagWriteError::Io)?;read(input,bytes,flag)}
fn le32(bytes:&[u8])->u32{u32::from_le_bytes(bytes.try_into().unwrap())}
fn be32(bytes:&[u8])->u32{u32::from_be_bytes(bytes.try_into().unwrap())}
fn le64(bytes:&[u8])->u64{u64::from_le_bytes(bytes.try_into().unwrap())}
#[derive(Clone,Debug)]
pub struct ContainerId3StagedPlan{length:u64,tag_offset:u64,original:Vec<u8>,replacement:Vec<u8>,media:(u64,u64),fields:Vec<LegacyField>}
impl ContainerId3StagedPlan{
 pub fn inspect(input:&mut(impl Read+Seek),length:u64,repairs:&[FieldRepair],flag:&AtomicBool)->Result<Self>{
  if length<12{return Err(bad("container header truncated"));}
  let mut header=[0u8;12];at(input,0,&mut header,flag)?;
  let (tag_offset,tag_length,media)=if &header[..4]==b"DSD " {Self::dsf(input,length,flag)?}
  else {Self::iff(input,length,&header,flag)?};
  if !(10..=16*1024*1024+10).contains(&tag_length){return Err(bad("container ID3 budget"));}
  let mut original=vec![0;tag_length as usize];at(input,tag_offset,&mut original,flag)?;
  let replacement=rewrite_id3_block(&original,repairs,flag)?;
  Ok(Self{length,tag_offset,original,replacement,media,fields:repairs.iter().map(|r|r.field).collect()})
 }
 fn iff(input:&mut(impl Read+Seek),length:u64,header:&[u8;12],flag:&AtomicBool)->Result<(u64,u64,(u64,u64))>{
  let wave=&header[..4]==b"RIFF"&&&header[8..]==b"WAVE";
  let aiff=&header[..4]==b"FORM"&&&header[8..]==b"AIFF";
  if !wave&&!aiff{return Err(bad("ordinary RIFF WAVE or uncompressed AIFF required"));}
  let size=if wave{le32(&header[4..8])}else{be32(&header[4..8])};
  if u64::from(size)+8!=length{return Err(bad("container size mismatch or trailing data"));}
  let mut offset=12;let mut count=0;let mut tag=None;let mut media=None;let mut format=None;
  while offset<length{
   check(flag)?;count+=1;if count>65536||length-offset<8{return Err(bad("container chunk budget or header"));}
   let mut chunk=[0u8;8];at(input,offset,&mut chunk,flag)?;
   let size=u64::from(if wave{le32(&chunk[4..])}else{be32(&chunk[4..])});
   let start=offset+8;let end=start.checked_add(size).ok_or(bad("chunk overflow"))?;
   let next=end.checked_add(size&1).ok_or(bad("chunk padding overflow"))?;
   if next>length{return Err(bad("chunk exceeds container"));}
   match &chunk[..4]{
    b"ID3 "|b"id3 "=>{if tag.replace((start,size)).is_some(){return Err(bad("duplicate ID3 chunks"));}}
    b"fmt " if wave=>{if format.is_some()||size!=16{return Err(bad("ordinary unique PCM format required"));}let mut data=[0;16];at(input,start,&mut data,flag)?;format=Some(data.to_vec());}
    b"COMM" if aiff=>{if format.is_some()||size!=18{return Err(bad("ordinary unique AIFF common chunk required"));}let mut data=[0;18];at(input,start,&mut data,flag)?;format=Some(data.to_vec());}
    b"data" if wave=>{if media.replace((start,end)).is_some(){return Err(bad("duplicate audio chunks"));}}
    b"SSND" if aiff=>{if media.is_some()||size<8{return Err(bad("AIFF sound chunk"));}let mut data=[0;8];at(input,start,&mut data,flag)?;let skip=u64::from(be32(&data[..4]));if skip>size-8||be32(&data[4..])!=0{return Err(bad("AIFF sound offset or block layout"));}media=Some((start+8+skip,end));}
    _=>{},
   }
   offset=next;
  }
  let format=format.ok_or(bad("container audio format absent"))?;let media=media.ok_or(bad("container audio absent"))?;
  if wave{
   let code=u16::from_le_bytes(format[..2].try_into().unwrap());let channels=u16::from_le_bytes(format[2..4].try_into().unwrap());let rate=le32(&format[4..8]);let align=u16::from_le_bytes(format[12..14].try_into().unwrap());let bits=u16::from_le_bytes(format[14..16].try_into().unwrap());
   if !matches!(code,1|3)||(code==3&&!matches!(bits,32|64))||channels==0||rate==0||bits==0||!bits.is_multiple_of(8)||u32::from(align)!=u32::from(channels)*u32::from(bits/8)||align==0||le32(&format[8..12])!=rate.checked_mul(u32::from(align)).ok_or(bad("WAVE rate overflow"))?||!(media.1-media.0).is_multiple_of(u64::from(align)){return Err(bad("WAVE sample layout"));}
  }else{
   let channels=u16::from_be_bytes(format[..2].try_into().unwrap());let frames=be32(&format[2..6]);let bits=u16::from_be_bytes(format[6..8].try_into().unwrap());
   let sample_bytes=u64::from(bits).div_ceil(8);let expected=u64::from(frames).checked_mul(u64::from(channels)).and_then(|v|v.checked_mul(sample_bytes)).ok_or(bad("AIFF sample extent overflow"))?;
   if channels==0||bits==0||bits>64||expected!=media.1-media.0||format[8]&128!=0||format[8..10]==[0x7f,0xff]||format[10..18].iter().all(|b|*b==0){return Err(bad("AIFF sample layout"));}
  }
  let (start,size)=tag.ok_or(bad("existing container ID3 tag required"))?;Ok((start,size,media))
 }
 fn dsf(input:&mut(impl Read+Seek),length:u64,flag:&AtomicBool)->Result<(u64,u64,(u64,u64))>{
  if length<92{return Err(bad("DSF fixed headers truncated"));}
  let mut bytes=[0;92];at(input,0,&mut bytes,flag)?;
  let pointer=le64(&bytes[20..28]);let data_size=le64(&bytes[84..92]);
  if &bytes[..4]!=b"DSD "||le64(&bytes[4..12])!=28||le64(&bytes[12..20])!=length||&bytes[28..32]!=b"fmt "||le64(&bytes[32..40])!=52||le32(&bytes[40..44])!=1||le32(&bytes[44..48])!=0||&bytes[80..84]!=b"data"||data_size<12||80u64.checked_add(data_size)!=Some(pointer)||pointer>=length{return Err(bad("DSF chunk or terminal metadata pointer"));}
  let channels=le32(&bytes[52..56]);let bits=le32(&bytes[60..64]);let samples=le64(&bytes[64..72]);let block=u64::from(le32(&bytes[72..76]));
  if channels==0||channels>6||!matches!((le32(&bytes[48..52]),channels),(1,1)|(2,2)|(3,3)|(4,4)|(5,4)|(6,5)|(7,6))||le32(&bytes[56..60])==0||!matches!(bits,1|8)||block!=4096||le32(&bytes[76..80])!=0{return Err(bad("ordinary DSF sample format"));}
  let expected=samples.div_ceil(8).div_ceil(block).checked_mul(block).and_then(|v|v.checked_mul(u64::from(channels))).ok_or(bad("DSF sample extent overflow"))?;
  if expected!=data_size-12{return Err(bad("DSF padded sample extent"));}
  Ok((pointer,length-pointer,(92,pointer)))
 }
 pub fn changes_bytes(&self)->bool{self.original!=self.replacement}
 pub fn staged_length(&self)->u64{self.length}
 pub fn write_staged(&self,input:&mut(impl Read+Seek),output:&mut impl Write,expected:&str,flag:&AtomicBool)->Result<StagedTagEvidence>{
  check(flag)?;let seek=input.seek(SeekFrom::Start(0));check(flag)?;seek.map_err(|_|TagWriteError::Io)?;
  let mut source=Sha256::new();let mut staged=Sha256::new();let mut media=Sha256::new();let mut pos=0u64;let mut buffer=[0;65536];
  while pos<self.length{
   let count=(self.length-pos).min(buffer.len()as u64)as usize;read(input,&mut buffer[..count],flag)?;let data=&mut buffer[..count];source.update(&*data);
   let left=pos.max(self.media.0);let right=(pos+count as u64).min(self.media.1);if left<right{media.update(&data[(left-pos)as usize..(right-pos)as usize]);}
   let left=pos.max(self.tag_offset);let right=(pos+count as u64).min(self.tag_offset+self.original.len()as u64);
   if left<right{let a=(left-pos)as usize;let b=(right-pos)as usize;let x=(left-self.tag_offset)as usize;let y=(right-self.tag_offset)as usize;if data[a..b]!=self.original[x..y]{return Err(TagWriteError::Stale);}data[a..b].copy_from_slice(&self.replacement[x..y]);}
   staged.update(&*data);check(flag)?;let result=output.write_all(data);check(flag)?;result.map_err(|_|TagWriteError::Io)?;pos+=count as u64;
  }
  let mut extra=[0];check(flag)?;let eof=input.read(&mut extra);check(flag)?;if eof.map_err(|_|TagWriteError::Io)?!=0{return Err(TagWriteError::Stale);}
  let source_sha256=format!("{:x}",source.finalize());if source_sha256!=expected{return Err(TagWriteError::Stale);}
  Ok(StagedTagEvidence{source_sha256,staged_sha256:format!("{:x}",staged.finalize()),media_sha256:format!("{:x}",media.finalize()),media_length:self.media.1-self.media.0,staged_length:self.length,changed_fields:self.fields.clone()})
 }
}
