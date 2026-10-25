#![allow(dead_code)]

use std::collections::BTreeMap;

use ysl_mod::package;
pub const ID_META: &[u8; 4] = b"META";
pub const ID_RHAI: &[u8; 4] = b"RHAI";

pub fn create_raw_chunk(id: &[u8; 4], len: u32, payload: &[u8]) -> Vec<u8> {
    let mut array: Vec<u8> = vec![];
    array.extend_from_slice(id);
    array.extend_from_slice(&len.to_le_bytes());
    array.extend_from_slice(payload);
    array
}

pub fn create_raw_header(magic: &[u8; 4], format_version: u16, flags: u16, len: u32) -> Vec<u8> {
    let mut array: Vec<u8> = vec![];
    array.extend_from_slice(magic);
    array.extend_from_slice(&format_version.to_le_bytes());
    array.extend_from_slice(&flags.to_le_bytes());
    array.extend_from_slice(&len.to_le_bytes());
    array
}

pub fn sample_raw_body() -> Vec<u8> {
    let mut body: Vec<u8> = vec![];
    body.extend_from_slice(&create_raw_chunk(ID_META, 3, &[0xA0, 0xA1, 0xA2]));
    body.extend_from_slice(&create_raw_chunk(ID_RHAI, 4, &[0xB0, 0xB1, 0xB2, 0xB3]));
    body.extend_from_slice(&create_raw_chunk(ID_RHAI, 4, &[0xC0, 0xC1, 0xC2, 0xC3]));
    body.extend_from_slice(&create_raw_chunk(ID_RHAI, 4, &[0xD0, 0xD1, 0xD2, 0xD3]));
    body.extend_from_slice(&create_raw_chunk(ID_RHAI, 4, &[0x00, 0x01, 0x02, 0x03]));
    body
}

pub fn dummy_metadata(entry: &str) -> String {
    format!(
        r#"mod_name = "test_mod"
version = "0.1.0"
engine = "rhai"
entry_point = "{}"
game_version = "1.0.0"
author_name = "test_author"
short_desc = "this is a test"

[dependencies]
other_test = "=1.0"
yet_another_test = ">0.2"
"#,
        entry
    )
}

pub fn dummy_no_entry_metadata() -> &'static str {
    r#"
mod_name = "test_mod"
version = "0.1.0"
engine = "rhai"
game_version = "1.0.0"
author_name = "test_author"
short_desc = "this is a test"

[dependencies]
other_test = "=1.0"
yet_another_test = ">0.2"

"#
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

    pub fn add_rhai(&mut self, name: &str) -> &mut Self {
        self.rhai_scripts
            .insert(name.to_string(), "fn load(mod) { }".to_string());
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
