#![forbid(unsafe_code)]

enum FourCC {
    LuaScript,
    RhaiScript,
    Metadata,
    Png,
}

#[repr(C)]
struct ChunkHeader {
    four_cc: [u8; 4],
    len: [u8; 4],
}

const _: () = assert!(size_of::<ChunkHeader>() == 8);

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RawChunk<'a> {
    pub four_cc: [u8; 4],
    pub payload: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
}

impl ChunkHeader {
    fn read(bytes: &[u8]) -> Option<(ChunkHeader, &[u8])> {
        let (&four_cc, rest) = bytes.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((ChunkHeader { four_cc, len }, rest))
    }
}

pub fn parse_to_chunk<'a>(bytes: &'a [u8]) -> Result<(RawChunk<'a>, &'a [u8]), ChunkRejection> {
    let Some((header, rest)) = ChunkHeader::read(bytes) else {
        return Err(ChunkRejection::NotEnoughDataHeader {
            needed: size_of::<ChunkHeader>(),
            available: bytes.len(),
        });
    };

    let len: u32 = u32::from_le_bytes(header.len);
    let Some((payload, rest)) = rest.split_at_checked(len as usize) else {
        return Err(ChunkRejection::NotEnoughDataPayload {
            declared: len,
            available: rest.len(),
        });
    };

    Ok((
        RawChunk {
            four_cc: header.four_cc,
            payload,
        },
        rest,
    ))
}
