use std::collections::{BTreeMap, btree_map::Entry};

use serde::{Deserialize, Deserializer, Serialize, de::Error};

use crate::raw_parser;

const RESERVED_WINDOWS_DEVICE_NAME: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    NoMetaChunk,
    MultipleMetaChunkFound,
    NoEntryPoint { expected: String },

    // metadata related rejections
    InvalidMetadata(toml::de::Error),

    // rhai related rejections
    RhaiNullPayload,
    RhaiNullName,
    RhaiInvalidNameLen,
    RhaiInvalidByteName { name: Vec<u8>, reason: String },
    RhaiInvalidName { name: String, reason: String },
    RhaiInvalidScript { name: String, reason: String },
    RhaiDuplicateName { found: String },
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

fn is_valid_reader_char(b: u8) -> bool {
    matches!(b, b'-' | b'_' | b'a'..=b'z' | b'0'..=b'9')
}

fn entry_point_deser<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let o: Option<String> = Option::deserialize(deserializer)?;

    match o {
        Some(s) => {
            let lower_s = s.to_ascii_lowercase();
            if lower_s.is_empty() {
                return Err(D::Error::custom("empty entry point"));
            }
            if let Some(c) = lower_s
                .chars()
                .find(|&c| !(c.is_ascii() && is_valid_reader_char(c as u8)))
            {
                return Err(D::Error::custom(format!(
                    "Invalid character {c:?} found in entry_name"
                )));
            }
            Ok(Some(lower_s))
        }
        None => Ok(None),
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub mod_name: String,
    pub version: semver::Version,
    pub engine: String,
    #[serde(default, deserialize_with = "entry_point_deser")]
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

#[derive(Debug, PartialEq)]
pub struct Package {
    pub meta: Metadata,
    pub rhai_scripts: BTreeMap<String, String>,
}

impl Package {
    pub fn entry(&self) -> &str {
        self.meta.entry()
    }
}

fn rhai_decoder(payload: &[u8]) -> Result<(String, String), PackageRejection> {
    let Some((&name_len, rest)) = payload.split_first() else {
        return Err(PackageRejection::RhaiNullPayload);
    };

    if name_len == 0 {
        return Err(PackageRejection::RhaiNullName);
    }

    let Some((name_bytes, rest)) = rest.split_at_checked(name_len as usize) else {
        return Err(PackageRejection::RhaiInvalidNameLen);
    };

    let name: String = name_bytes
        .iter()
        .map(|&b| {
            if is_valid_reader_char(b) {
                Ok(b as char)
            } else {
                Err(PackageRejection::RhaiInvalidByteName {
                    name: name_bytes.to_vec(),
                    reason: format!("Invalid character {:?} ({b:#x}) found in name", b as char),
                })
            }
        })
        .collect::<Result<_, PackageRejection>>()?;

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

    let mut rhai_scripts: BTreeMap<String, String> = BTreeMap::new();
    for c in rhai_filter {
        let (name, data) = rhai_decoder(c.payload)?;
        match rhai_scripts.entry(name) {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(data);
            }
            Entry::Occupied(occupied_entry) => {
                return Err(PackageRejection::RhaiDuplicateName {
                    found: occupied_entry.key().to_owned(),
                });
            }
        }
    }

    let expected_entry_point = meta.entry();
    if !rhai_scripts.contains_key(expected_entry_point) {
        return Err(PackageRejection::NoEntryPoint {
            expected: expected_entry_point.to_string(),
        });
    }

    Ok(Package { meta, rhai_scripts })
}
