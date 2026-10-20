#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
}

#[derive(Debug, PartialEq)]
pub enum YffRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    InvalidSizeFile { declared: u32, available: usize },
    InvalidChunk(ChunkRejection),
    InvalidFourCC { four_cc: [u8; 4] },
}

const YFF_MAGIC: &[u8; 4] = b"YFF\0";

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RawFileHeader {
    pub magic: [u8; 4],
    pub len: [u8; 4],
    pub format_version: [u8; 2],
    pub flags: [u8; 2],
}

const _: () = assert!(size_of::<RawFileHeader>() == 12);

impl RawFileHeader {
    fn read(bytes: &[u8]) -> Option<(RawFileHeader, &[u8])> {
        let (&magic, rest) = bytes.split_first_chunk()?;

        if magic != *YFF_MAGIC {
            return None;
        }

        let (&len, rest) = rest.split_first_chunk()?;
        let (&format_version, rest) = rest.split_first_chunk()?;
        let (&flags, rest) = rest.split_first_chunk()?;
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

struct RawChunkIter<'a> {
    state: &'a [u8],
    stopping_reason: Option<YffRejection>,
}

impl<'a> From<&'a [u8]> for RawChunkIter<'a> {
    fn from(value: &'a [u8]) -> Self {
        RawChunkIter {
            state: value,
            stopping_reason: None,
        }
    }
}

impl<'a> Iterator for RawChunkIter<'a> {
    type Item = RawChunk<'a>;

    fn next(&mut self) -> Option<RawChunk<'a>> {
        let val = self.state;
        match parse_chunk(val) {
            Ok((rc, r)) => {
                self.state = r;
                Some(rc)
            }

            Err(e) => {
                self.stopping_reason = Some(YffRejection::InvalidChunk(e));
                None
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

    let len: u32 = u32::from_le_bytes(header.len);
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

    Ok((
        RawChunk {
            four_cc: header.four_cc,
            payload,
        },
        rest,
    ))
}

pub fn parse_file<'a>(bytes: &'a [u8]) -> Result<RawFile<'a>, YffRejection> {
    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let iter = RawChunkIter::from(raw_chunks);

    let mut raw_chunks = vec![];
    for raw_chunk in iter.fuse() {
        raw_chunks.push(raw_chunk)
    }
    Ok(RawFile {
        header: raw_header,
        chunks: raw_chunks,
    })
}
