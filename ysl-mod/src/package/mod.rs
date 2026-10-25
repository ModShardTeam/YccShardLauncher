pub mod read;
pub mod validate;
pub mod write;

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error};

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
    TooBigPayload { declared: usize },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum NameRejection {
    Null,
    InvalidLen,
    Invalid { name: String, reason: String },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum RhaiRejection {
    Name(NameRejection),
    NullPayload,
    InvalidByteName { name: Vec<u8>, reason: String },
    InvalidScript { name: String, reason: String },
    DuplicateName { found: String },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum EntryRejection {
    Name(NameRejection),
    None { expected: String },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    // raw data related rejections
    NotEnoughDataHeader { needed: usize, available: usize },
    BadMagic { found: [u8; 4] },
    UnsupportedVersion { found: u16 },
    UnknownBits { found: u16 },
    InvalidSizeFile { declared: u32, available: usize },
    TooBigFile { declared: usize },

    Chunk(ChunkRejection),

    NoMetaChunk,
    MultipleMetaChunkFound,

    // entry point related rejection
    Entry(EntryRejection),

    // metadata related rejections
    InvalidDeserMetadata(toml::de::Error),
    InvalidSerMetadata(toml::ser::Error),

    // rhai related rejections
    Rhai(RhaiRejection),
}

impl From<ChunkRejection> for PackageRejection {
    fn from(value: ChunkRejection) -> Self {
        PackageRejection::Chunk(value)
    }
}

impl From<RhaiRejection> for PackageRejection {
    fn from(value: RhaiRejection) -> Self {
        PackageRejection::Rhai(value)
    }
}

impl From<EntryRejection> for PackageRejection {
    fn from(value: EntryRejection) -> Self {
        PackageRejection::Entry(value)
    }
}

#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FourCC {
    LuaScript,
    RhaiScript,
    Metadata,
    Png,
}

impl TryFrom<[u8; 4]> for FourCC {
    type Error = [u8; 4];

    fn try_from(v: [u8; 4]) -> Result<Self, [u8; 4]> {
        match &v {
            b"LUA\0" => Ok(FourCC::LuaScript),
            b"RHAI" => Ok(FourCC::RhaiScript),
            b"META" => Ok(FourCC::Metadata),
            b"PNG\0" => Ok(FourCC::Png),
            _ => Err(v),
        }
    }
}

impl From<FourCC> for [u8; 4] {
    fn from(value: FourCC) -> Self {
        *match value {
            FourCC::LuaScript => b"LUA\0",
            FourCC::RhaiScript => b"RHAI",
            FourCC::Metadata => b"META",
            FourCC::Png => b"PNG\0",
        }
    }
}

const YFF_MAGIC: &[u8; 4] = b"YFF\0";
const OLDEST_SUPPORTED_VERSION: u16 = 1;
const CURRENT_FORMAT_VERSION: u16 = 1;

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

#[derive(Debug, PartialEq, Clone)]
pub struct ChunkBuf {
    pub four_cc: [u8; 4],
    pub payload: Vec<u8>,
}

impl<'a> From<&'a ChunkBuf> for Chunk<'a> {
    fn from(value: &'a ChunkBuf) -> Self {
        Chunk {
            four_cc: value.four_cc,
            payload: &value.payload,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub mod_name: String,
    pub version: semver::Version,
    pub engine: String,
    #[serde(default)]
    pub entry_point: Option<String>,
    pub game_version: String,
    pub author_name: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, semver::VersionReq>,
}

impl Metadata {
    pub fn entry(&self) -> &str {
        self.entry_point.as_deref().unwrap_or("mod")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Package {
    pub meta: Metadata,
    pub rhai_scripts: BTreeMap<String, String>,
}

impl Package {
    pub fn entry(&self) -> &str {
        self.meta.entry()
    }
}
