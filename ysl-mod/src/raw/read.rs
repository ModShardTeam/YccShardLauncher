use crate::raw;
#[repr(C)]
#[derive(Debug, PartialEq)]
struct Header {
    magic: [u8; 4],
    format_version: [u8; 2],
    flags: [u8; 2],
    len: [u8; 4],
}

const _: () = assert!(size_of::<Header>() == 12);

impl Header {
    fn read(bytes: &[u8]) -> Option<(Header, &[u8])> {
        let (&magic, rest) = bytes.split_first_chunk()?;
        let (&format_version, rest) = rest.split_first_chunk()?;
        let (&flags, rest) = rest.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((
            Header {
                magic,
                len,
                format_version,
                flags,
            },
            rest,
        ))
    }
}

#[repr(C)]
#[derive(Debug, PartialEq)]
struct ChunkHeader {
    four_cc: [u8; 4],
    len: [u8; 4],
}

pub struct ChunkIter<'a> {
    state: &'a [u8],
}

impl<'a> From<&'a [u8]> for ChunkIter<'a> {
    fn from(value: &'a [u8]) -> Self {
        ChunkIter { state: value }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<raw::Chunk<'a>, raw::ChunkRejection>;

    fn next(&mut self) -> Option<Result<raw::Chunk<'a>, raw::ChunkRejection>> {
        let val = self.state;
        if val.is_empty() {
            return None;
        }

        match parse_chunk(val) {
            Ok((rc, r)) => {
                self.state = r;
                Some(Ok(rc))
            }

            Err(e) => {
                self.state = &[];
                Some(Err(e))
            }
        }
    }
}

fn parse_header(bytes: &[u8]) -> Result<(Header, &[u8]), raw::RawRejection> {
    let Some((header, rest)) = Header::read(bytes) else {
        return Err(raw::RawRejection::NotEnoughDataHeader {
            needed: size_of::<Header>(),
            available: bytes.len(),
        });
    };

    if header.magic != *raw::YFF_MAGIC {
        return Err(raw::RawRejection::BadMagic {
            found: header.magic,
        });
    }

    let version = u16::from_le_bytes(header.format_version);
    if !(raw::OLDEST_SUPPORTED_VERSION..=raw::CURRENT_FORMAT_VERSION).contains(&version) {
        return Err(raw::RawRejection::UnsupportedVersion { found: version });
    }

    let flags = u16::from_le_bytes(header.flags);
    if (flags & !raw::KNOWN_MASK) != 0 {
        return Err(raw::RawRejection::UnknownBits { found: flags });
    }

    let len: u32 = u32::from_le_bytes(header.len);
    // exclude the first 12 bytes, RIFF-style
    if rest.len() != len as usize {
        return Err(raw::RawRejection::InvalidSizeFile {
            declared: len,
            available: rest.len(),
        });
    }

    Ok((header, rest))
}

fn parse_chunk_header(bytes: &[u8]) -> Result<(ChunkHeader, &[u8]), raw::ChunkRejection> {
    let Some((&four_cc, rest)) = bytes.split_first_chunk() else {
        return Err(raw::ChunkRejection::NotEnoughDataHeader {
            needed: size_of::<ChunkHeader>(),
            available: bytes.len(),
        });
    };
    let Some((&len, rest)) = rest.split_first_chunk() else {
        return Err(raw::ChunkRejection::NotEnoughDataHeader {
            needed: size_of::<ChunkHeader>(),
            available: bytes.len(),
        });
    };

    Ok((ChunkHeader { four_cc, len }, rest))
}

pub fn parse_chunk<'a>(bytes: &'a [u8]) -> Result<(raw::Chunk<'a>, &'a [u8]), raw::ChunkRejection> {
    let (chunk_header, rest) = parse_chunk_header(bytes)?;

    let len: u32 = u32::from_le_bytes(chunk_header.len);
    let Some((payload, rest)) = rest.split_at_checked(len as usize) else {
        return Err(raw::ChunkRejection::NotEnoughDataPayload {
            declared: len,
            available: rest.len(),
        });
    };
    if payload.len() > raw::MAXIMUM_PAYLOAD_SIZE {
        return Err(raw::ChunkRejection::TooBigPayload {
            declared: payload.len(),
        });
    }

    Ok((
        raw::Chunk {
            four_cc: chunk_header.four_cc,
            payload,
        },
        rest,
    ))
}

pub fn parse_file<'a>(bytes: &'a [u8]) -> Result<raw::File<'a>, raw::RawRejection> {
    if bytes.len() >= raw::MAXIMUM_FILE_SIZE {
        return Err(raw::RawRejection::TooBigFile {
            declared: bytes.len(),
        });
    }

    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let format_version: u16 = u16::from_le_bytes(raw_header.format_version);
    let flags: u16 = u16::from_le_bytes(raw_header.flags);

    let iter = ChunkIter::from(raw_chunks);

    Ok(raw::File {
        format_version,
        flags,
        chunks: iter.collect::<Result<_, _>>()?,
    })
}
