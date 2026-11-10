pub const AUDIO: &[u8] = include_bytes!("../fixtures/media/blank.wv");
pub fn item(key: &str, flags: u32, value: &[u8]) -> Vec<u8> {
    let mut bytes = (value.len() as u32).to_le_bytes().to_vec();
    bytes.extend(flags.to_le_bytes());
    bytes.extend(key.as_bytes());
    bytes.push(0);
    bytes.extend(value);
    bytes
}
pub fn fixture(header: bool, multi: bool) -> Vec<u8> {
    let mut items = Vec::new();
    for key in ["Artist", "Album Artist", "Title", "Album", "Genre"] {
        items.extend(item(
            key,
            0,
            if multi {
                "à\0á".as_bytes()
            } else {
                "à".as_bytes()
            },
        ));
    }
    items.extend(item(
        "Cover Art (Front)",
        2,
        b"cover.png\0exact artwork bytes\xff",
    ));
    items.extend(item("Unknown Binary", 2, b"opaque\0\xff"));
    with_items(&items, 7, header)
}
pub fn with_items(items: &[u8], count: u32, header: bool) -> Vec<u8> {
    let mut footer = b"APETAGEX".to_vec();
    footer.extend(2000u32.to_le_bytes());
    footer.extend((items.len() as u32 + 32).to_le_bytes());
    footer.extend(count.to_le_bytes());
    footer.extend(if header { 0x80000000u32 } else { 0 }.to_le_bytes());
    footer.extend([0; 8]);
    let mut bytes = AUDIO.to_vec();
    if header {
        let mut h = footer.clone();
        h[20..24].copy_from_slice(&0xa0000000u32.to_le_bytes());
        bytes.extend(h);
    }
    bytes.extend(items);
    bytes.extend(footer);
    bytes
}
