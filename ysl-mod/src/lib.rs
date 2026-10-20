#![forbid(unsafe_code)]
pub mod raw_parser;

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    InvalidFourCC { four_cc: [u8; 4] },
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
    pub len: u32,
    pub format_version: u16,
    pub flags: u16,
}

impl FileHeader {
    pub fn from_raw(raw: raw_parser::RawFileHeader) -> FileHeader {
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
    pub fn from_raw(raw: raw_parser::RawChunk) -> Result<Chunk, PackageRejection> {
        let Ok(four_cc) = raw.four_cc.try_into() else {
            return Err(PackageRejection::InvalidFourCC {
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
