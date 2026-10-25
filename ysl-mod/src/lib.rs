#![forbid(unsafe_code)]

pub mod package;

pub fn read(bytes: &[u8]) -> Result<package::Package, package::PackageRejection> {
    package::read::read(bytes)
}

pub fn write(package: &package::Package, flags: u16) -> Result<Vec<u8>, package::PackageRejection> {
    package::write::write(package, flags)
}
