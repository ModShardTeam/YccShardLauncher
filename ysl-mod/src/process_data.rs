use serde::{Deserialize, Serialize};

use crate::raw_parser;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    InvalidFourCC { four_cc: [u8; 4] },
    NoMetaChunk,
    MultipleMetaChunkFound,
    FailedDecodedChunk(String),
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
    pub fn from_raw(raw: raw_parser::RawChunk) -> Result<Chunk, PackageRejection> {
        let Ok(four_cc) = raw.four_cc.try_into() else {
            return Err(PackageRejection::InvalidFourCC {
                four_cc: raw.four_cc,
            });
        };
        Ok(Chunk {
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
    pub fn from_raw(raw: raw_parser::RawFile) -> Result<File, PackageRejection> {
        let header = FileHeader::from_raw(raw.header);

        Ok(File {
            header,
            chunks: raw
                .chunks
                .iter()
                .filter_map(|&rc| Chunk::from_raw(rc).ok())
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Metadata {
    pub mod_name: String,
    pub version: String,
    pub target_version: String,
    pub author_name: String,
    pub short_desc: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct Package {
    pub meta: Metadata,
}

enum SearchU<T> {
    None,
    Unique(T),
    Multiple,
}

pub fn process_package(file: File) -> Result<Package, PackageRejection> {
    let meta_chunk = match file
        .chunks
        .iter()
        .filter(|&c| c.four_cc == FourCC::Metadata)
        .fold(SearchU::None, |status, x| match status {
            SearchU::None => SearchU::Unique(x),
            _ => SearchU::Multiple,
        }) {
        SearchU::Unique(x) => x,
        SearchU::None => {
            return Err(PackageRejection::NoMetaChunk);
        }
        SearchU::Multiple => {
            return Err(PackageRejection::MultipleMetaChunkFound);
        }
    };

    match toml::from_slice(meta_chunk.payload) {
        Err(e) => Err(PackageRejection::FailedDecodedChunk(format!("{e:#}"))),

        Ok(meta) => Ok(Package { meta }),
    }
}
