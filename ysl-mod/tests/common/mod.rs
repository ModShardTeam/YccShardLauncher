#![allow(dead_code)]

use std::collections::BTreeMap;

use ysl_mod::package;
pub const ID_META: &[u8; 4] = b"META";
pub const ID_RHAI: &[u8; 4] = b"RHAI";

pub fn dummy_metadata(entry: &str, version: &str, req: &str) -> String {
    format!(
        r#"mod_name = "test_mod"
version = {version:?}
engine = "rhai"
entry_point = {entry:?}
game_version = "1.0.0"
author_name = "test_author"
short_desc = "this is a test"

[dependencies]
other_test = {req:?}
yet_another_test = ">0.2"

"#
    )
}

pub fn dummy_no_entry_metadata(version: &str, req: &str) -> String {
    format!(
        r#"mod_name = "test_mod"
version = {version:?}
engine = "rhai"
game_version = "1.0.0"
author_name = "test_author"
short_desc = "this is a test"

[dependencies]
other_test = {req:?}
yet_another_test = ">0.2"

"#
    )
}

pub fn dummy_rhai_payload(name: &[u8]) -> Vec<u8> {
    let mut payload: Vec<u8> = vec![];
    payload.push(name.len() as u8);
    payload.extend_from_slice(name);
    payload.extend_from_slice("fn load(mod) { }".as_bytes());
    payload
}

pub struct PackageBuilder {
    mod_name: String,
    version: semver::Version,
    engine: String,
    entry_point: Option<String>,
    game_version: String,
    author_name: String,
    short_desc: Option<String>,
    dependencies: BTreeMap<String, semver::VersionReq>,
    rhai_scripts: BTreeMap<String, String>,
}

impl PackageBuilder {
    pub fn new() -> Self {
        PackageBuilder {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: None,
            game_version: "1.0.0".to_string(),
            author_name: "test_author".to_string(),
            short_desc: Some("this is a test".to_string()),
            dependencies: BTreeMap::new(),
            rhai_scripts: BTreeMap::from([("mod".to_string(), "fn load(mod) { }".to_string())]),
        }
    }

    pub fn mod_name(&mut self, name: &str) -> &mut Self {
        self.mod_name = name.to_string();
        self
    }

    pub fn version(&mut self, ver: &str) -> &mut Self {
        self.version = semver::Version::parse(ver).expect("valid operation");
        self
    }

    pub fn engine(&mut self, engine: &str) -> &mut Self {
        self.engine = engine.to_string();
        self
    }

    pub fn clear_entry_point(&mut self) -> &mut Self {
        self.entry_point = None;
        self
    }

    pub fn entry_point(&mut self, entry: &str) -> &mut Self {
        self.entry_point = Some(entry.to_string());
        self
    }

    pub fn game_version(&mut self, ver: &str) -> &mut Self {
        self.game_version = ver.to_string();
        self
    }

    pub fn author_name(&mut self, name: &str) -> &mut Self {
        self.author_name = name.to_string();
        self
    }

    pub fn clear_short_desc(&mut self) -> &mut Self {
        self.short_desc = None;
        self
    }

    pub fn short_desc(&mut self, desc: &str) -> &mut Self {
        self.short_desc = Some(desc.to_string());
        self
    }

    pub fn clear_dependencies(&mut self) -> &mut Self {
        self.dependencies.clear();
        self
    }

    pub fn add_dependencies(&mut self, name: &str, req: &str) -> &mut Self {
        self.dependencies.insert(
            name.to_string(),
            semver::VersionReq::parse(req).expect("valid operation"),
        );
        self
    }

    pub fn clear_rhai(&mut self) -> &mut Self {
        self.rhai_scripts.clear();
        self
    }

    pub fn add_default_rhai(&mut self, name: &str) -> &mut Self {
        self.rhai_scripts
            .insert(name.to_string(), "fn load(mod) { }".to_string());
        self
    }

    pub fn add_rhai(&mut self, name: &str, data: &str) -> &mut Self {
        self.rhai_scripts.insert(name.to_string(), data.to_string());
        self
    }

