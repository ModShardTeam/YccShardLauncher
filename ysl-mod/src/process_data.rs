use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::raw_parser;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    NoMetaChunk,
    MultipleMetaChunkFound,
    InvalidMetadata(toml::de::Error),
    InvalidRhai(String),
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
    pub game_version: String,
    pub author_name: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, semver::VersionReq>,
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub meta: Metadata,
    pub rhai_scripts: Vec<String>,
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

    let rhai_decoder = rhai_filter
        .filter_map(|c: &Chunk<'a>| {
            // TODO: do not silence zstd errors
            if file.header.flags & raw_parser::ZSTD != 0 {
                zstd::stream::decode_all(c.payload).ok()
            } else {
                Some(c.payload.to_vec())
            }
        })
        .map(|stream| String::from_utf8(stream));

    let Some(meta_chunk) = metadata_filter.next() else {
        return Err(PackageRejection::NoMetaChunk);
    };
    if metadata_filter.next().is_some() {
        return Err(PackageRejection::MultipleMetaChunkFound);
    }
    let meta = match toml::from_slice(meta_chunk.payload) {
        Err(e) => return Err(PackageRejection::InvalidMetadata(e)),
        Ok(meta) => meta,
    };

    Ok(Package {
        meta,
        rhai_scripts: rhai_decoder
            .collect::<Result<_, _>>()
            .map_err(|e| PackageRejection::InvalidRhai(format!("{e:#}")))?,
    })
}
