//! Generated owned containers based on the repository's licensed tiny media.
pub const KINDS:[&str;3]=["wav","aiff","dsf"];
fn sync(n:usize)->[u8;4]{[((n>>21)&127)as u8,((n>>14)&127)as u8,((n>>7)&127)as u8,(n&127)as u8]}
pub fn tag(version:u8,padding:usize)->Vec<u8>{
 let mut body=Vec::new();
 for id in [b"TPE1",b"TPE2",b"TIT2",b"TALB",b"TCON"] {body.extend(id);body.extend(if version==4{sync(2)}else{2u32.to_be_bytes()});body.extend([0,0,0,0xe0]);}
 for(id,payload)in[(b"PRIV",b"owned\0opaque\xff".as_slice()),(b"APIC",b"\0image/png\0\x03\0retained-artwork".as_slice())]{body.extend(id);body.extend(if version==4{sync(payload.len())}else{(payload.len()as u32).to_be_bytes()});body.extend([0,0]);body.extend(payload);}
 body.resize(body.len()+padding,0);let mut bytes=vec![b'I',b'D',b'3',version,0,0];bytes.extend(sync(body.len()));bytes.extend(body);bytes
}
pub fn fixture(kind:&str,version:u8,padding:usize)->Vec<u8>{
 let mut bytes=match kind{"wav"=>include_bytes!("../fixtures/media/blank.wav").to_vec(),"aiff"=>include_bytes!("../fixtures/media/blank.aiff").to_vec(),"dsf"=>include_bytes!("../fixtures/media/blank.dsf").to_vec(),_=>panic!("fixture kind")};
 let tag=tag(version,padding);
 if kind=="dsf"{let at=bytes.len()as u64;bytes[20..28].copy_from_slice(&at.to_le_bytes());bytes.extend(tag);let len=bytes.len()as u64;bytes[12..20].copy_from_slice(&len.to_le_bytes());}
 else{append_chunk(&mut bytes,kind,b"JUNK",b"opaque chunk\0\xff");if kind=="wav"{append_chunk(&mut bytes,kind,b"LIST",b"INFOINAM\x04\0\0\0old\0");}append_chunk(&mut bytes,kind,b"ID3 ",&tag);}
 bytes
}
pub fn append_chunk(bytes:&mut Vec<u8>,kind:&str,id:&[u8;4],data:&[u8]){bytes.extend(id);bytes.extend(if kind=="wav"{(data.len()as u32).to_le_bytes()}else{(data.len()as u32).to_be_bytes()});bytes.extend(data);if !data.len().is_multiple_of(2){bytes.push(0x5a);}let len=bytes.len()as u32-8;bytes[4..8].copy_from_slice(&if kind=="wav"{len.to_le_bytes()}else{len.to_be_bytes()});}
pub fn tag_span(bytes:&[u8],kind:&str)->(usize,usize){if kind=="dsf"{let at=u64::from_le_bytes(bytes[20..28].try_into().unwrap())as usize;return(at,bytes.len());}let mut at=12;while at<bytes.len(){let n=if kind=="wav"{u32::from_le_bytes(bytes[at+4..at+8].try_into().unwrap())}else{u32::from_be_bytes(bytes[at+4..at+8].try_into().unwrap())}as usize;if &bytes[at..at+4]==b"ID3 "{return(at+8,at+8+n);}at+=8+n+(n&1);}panic!("tag");}