    pub fn build(&self) -> package::Package {
        package::Package {
            meta: package::Metadata {
                mod_name: self.mod_name.clone(),
                version: self.version.clone(),
                engine: self.engine.clone(),
                entry_point: self.entry_point.clone(),
                game_version: self.game_version.clone(),
                author_name: self.author_name.clone(),
                short_desc: self.short_desc.clone(),
                dependencies: self.dependencies.clone(),
            },
            rhai_scripts: self.rhai_scripts.clone(),
        }
    }
}

enum ChunkLen {
    None,
    Computed,
    Set(Vec<u8>),
}

pub struct ChunkBuilder {
    id: Vec<u8>,
    len: ChunkLen,
    payload: Vec<u8>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        ChunkBuilder {
            id: vec![],
            len: ChunkLen::Computed,
            payload: vec![],
        }
    }

    pub fn id(&mut self, id: &[u8]) -> &mut Self {
        self.id = id.to_vec();
        self
    }

    pub fn len(&mut self, len: u32) -> &mut Self {
        self.len = ChunkLen::Set(len.to_le_bytes().to_vec());
        self
    }

    pub fn arbitrary_len(&mut self, len: &[u8]) -> &mut Self {
        self.len = ChunkLen::Set(len.to_vec());
        self
    }

    pub fn remove_len(&mut self) -> &mut Self {
        self.len = ChunkLen::None;
        self
    }

    pub fn payload(&mut self, payload: &[u8]) -> &mut Self {
        self.payload = payload.to_vec();
        self
    }

    pub fn build(&self) -> Vec<u8> {
        match self.len {
            ChunkLen::None => self.id.clone(),
            ChunkLen::Computed => {
                let len = self.payload.len() as u32;
                [
                    &self.id as &[u8],
                    &len.to_le_bytes(),
                    &self.payload as &[u8],
                ]
                .concat()
            }
            ChunkLen::Set(ref l) => [&self.id as &[u8], l, &self.payload as &[u8]].concat(),
        }
    }
}

pub struct BytesBuilder {
    magic: [u8; 4],
    format_version: u16,
    flags: u16,
    len: Option<u32>,
    chunks: Vec<u8>,
}

impl BytesBuilder {
    pub fn new() -> Self {
        BytesBuilder {
            magic: *b"YFF\0",
            format_version: 0,
            flags: 0,
            len: None,
            chunks: vec![],
        }
    }

    pub fn magic(&mut self, magic: [u8; 4]) -> &mut Self {
        self.magic = magic;
        self
    }

    pub fn format_version(&mut self, format_version: u16) -> &mut Self {
        self.format_version = format_version;
        self
    }

    pub fn flags(&mut self, flags: u16) -> &mut Self {
        self.flags = flags;
        self
    }

    pub fn len(&mut self, len: u32) -> &mut Self {
        self.len = Some(len);
        self
    }

    pub fn add_meta(&mut self, entry: Option<&str>, version: &str, req: &str) -> &mut Self {
        let meta_string = match entry {
            None => dummy_no_entry_metadata(version, req),
            Some(e) => dummy_metadata(e, version, req),
        };
        self.chunks.extend(
            [
                ID_META,
                &(meta_string.len() as u32).to_le_bytes(),
                meta_string.as_bytes(),
            ]
            .concat(),
        );
        self
    }

    pub fn add_default_rhai(&mut self, name: &str) -> &mut Self {
        let payload: &[u8] = &[
            &(name.len() as u8).to_le_bytes(),
            name.as_bytes(),
            "fn load(mod) { }".as_bytes(),
        ]
        .concat();
        self.chunks
            .extend([ID_RHAI, &(payload.len() as u32).to_le_bytes(), payload].concat());
        self
    }

    pub fn add_arbitrary_chunk(&mut self, c: &[u8]) -> &mut Self {
        self.chunks.extend(c);
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.magic);
        bytes.extend(self.format_version.to_le_bytes());
        bytes.extend(self.flags.to_le_bytes());
        bytes.extend(self.len.unwrap_or(self.chunks.len() as u32).to_le_bytes());
        bytes.extend(self.chunks.clone());
        bytes
    }
}
