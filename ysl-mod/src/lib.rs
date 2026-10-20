#![forbid(unsafe_code)]

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum FourCC {
    LuaScript = u32::from_le_bytes(*b"LUA\0"),
    RhaiScript = u32::from_le_bytes(*b"RHAI"),
    Metadata = u32::from_le_bytes(*b"META"),
    Png = u32::from_le_bytes(*b"PNG\0"),
}

impl TryFrom<u32> for FourCC {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == FourCC::LuaScript as u32 => Ok(FourCC::LuaScript),
            x if x == FourCC::RhaiScript as u32 => Ok(FourCC::RhaiScript),
            x if x == FourCC::Metadata as u32 => Ok(FourCC::Metadata),
            x if x == FourCC::Png as u32 => Ok(FourCC::Png),
            _ => Err(()),
        }
    }
}

#[repr(C)]
pub struct RawFileHeader {
    pub magic: [u8; 4],
    pub len: [u8; 4],
    pub format_version: [u8; 2],
    pub flags: [u8; 2],
}

const _: () = assert!(size_of::<RawFileHeader>() == 12);

#[repr(C)]
struct RawChunkHeader {
    four_cc: [u8; 4],
    len: [u8; 4],
}

const _: () = assert!(size_of::<RawChunkHeader>() == 8);
const YFF_MAGIC: &[u8; 4] = b"YFF\0";

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

#[derive(Debug, PartialEq)]
pub enum YffRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    InvalidSizeFile { declared: u32, available: usize },
    InvalidChunk(ChunkRejection),
    InvalidFourCC { four_cc: [u8; 4] },
}

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

impl RawChunkHeader {
    fn read(bytes: &[u8]) -> Option<(RawChunkHeader, &[u8])> {
        let (&four_cc, rest) = bytes.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((RawChunkHeader { four_cc, len }, rest))
    }
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
    type Item = Chunk;

    fn next(&mut self) -> Option<Chunk> {
        let val = self.state;
        match parse_chunk(val) {
            Ok((rc, r)) => match Chunk::from_raw(rc) {
                Ok(c) => {
                    self.state = r;
                    Some(c)
                }
                Err(e) => {
                    self.stopping_reason = Some(e);
                    None
                }
            },
            Err(e) => {
                self.stopping_reason = Some(YffRejection::InvalidChunk(e));
                None
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FileHeader {
    pub len: u32,
    pub format_version: u16,
    pub flags: u16,
}

impl FileHeader {
    pub fn from_raw(raw: RawFileHeader) -> FileHeader {
        let len: u32 = u32::from_le_bytes(raw.len);
        let format_version: u16 = u16::from_le_bytes(raw.format_version);
        let flags: u16 = u16::from_le_bytes(raw.flags);

        FileHeader {
            len,
            format_version,
            flags,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Chunk {
    pub four_cc: FourCC,
    pub payload: Vec<u8>,
}

impl Chunk {
    pub fn from_raw(raw: RawChunk) -> Result<Chunk, YffRejection> {
        let Ok(four_cc) = u32::from_le_bytes(raw.four_cc).try_into() else {
            return Err(YffRejection::InvalidFourCC {
                four_cc: raw.four_cc,
            });
        };
        Ok(Chunk {
            four_cc,
            payload: raw.payload.to_vec(),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct File {
    pub header: FileHeader,
    pub chunks: Vec<Chunk>,
}

pub fn parse_file(bytes: &[u8]) -> Result<File, YffRejection> {
    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let header = FileHeader::from_raw(raw_header);
    let iter = RawChunkIter::from(raw_chunks);

    let mut chunks = vec![];
    for chunk in iter.fuse() {
        chunks.push(chunk)
    }
    Ok(File { header, chunks })
}
