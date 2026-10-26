use std::path::PathBuf;

use crate::package;

pub fn export(
    package: &package::Package,
    folder: PathBuf,
) -> Result<Vec<(PathBuf, Vec<u8>)>, package::PackageRejection> {
    package::validate::validate_package(package)?;

    let mut path_data = vec![];

    if !folder.is_dir() {
        return Err(package::PackageRejection::InvalidFolder {
            folder: format!("{folder:?}"),
            reason: "not a directory".to_string(),
        });
    }

    let root = folder.join(&package.meta.mod_name);

    if root.exists()
        && root
            .read_dir()
            .map_err(|e| package::PackageRejection::InvalidFolder {
                folder: format!("{root:?}"),
                reason: format!("{e:#}"),
            })?
            .next()
            .is_some()
    {
        return Err(package::PackageRejection::NonEmptyFolder {
            folder: format!("{root:?}"),
        });
    }

    path_data.push((
        root.join("mod.toml"),
        toml::to_string(&package.meta)
            .map_err(package::PackageRejection::InvalidSerMetadata)?
            .as_bytes()
            .to_vec(),
    ));

    for (name, data) in &package.rhai_scripts {
        path_data.push((
            root.join(format!("rhai/{name}.rhai")),
            data.as_bytes().to_vec(),
        ));
    }

    Ok(path_data)
}
