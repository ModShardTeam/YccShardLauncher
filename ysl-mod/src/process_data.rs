use std::collections::BTreeMap;

use rhai::Engine;
use serde::{Deserialize, Serialize};

use crate::raw_parser;

const CAPITALA: u8 = b'A';
const CAPITALZ: u8 = b'Z';
const LOWERA: u8 = b'a';
const LOWERZ: u8 = b'z';
const DIGIT0: u8 = b'0';
const DIGIT9: u8 = b'9';
const HYPHEN: u8 = b'-';
const UNDERSCORE: u8 = b'_';
const RESERVED_WINDOWS_DEVICE_NAME: [&str; 28] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
    "LPT9", "LPT¹", "LPT²", "LPT³",
];

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    NoMetaChunk,
    MultipleMetaChunkFound,
    NoEntryPoint,

    // metadata related rejections
    InvalidMetadata(toml::de::Error),

    // rhai related rejections
    RhaiNullPayload,
    RhaiInvalidNameLen,
    RhaiInvalidByteName { name: Vec<u8>, reason: String },
    RhaiInvalidName { name: String, reason: String },
    RhaiInvalidScript { name: String, reason: String },
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct FileHeader {
    pub format_version: u16,
    pub flags: u16,
}

impl FileHeader {
    pub fn from_raw(raw: raw_parser::RawFileHeader) -> FileHeader {
        let format_version: u16 = u16::from_le_bytes(raw.format_version);
        let flags: u16 = u16::from_le_bytes(raw.flags);

        FileHeader {
            format_version,
            flags,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Chunk<'a> {
    pub four_cc: FourCC,
    pub payload: &'a [u8],
}

impl<'a> Chunk<'a> {
    pub fn from_raw(raw: raw_parser::RawChunk<'a>) -> Option<Chunk<'a>> {
        let Ok(four_cc) = raw.four_cc.try_into() else {
            return None;
        };

        Some(Chunk {
            four_cc,
            payload: raw.payload,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct File<'a> {
    pub header: FileHeader,
    pub chunks: Vec<Chunk<'a>>,
}

impl<'a> File<'a> {
    pub fn from_raw(raw: raw_parser::RawFile<'a>) -> File<'a> {
        let header = FileHeader::from_raw(raw.header);

        File {
            header,
            chunks: raw
                .chunks
                .iter()
                .filter_map(|&rc| Chunk::from_raw(rc))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub mod_name: String,
    pub version: semver::Version,
    pub engine: String,
    pub entry_point: Option<String>,
    pub game_version: String,
    pub author_name: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, semver::VersionReq>,
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub meta: Metadata,
    pub rhai_scripts: BTreeMap<String, String>,
}

fn rhai_decoder(payload: &[u8]) -> Result<(String, String), PackageRejection> {
    let Some((&name_len, rest)) = payload.split_first() else {
        return Err(PackageRejection::RhaiNullPayload);
    };

    let Some((name_bytes, rest)) = rest.split_at_checked(name_len as usize) else {
        return Err(PackageRejection::RhaiInvalidNameLen);
    };

    if let Some(v) = name_bytes
        .iter()
        .filter_map(|s| match s {
            CAPITALA..=CAPITALZ | LOWERA..=LOWERZ | DIGIT0..=DIGIT9 | &HYPHEN | &UNDERSCORE => None,
            v => Some(v),
        })
        .next()
    {
        return Err(PackageRejection::RhaiInvalidByteName {
            name: name_bytes.to_vec(),
            reason: format!("Invalid character {:?} ({v:#x}) found in name", *v as char),
        });
    }

    let name = str::from_utf8(name_bytes).map_err(|e| PackageRejection::RhaiInvalidByteName {
        name: name_bytes.to_vec(),
        reason: format!("{e:#}"),
    })?;

    let uppercase = &name.to_ascii_uppercase() as &str;
    if RESERVED_WINDOWS_DEVICE_NAME.contains(&uppercase) {
        return Err(PackageRejection::RhaiInvalidName {
            name: name.to_string(),
            reason: "Reserved device name".to_string(),
        });
    }

    let data =
        String::from_utf8(rest.to_vec()).map_err(|e| PackageRejection::RhaiInvalidScript {
            name: name.to_string(),
            reason: format!("{e:#}"),
        })?;

    let engine = Engine::new();
    engine
        .compile(&data)
        .map_err(|e| PackageRejection::RhaiInvalidScript {
            name: name.to_string(),
            reason: format!("{e:#}"),
        })?;
    Ok((name.to_string(), data))
}

pub fn process_package<'a>(file: File<'a>) -> Result<Package, PackageRejection> {
    let mut metadata_filter = file
        .chunks
        .iter()
        .filter(|&c| c.four_cc == FourCC::Metadata);

    let rhai_filter = file
        .chunks
        .iter()
        .filter(|&c| c.four_cc == FourCC::RhaiScript);

    let Some(meta_chunk) = metadata_filter.next() else {
        return Err(PackageRejection::NoMetaChunk);
    };
    if metadata_filter.next().is_some() {
        return Err(PackageRejection::MultipleMetaChunkFound);
    }
    let meta: Metadata = match toml::from_slice(meta_chunk.payload) {
        Err(e) => return Err(PackageRejection::InvalidMetadata(e)),
        Ok(meta) => meta,
    };

    let rhai_scripts: BTreeMap<String, String> = rhai_filter
        .map(|c| rhai_decoder(c.payload))
        .collect::<Result<_, _>>()?;

    if !rhai_scripts.iter().any(|(n, _)| n == "mod") && meta.entry_point.is_none() {
        return Err(PackageRejection::NoEntryPoint);
    }

    Ok(Package { meta, rhai_scripts })
}
