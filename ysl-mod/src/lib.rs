#![forbid(unsafe_code)]

pub mod package;

pub fn parse(bytes: &[u8]) -> Result<package::Package, package::PackageRejection> {
    package::read::read(bytes)
}
