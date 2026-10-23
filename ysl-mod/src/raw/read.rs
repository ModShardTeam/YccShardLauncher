use crate::raw;
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

pub fn parse_header(bytes: &[u8]) -> Result<(raw::Header, &[u8]), raw::RawRejection> {
    let Some((header, rest)) = raw::Header::read(bytes) else {
        return Err(raw::RawRejection::NotEnoughDataHeader {
            needed: size_of::<raw::Header>(),
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

pub fn parse_chunk<'a>(bytes: &'a [u8]) -> Result<(raw::Chunk<'a>, &'a [u8]), raw::ChunkRejection> {
    let Some((&four_cc, rest)) = bytes.split_first_chunk() else {
        return Err(raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: bytes.len(),
        });
    };
    let Some((&len_bytes, rest)) = rest.split_first_chunk() else {
        return Err(raw::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: bytes.len(),
        });
    };

    let len: u32 = u32::from_le_bytes(len_bytes);
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

    Ok((raw::Chunk { four_cc, payload }, rest))
}

pub fn parse_file<'a>(bytes: &'a [u8]) -> Result<raw::File<'a>, raw::RawRejection> {
    if bytes.len() >= raw::MAXIMUM_FILE_SIZE {
        return Err(raw::RawRejection::TooBigFile {
            declared: bytes.len(),
        });
    }

    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let iter = ChunkIter::from(raw_chunks);

    Ok(raw::File {
        header: raw_header,
        chunks: iter.collect::<Result<_, _>>()?,
    })
}
