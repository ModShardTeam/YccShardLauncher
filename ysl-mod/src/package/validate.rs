use crate::package;

pub const RESERVED_WINDOWS_DEVICE_NAME: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

pub fn is_valid_name_byte(b: u8) -> bool {
    matches!(b, b'-' | b'_' | b'a'..=b'z' | b'0'..=b'9')
}

fn validate_name(name: &str) -> Result<(), package::NameRejection> {
    if name.is_empty() {
        return Err(package::NameRejection::Empty);
    }

    if u8::try_from(name.len()).is_err() {
        return Err(package::NameRejection::TooLong {
            name: name.to_string(),
            len: name.len(),
        });
    }

    if let Some(c) = name
        .chars()
        .find(|&c| !(c.is_ascii() && is_valid_name_byte(c as u8)))
    {
        return Err(package::NameRejection::Invalid {
            name: name.to_string(),
            reason: format!("Invalid character {c:?} found"),
        });
    }

    let uppercase = &name.to_ascii_uppercase() as &str;
    if RESERVED_WINDOWS_DEVICE_NAME.contains(&uppercase) {
        Err(package::NameRejection::Invalid {
            name: name.to_string(),
            reason: "Reserved device name".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn validate_package(package: &package::Package) -> Result<(), package::PackageRejection> {
    validate_name(&package.meta.mod_name).map_err(package::PackageRejection::ModName)?;

    let expected_entry_point = package.entry();
    validate_name(expected_entry_point).map_err(package::EntryRejection::Name)?;
    if !package.rhai_scripts.contains_key(expected_entry_point) {
        return Err(package::PackageRejection::Entry(
            package::EntryRejection::None {
                expected: expected_entry_point.to_string(),
            },
        ));
    }

    for name in package.rhai_scripts.keys() {
        validate_name(name).map_err(package::RhaiRejection::Name)?;
    }

    Ok(())
}

pub fn validate_size_chunk(c: &package::Chunk) -> Result<(), package::ChunkRejection> {
    if c.size() > package::MAXIMUM_CHUNK_SIZE {
        return Err(package::ChunkRejection::TooBigChunk { declared: c.size() });
    }
    Ok(())
}

pub fn validate_flags(flags: &u16) -> Result<(), package::PackageRejection> {
    if (flags & !package::KNOWN_MASK) != 0 {
        return Err(package::PackageRejection::UnknownBits { found: *flags });
    }

    Ok(())
}

pub fn validate_header_values(
    header: &package::Header,
    len_raw_chunks: usize,
) -> Result<(), package::PackageRejection> {
    if header.magic != *package::YFF_MAGIC {
        return Err(package::PackageRejection::BadMagic {
            found: header.magic,
        });
    }

    let version = u16::from_le_bytes(header.format_version);
    if !(package::OLDEST_SUPPORTED_VERSION..=package::CURRENT_FORMAT_VERSION).contains(&version) {
        return Err(package::PackageRejection::UnsupportedVersion { found: version });
    }

    let flags = u16::from_le_bytes(header.flags);
    validate_flags(&flags)?;

    let len: u32 = u32::from_le_bytes(header.len);
    // exclude the first 12 bytes, RIFF-style
    if len_raw_chunks != len as usize {
        return Err(package::PackageRejection::InvalidSizeFile {
            declared: len,
            available: len_raw_chunks,
        });
    }

    Ok(())
}
