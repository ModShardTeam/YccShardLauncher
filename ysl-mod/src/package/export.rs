use std::path::PathBuf;

use crate::package;

pub fn export(
    package: &package::Package,
    folder: PathBuf,
) -> Result<Vec<(PathBuf, Vec<u8>)>, package::PackageRejection> {
    package::validate::validate_package(package)?;

    let mut path_data = vec![];
    //TODO: test if folder exist
    //TODO: test if folder is a folder
    //TODO: test if folder is empty

    let root = &package.meta.mod_name;
    path_data.push((
        folder.join(format!("{root}/mod.toml")),
        toml::to_string(&package.meta)
            .map_err(package::PackageRejection::InvalidSerMetadata)?
            .as_bytes()
            .to_vec(),
    ));

    for (name, data) in &package.rhai_scripts {
        path_data.push((
            folder.join(format!("{root}/rhai/{name}.rhai")),
            data.as_bytes().to_vec(),
        ));
    }

    Ok(path_data)
}
