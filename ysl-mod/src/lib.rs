#![forbid(unsafe_code)]

pub mod process_data;
pub mod raw_parser;

#[non_exhaustive]
pub enum ParserRejection {
    YffRejection(raw_parser::YffRejection),
    PackageRejection(process_data::PackageRejection),
}

impl From<raw_parser::YffRejection> for ParserRejection {
    fn from(value: raw_parser::YffRejection) -> Self {
        ParserRejection::YffRejection(value)
    }
}

impl From<process_data::PackageRejection> for ParserRejection {
    fn from(value: process_data::PackageRejection) -> Self {
        ParserRejection::PackageRejection(value)
    }
}

pub fn parse(bytes: &[u8]) -> Result<process_data::Package, ParserRejection> {
    let raw_file = raw_parser::parse_file(bytes)?;
    let file = process_data::File::from_raw(raw_file);
    let package = process_data::process_package(file)?;

    Ok(package)
}
