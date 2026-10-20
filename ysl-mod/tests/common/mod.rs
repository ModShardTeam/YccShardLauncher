#![allow(dead_code)]
pub const ID_META: &[u8; 4] = b"META";
pub const ID_RHAI: &[u8; 4] = b"RHAI";

pub fn create_chunk(id: &[u8; 4], len: u32, payload: &[u8]) -> Vec<u8> {
    let mut array: Vec<u8> = vec![];
    array.extend_from_slice(id);
    array.extend_from_slice(&len.to_le_bytes());
    array.extend_from_slice(payload);
    array
}

pub fn create_header(magic: &[u8; 4], format_version: u16, flags: u16, len: u32) -> Vec<u8> {
    let mut array: Vec<u8> = vec![];
    array.extend_from_slice(magic);
    array.extend_from_slice(&format_version.to_le_bytes());
    array.extend_from_slice(&flags.to_le_bytes());
    array.extend_from_slice(&len.to_le_bytes());
    array
}

pub fn sample_body() -> Vec<u8> {
    let mut body: Vec<u8> = vec![];
    body.extend_from_slice(&create_chunk(ID_META, 3, &[0xA0, 0xA1, 0xA2]));
    body.extend_from_slice(&create_chunk(ID_RHAI, 4, &[0xB0, 0xB1, 0xB2, 0xB3]));
    body.extend_from_slice(&create_chunk(ID_RHAI, 4, &[0xC0, 0xC1, 0xC2, 0xC3]));
    body.extend_from_slice(&create_chunk(ID_RHAI, 4, &[0xD0, 0xD1, 0xD2, 0xD3]));
    body.extend_from_slice(&create_chunk(ID_RHAI, 4, &[0x00, 0x01, 0x02, 0x03]));
    body
}
