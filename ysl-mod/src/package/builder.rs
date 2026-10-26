use std::collections::BTreeMap;

use crate::package;

pub struct PackageBuilder {
    mod_name: String,
    version: semver::Version,
    engine: package::Engine,
    entry_point: Option<String>,
    game_version: String,
    author_name: String,
    short_desc: Option<String>,
    dependencies: BTreeMap<String, semver::VersionReq>,
    rhai_scripts: BTreeMap<String, String>,
}

impl PackageBuilder {
    pub fn new(name: &str, engine: package::Engine) -> Self {
        PackageBuilder {
            mod_name: name.to_string(),
            version: semver::Version::new(1, 0, 0),
            engine: engine,
            entry_point: None,
            game_version: "1.0.0".to_string(),
            author_name: "".to_string(),
            short_desc: None,
            dependencies: BTreeMap::new(),
            rhai_scripts: BTreeMap::new(),
        }
    }

    pub fn mod_name(&mut self, name: &str) -> &mut Self {
        self.mod_name = name.to_string();
        self
    }

    pub fn version(&mut self, major: u64, minor: u64, patch: u64) -> &mut Self {
        self.version = semver::Version::new(major, minor, patch);
        self
    }

    pub fn engine(&mut self, engine: package::Engine) -> &mut Self {
        self.engine = engine;
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

    pub fn add_dependencies(&mut self, name: &str, req: &str) -> Result<&mut Self, semver::Error> {
        self.dependencies
            .insert(name.to_string(), semver::VersionReq::parse(req)?);
        Ok(self)
    }

    pub fn clear_rhai(&mut self) -> &mut Self {
        self.rhai_scripts.clear();
        self
    }

    pub fn add_rhai(&mut self, name: &str, data: &str) -> &mut Self {
        self.rhai_scripts.insert(name.to_string(), data.to_string());
        self
    }

    pub fn build(&mut self) -> Result<package::Package, package::PackageRejection> {
        match self.engine {
            package::Engine::Rhai => {
                if self.rhai_scripts.is_empty() {
                    self.rhai_scripts
                        .insert("mod".to_string(), "fn load(mod) {\n\n}".to_string());
                }
            }
            package::Engine::Lua => todo!("lua is not supported yet"),
        };
        let package = package::Package {
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
        };
        package::validate::validate_package(&package)?;
        Ok(package)
    }
}
