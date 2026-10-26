use std::path::PathBuf;

use crate::package;

pub fn import(
    paths: Vec<(PathBuf, Vec<u8>)>,
) -> Result<package::Package, package::PackageRejection> {
    todo!()
}
