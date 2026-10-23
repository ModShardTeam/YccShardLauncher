pub mod read;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
    TooBigPayload { declared: usize },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum RawRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    BadMagic { found: [u8; 4] },
    UnsupportedVersion { found: u16 },
    UnknownBits { found: u16 },
    InvalidSizeFile { declared: u32, available: usize },
    TooBigFile { declared: usize },
    InvalidChunk(ChunkRejection),
}

impl From<ChunkRejection> for RawRejection {
    fn from(value: ChunkRejection) -> Self {
        RawRejection::InvalidChunk(value)
    }
}

const YFF_MAGIC: &[u8; 4] = b"YFF\0";
const OLDEST_SUPPORTED_VERSION: u16 = 1;
pub const CURRENT_FORMAT_VERSION: u16 = 1;

const KNOWN_MASK: u16 = 0;

const MAXIMUM_FILE_SIZE: usize = 1_024 * 1_024; // 1 MiB
const MAXIMUM_PAYLOAD_SIZE: usize = 10 * 1_024; // 10 kiB

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Header {
    pub magic: [u8; 4],
    pub format_version: [u8; 2],
    pub flags: [u8; 2],
    pub len: [u8; 4],
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Chunk<'a> {
    pub four_cc: [u8; 4],
    pub payload: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub struct File<'a> {
    pub header: Header,
    pub chunks: Vec<Chunk<'a>>,
}
