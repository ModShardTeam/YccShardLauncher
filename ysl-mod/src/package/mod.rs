pub mod builder;
pub mod export;
pub mod import;
pub mod read;
pub mod validate;
pub mod write;

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::package;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ChunkRejection {
    NotEnoughDataHeader { needed: usize, available: usize },
    NotEnoughDataPayload { declared: u32, available: usize },
    TooBigChunk { declared: usize },
}

impl fmt::Display for ChunkRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkRejection::NotEnoughDataHeader { needed, available } => write!(
                f,
                "header is too small, it expects {needed} bytes but found {available} bytes"
            ),
            ChunkRejection::NotEnoughDataPayload {
                declared,
                available,
            } => write!(
                f,
                "payload is too small, it expects {declared} bytes but found {available} bytes"
            ),
            ChunkRejection::TooBigChunk { declared } => write!(
                f,
                "chunk declared with {declared} bytes, maximum size is {} bytes",
                package::MAXIMUM_CHUNK_SIZE
            ),
        }
    }
}

impl Error for ChunkRejection {}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum NameRejection {
    Empty,
    NameLenPastEnd { declared: usize, available: usize },
    TooLong { name: String, len: usize },
    Invalid { name: String, reason: String },
}

impl fmt::Display for NameRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameRejection::Empty => write!(f, "name is empty"),
            NameRejection::NameLenPastEnd {
                declared,
                available,
            } => write!(
                f,
                "name bigger than the rest of the chunk, declared {declared} bytes but only {available} available bytes"
            ),
            NameRejection::TooLong { name, len } => {
                write!(f, "name {name:.300} is {len} bytes, maximum is 255")
            }
            NameRejection::Invalid { name, reason } => write!(f, "invalid {name}: {reason}"),
        }
    }
}

impl Error for NameRejection {}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum RhaiRejection {
    Name(NameRejection),
    NullPayload,
    InvalidByteName { name: Vec<u8>, reason: String },
    InvalidScript { name: String, reason: String },
    DuplicateName { found: String },
}

impl fmt::Display for RhaiRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RhaiRejection::Name(name_rejection) => {
                write!(f, "invalid rhai name: {name_rejection}")
            }
            RhaiRejection::NullPayload => write!(f, "empty payload"),
            RhaiRejection::InvalidByteName { name, reason } => {
                write!(f, "script name {}: {reason}", name.escape_ascii())
            }
            RhaiRejection::InvalidScript { name, reason } => write!(f, "script {name}: {reason}"),
            RhaiRejection::DuplicateName { found } => {
                write!(f, "found multiple rhai scripts with the same name {found}")
            }
        }
    }
}

impl Error for RhaiRejection {}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum EntryRejection {
    Name(NameRejection),
    None { expected: String },
}

impl fmt::Display for EntryRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryRejection::Name(name_rejection) => {
                write!(f, "invalid entry name: {name_rejection}")
            }
            EntryRejection::None { expected } => {
                write!(f, "no entry name found, expected: {expected}")
            }
        }
    }
}

impl Error for EntryRejection {}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    InvalidFolder { folder: String, reason: String },
    NonEmptyFolder { folder: String },

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
    KnownButUnsupportedId { found: FourCC },

    // mod name related rejection
    ModName(NameRejection),

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

impl fmt::Display for PackageRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageRejection::InvalidFolder { folder, reason } => {
                write!(f, "invalid destination {folder}: {reason}")
            }
            PackageRejection::NonEmptyFolder { folder } => {
                write!(f, "folder {folder} is not empty")
            }
            PackageRejection::NotEnoughDataHeader { needed, available } => write!(
                f,
                "incomplete header, size is expected {needed} bytes but {available} bytes available"
            ),
            PackageRejection::BadMagic { found } => {
                write!(f, "bad magic found {}", found.escape_ascii())
            }
            PackageRejection::UnsupportedVersion { found } => {
                write!(
                    f,
                    "this mod uses format version {found}; this YSL supports {} to {}, update YSL",
                    package::OLDEST_SUPPORTED_VERSION,
                    package::CURRENT_FORMAT_VERSION
                )
            }
            PackageRejection::UnknownBits { found } => write!(
                f,
                "unknown flag {found:#06x}, currently supported is {:#06x}",
                package::KNOWN_MASK
            ),
            PackageRejection::InvalidSizeFile {
                declared,
                available,
            } => write!(
                f,
                "file declared with size {declared} bytes but found {available} bytes"
            ),
            PackageRejection::TooBigFile { declared } => {
                write!(
                    f,
                    "file declared with size {declared} bytes, exceeding maximum {} bytes",
                    package::MAXIMUM_FILE_SIZE
                )
            }
            PackageRejection::Chunk(chunk_rejection) => write!(f, "in chunk {}", chunk_rejection),
            PackageRejection::NoMetaChunk => write!(f, "no metadata chunk found"),
            PackageRejection::MultipleMetaChunkFound => write!(f, "multiple metadata chunk found"),
            PackageRejection::KnownButUnsupportedId { found } => {
                write!(f, "chunk {found} is known but currently unsupported")
            }
            PackageRejection::Entry(entry_rejection) => {
                write!(f, "invalid entry: {entry_rejection}")
            }
            PackageRejection::InvalidDeserMetadata(error) => {
                write!(f, "while deserializing: {error:#}")
            }
            PackageRejection::InvalidSerMetadata(error) => {
                write!(f, "while serializing: {error:#}")
            }
            PackageRejection::Rhai(rhai_rejection) => {
                write!(f, "invalid rhai script: {rhai_rejection}")
            }
            PackageRejection::ModName(name_rejection) => {
                write!(f, "invalid mod name: {name_rejection}")
            }
        }
    }
}

impl Error for PackageRejection {}

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

impl fmt::Display for FourCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FourCC::LuaScript => write!(f, "LUA"),
            FourCC::RhaiScript => write!(f, "RHAI"),
            FourCC::Metadata => write!(f, "META"),
            FourCC::Png => write!(f, "PNG"),
        }
    }
}

const YFF_MAGIC: &[u8; 4] = b"YFF\0";
const OLDEST_SUPPORTED_VERSION: u16 = 1;
const CURRENT_FORMAT_VERSION: u16 = 1;

const KNOWN_MASK: u16 = 0;

const MAXIMUM_FILE_SIZE: usize = 256 * 1_024; // 256 kiB
const MAXIMUM_CHUNK_SIZE: usize = 10 * 1_024; // 10 kiB

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

impl<'a> Chunk<'a> {
    pub fn size(&self) -> usize {
        self.four_cc.len() + self.payload.len()
    }
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Rhai,
    Lua,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub mod_name: String,
    pub version: semver::Version,
    pub engine: Engine,
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
