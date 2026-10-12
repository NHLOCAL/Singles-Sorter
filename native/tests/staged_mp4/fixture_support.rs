pub const AAC: &[u8] = include_bytes!("../fixtures/media/blank.m4a");
pub const ALAC: &[u8] = include_bytes!("../fixtures/media/blank.alac");
pub fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(body);
    out
}
pub fn children(bytes: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = vec![];
    let mut at = 0;
    while at < bytes.len() {
        let n = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        assert!(n >= 8 && at + n <= bytes.len());
        out.push((
            bytes[at + 4..at + 8].try_into().unwrap(),
            &bytes[at..at + n],
        ));
        at += n;
    }
    out
}
pub fn text(kind: &[u8; 4], values: &[&str]) -> Vec<u8> {
    let mut body = vec![];
    for value in values {
        let mut data = vec![0, 0, 0, 1, 0, 0, 0, 0];
        data.extend(value.as_bytes());
        body.extend(atom(b"data", &data));
    }
    atom(kind, &body)
}
pub fn ilst(bytes: &[u8]) -> &[u8] {
    let movie = children(bytes)
        .into_iter()
        .find(|(kind, _)| kind == b"moov")
        .unwrap()
        .1;
    let user = children(&movie[8..])
        .into_iter()
        .find(|(kind, _)| kind == b"udta")
        .unwrap()
        .1;
    let meta = children(&user[8..])
        .into_iter()
        .find(|(kind, _)| kind == b"meta")
        .unwrap()
        .1;
    children(&meta[12..])
        .into_iter()
        .find(|(kind, _)| kind == b"ilst")
        .unwrap()
        .1
}
pub fn replace_list(bytes: &[u8], list: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    for (kind, box_bytes) in children(bytes) {
        if &kind == b"ilst" {
            out.extend(atom(&kind, list));
        } else if matches!(&kind, b"moov" | b"udta") {
            out.extend(atom(&kind, &replace_list(&box_bytes[8..], list)));
        } else if &kind == b"meta" {
            let mut body = box_bytes[8..12].to_vec();
            body.extend(replace_list(&box_bytes[12..], list));
            out.extend(atom(&kind, &body));
        } else {
            out.extend(box_bytes);
        }
    }
    out
}
pub fn fixture(alac: bool, values: &[(&[u8; 4], Vec<&str>)]) -> Vec<u8> {
    let mut list = vec![];
    for (kind, strings) in values {
        list.extend(text(kind, strings));
    }
    list.extend(atom(b"zzzz", b"opaque unrecognized atom \0\xff"));
    const TAGGED: &[u8] = include_bytes!("../fixtures/media/tagged.m4a");
    list.extend(metadata_item(TAGGED, b"covr").remove(0));
    let base = if alac { ALAC } else { AAC };
    let top = children(base);
    assert_eq!(top.last().unwrap().0, *b"moov");
    replace_list(base, &list)
}
pub fn media(bytes: &[u8]) -> Vec<u8> {
    children(bytes)
        .into_iter()
        .filter(|(kind, _)| kind == b"mdat")
        .flat_map(|(_, data)| data[8..].iter().copied())
        .collect()
}
pub fn metadata_item(bytes: &[u8], key: &[u8; 4]) -> Vec<Vec<u8>> {
    children(&ilst(bytes)[8..])
        .into_iter()
        .filter(|(kind, _)| kind == key)
        .map(|(_, data)| data.to_vec())
        .collect()
}
pub fn fast_start(bytes: &[u8]) -> Vec<u8> {
    fn offsets(bytes: &[u8], delta: u32) -> Vec<u8> {
        let mut out = vec![];
        for (kind, data) in children(bytes) {
            if matches!(&kind, b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl") {
                out.extend(atom(&kind, &offsets(&data[8..], delta)));
            } else if &kind == b"stco" {
                let mut changed = data.to_vec();
                for word in changed[16..].chunks_exact_mut(4) {
                    let old = u32::from_be_bytes(word.try_into().unwrap());
                    word.copy_from_slice(&old.checked_add(delta).unwrap().to_be_bytes());
                }
                out.extend(changed);
            } else {
                out.extend(data);
            }
        }
        out
    }
    let top = children(bytes);
    let movie = top.last().unwrap().1;
    assert_eq!(&movie[4..8], b"moov");
    let mut out = top[0].1.to_vec();
    out.extend(offsets(movie, movie.len() as u32));
    for (_, data) in &top[1..top.len() - 1] {
        out.extend(*data);
    }
    out
}

pub fn replace_leaf(bytes: &[u8], key: &[u8; 4], replacement: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    for (kind, data) in children(bytes) {
        if &kind == key {
            out.extend(replacement);
        } else if matches!(
            &kind,
            b"moov" | b"trak" | b"mdia" | b"minf" | b"dinf" | b"stbl" | b"edts" | b"udta"
        ) {
            out.extend(atom(&kind, &replace_leaf(&data[8..], key, replacement)));
        } else {
            out.extend(data);
        }
    }
    out
}
