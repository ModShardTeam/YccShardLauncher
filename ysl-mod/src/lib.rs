#![forbid(unsafe_code)]

pub mod package;
pub mod raw;

#[non_exhaustive]
pub enum Rejection {
    RawRejection(raw::RawRejection),
    PackageRejection(package::PackageRejection),
}

impl From<raw::RawRejection> for Rejection {
    fn from(value: raw::RawRejection) -> Self {
        Rejection::RawRejection(value)
    }
}

impl From<package::PackageRejection> for Rejection {
    fn from(value: package::PackageRejection) -> Self {
        Rejection::PackageRejection(value)
    }
}

pub fn parse(bytes: &[u8]) -> Result<package::Package, Rejection> {
    let raw_file = raw::read::parse_file(bytes)?;
    let package = package::read::read(raw_file)?;

    Ok(package)
}
