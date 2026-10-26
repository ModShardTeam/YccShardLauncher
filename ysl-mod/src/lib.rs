#![forbid(unsafe_code)]

use std::path::PathBuf;

pub mod package;

pub fn read(bytes: &[u8]) -> Result<package::Package, package::PackageRejection> {
    package::read::read(bytes)
}

pub fn export(
    package: &package::Package,
    folder: PathBuf,
) -> Result<Vec<(PathBuf, Vec<u8>)>, package::PackageRejection> {
    package::export::export(package, folder)
}

pub fn import(paths: Vec<(PathBuf, Vec<u8>)>) -> package::Package {
    todo!()
}

pub fn write(package: &package::Package, flags: u16) -> Result<Vec<u8>, package::PackageRejection> {
    package::write::write(package, flags)
}
