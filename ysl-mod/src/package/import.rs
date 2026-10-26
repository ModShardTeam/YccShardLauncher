use std::{ffi::OsStr, path::PathBuf};

use crate::package;

pub fn import(
    paths: Vec<(PathBuf, Vec<u8>)>,
) -> Result<package::Package, package::PackageRejection> {
    let mut builder = package::builder::PackageBuilder::new("", package::Engine::Rhai);
    for (path, data) in paths {
        let Some(parent) = path.parent() else {
            return Err(package::PackageRejection::InvalidFolder {
                folder: format!("{path:?}"),
                reason: "Cannot find its parent".to_string(),
            });
        };

        if parent == "rhai" {
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                return Err(
                    package::RhaiRejection::Name(package::NameRejection::Invalid {
                        name: format!("{path:?}"),
                        reason: "cannot decode from path".to_string(),
                    })
                    .into(),
                );
            };
            let script =
                String::from_utf8(data).map_err(|e| package::RhaiRejection::InvalidScript {
                    name: name.to_owned(),
                    reason: format!("{e:#}"),
                })?;
            builder.add_rhai(name, &script);
        } else if path.file_name() == Some(OsStr::new("mod.toml")) {
            let metadata = match toml::from_slice(&data) {
                Err(e) => return Err(package::PackageRejection::InvalidDeserMetadata(e)),
                Ok(meta) => meta,
            };
            builder.add_meta(metadata);
        }
    }

    let package = builder.build()?;
    package::validate::validate_package(&package)?;

    Ok(package)
}
