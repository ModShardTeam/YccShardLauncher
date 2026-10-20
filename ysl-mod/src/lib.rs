#![forbid(unsafe_code)]
pub mod raw_parser;

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
    pub fn from_raw(raw: raw_parser::RawChunk) -> Result<Chunk, raw_parser::YffRejection> {
        let Ok(four_cc) = u32::from_le_bytes(raw.four_cc).try_into() else {
            return Err(raw_parser::YffRejection::InvalidFourCC {
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
