#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
    TooBigPayload { declared: usize },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum YffRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    BadMagic { found: [u8; 4] },
    UnsupportedVersion { found: u16 },
    UnknownBits { found: u16 },
    InvalidSizeFile { declared: u32, available: usize },
    TooBigFile { declared: usize },
    InvalidChunk(ChunkRejection),
}

impl From<ChunkRejection> for YffRejection {
    fn from(value: ChunkRejection) -> Self {
        YffRejection::InvalidChunk(value)
    }
}

const YFF_MAGIC: &[u8; 4] = b"YFF\0";
const OLDEST_SUPPORTED_VERSION: u16 = 1;
pub const CURRENT_FORMAT_VERSION: u16 = 1;

const KNOWN_MASK: u16 = 0;

const MAXIMUM_SIZE_YFF: usize = 1_024 * 1_024; // 1 MiB
const MAXIMUM_SIZE_PAYLOAD: usize = 10 * 1_024; // 10 kiB

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RawFileHeader {
    pub magic: [u8; 4],
    pub format_version: [u8; 2],
    pub flags: [u8; 2],
    pub len: [u8; 4],
}

const _: () = assert!(size_of::<RawFileHeader>() == 12);

impl RawFileHeader {
    fn read(bytes: &[u8]) -> Option<(RawFileHeader, &[u8])> {
        let (&magic, rest) = bytes.split_first_chunk()?;
        let (&format_version, rest) = rest.split_first_chunk()?;
        let (&flags, rest) = rest.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((
            RawFileHeader {
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
struct RawChunkHeader {
    four_cc: [u8; 4],
    len: [u8; 4],
}

const _: () = assert!(size_of::<RawChunkHeader>() == 8);

impl RawChunkHeader {
    fn read(bytes: &[u8]) -> Option<(RawChunkHeader, &[u8])> {
        let (&four_cc, rest) = bytes.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((RawChunkHeader { four_cc, len }, rest))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RawChunk<'a> {
    pub four_cc: [u8; 4],
    pub payload: &'a [u8],
}

pub struct RawChunkIter<'a> {
    state: &'a [u8],
}

impl<'a> From<&'a [u8]> for RawChunkIter<'a> {
    fn from(value: &'a [u8]) -> Self {
        RawChunkIter { state: value }
    }
}

impl<'a> Iterator for RawChunkIter<'a> {
    type Item = Result<RawChunk<'a>, ChunkRejection>;

    fn next(&mut self) -> Option<Result<RawChunk<'a>, ChunkRejection>> {
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

#[derive(Debug, PartialEq)]
pub struct RawFile<'a> {
    pub header: RawFileHeader,
    pub chunks: Vec<RawChunk<'a>>,
}

pub fn parse_header(bytes: &[u8]) -> Result<(RawFileHeader, &[u8]), YffRejection> {
    let Some((header, rest)) = RawFileHeader::read(bytes) else {
        return Err(YffRejection::NotEnoughDataHeader {
            needed: size_of::<RawFileHeader>(),
            available: bytes.len(),
        });
    };

    if header.magic != *YFF_MAGIC {
        return Err(YffRejection::BadMagic {
            found: header.magic,
        });
    }

    let version = u16::from_le_bytes(header.format_version);
    if !(OLDEST_SUPPORTED_VERSION..=CURRENT_FORMAT_VERSION).contains(&version) {
        return Err(YffRejection::UnsupportedVersion { found: version });
    }

    let flags = u16::from_le_bytes(header.flags);
    if (flags & !KNOWN_MASK) != 0 {
        return Err(YffRejection::UnknownBits { found: flags });
    }

    let len: u32 = u32::from_le_bytes(header.len);
    // exclude the first 12 bytes, RIFF-style
    if rest.len() != len as usize {
        return Err(YffRejection::InvalidSizeFile {
            declared: len,
            available: rest.len(),
        });
    }

    Ok((header, rest))
}

pub fn parse_chunk<'a>(bytes: &'a [u8]) -> Result<(RawChunk<'a>, &'a [u8]), ChunkRejection> {
    let Some((header, rest)) = RawChunkHeader::read(bytes) else {
        return Err(ChunkRejection::NotEnoughDataHeader {
            needed: size_of::<RawChunkHeader>(),
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
    if payload.len() > MAXIMUM_SIZE_PAYLOAD {
        return Err(ChunkRejection::TooBigPayload {
            declared: payload.len(),
        });
    }

    Ok((
        RawChunk {
            four_cc: header.four_cc,
            payload,
        },
        rest,
    ))
}

pub fn parse_file<'a>(bytes: &'a [u8]) -> Result<RawFile<'a>, YffRejection> {
    if bytes.len() >= MAXIMUM_SIZE_YFF {
        return Err(YffRejection::TooBigFile {
            declared: bytes.len(),
        });
    }

    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let iter = RawChunkIter::from(raw_chunks);

    Ok(RawFile {
        header: raw_header,
        chunks: iter.collect::<Result<_, _>>()?,
    })
}
