pub mod read;
pub mod write;

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RChunk<'a> {
    pub four_cc: [u8; 4],
    pub payload: &'a [u8],
}

#[derive(Debug, PartialEq, Clone)]
pub struct WChunk {
    pub four_cc: [u8; 4],
    pub payload: Vec<u8>,
}

impl<'a> From<&'a WChunk> for RChunk<'a> {
    fn from(value: &'a WChunk) -> Self {
        RChunk {
            four_cc: value.four_cc,
            payload: &value.payload,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RFile<'a> {
    pub format_version: u16,
    pub flags: u16,
    pub chunks: Vec<RChunk<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct WFile {
    pub format_version: u16,
    pub flags: u16,
    pub chunks: Vec<WChunk>,
}

impl<'a> From<&'a WFile> for RFile<'a> {
    fn from(value: &'a WFile) -> Self {
        RFile {
            format_version: value.format_version,
            flags: value.flags,
            chunks: value.chunks.iter().map(|c| c.into()).collect::<Vec<_>>(),
        }
    }
}
