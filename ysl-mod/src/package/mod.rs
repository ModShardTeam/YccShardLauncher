pub mod name;
pub mod read;
pub mod write;

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error};

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum PackageRejection {
    NoMetaChunk,
    MultipleMetaChunkFound,
    NoEntryPoint { expected: String },

    // metadata related rejections
    InvalidDeserMetadata(toml::de::Error),
    InvalidSerMetadata(toml::ser::Error),

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
                .find(|&c| !(c.is_ascii() && name::is_valid_name_byte(c as u8)))
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
