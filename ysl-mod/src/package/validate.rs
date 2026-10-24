use crate::package;

pub const RESERVED_WINDOWS_DEVICE_NAME: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

pub fn is_valid_name_byte(b: u8) -> bool {
    matches!(b, b'-' | b'_' | b'a'..=b'z' | b'0'..=b'9')
}

fn validate_name(name: &str) -> Result<(), package::PackageRejection> {
    if name.len() == 0 {
        return Err(package::PackageRejection::RhaiNullName);
    }

    if let Err(_) = u32::try_from(name.len()) {
        return Err(package::PackageRejection::RhaiInvalidNameLen);
    }

    if let Some(c) = name
        .chars()
        .find(|&c| !(c.is_ascii() && is_valid_name_byte(c as u8)))
    {
        return Err(package::PackageRejection::RhaiInvalidName {
            name: name.to_string(),
            reason: format!("Invalid character {c:?} found"),
        });
    }

    let uppercase = &name.to_ascii_uppercase() as &str;
    if RESERVED_WINDOWS_DEVICE_NAME.contains(&uppercase) {
        Err(package::PackageRejection::RhaiInvalidName {
            name: name.to_string(),
            reason: "Reserved device name".to_string(),
        })
    } else {
        Ok(())
    }
}

pub fn convert_name(name_bytes: &[u8]) -> Result<String, package::PackageRejection> {
    let name: String = name_bytes
        .iter()
        .map(|&b| {
            if is_valid_name_byte(b) {
                Ok(b as char)
            } else {
                Err(package::PackageRejection::RhaiInvalidByteName {
                    name: name_bytes.to_vec(),
                    reason: format!("Invalid character {:?} ({b:#x}) found in name", b as char),
                })
            }
        })
        .collect::<Result<_, package::PackageRejection>>()?;

    let uppercase = &name.to_ascii_uppercase() as &str;
    if RESERVED_WINDOWS_DEVICE_NAME.contains(&uppercase) {
        Err(package::PackageRejection::RhaiInvalidName {
            name: name.to_string(),
            reason: "Reserved device name".to_string(),
        })
    } else {
        Ok(name)
    }
}

pub fn validate(package: &package::Package) -> Result<(), package::PackageRejection> {
    let expected_entry_point = package.entry();
    if !package.rhai_scripts.contains_key(expected_entry_point) {
        return Err(package::PackageRejection::NoEntryPoint {
            expected: expected_entry_point.to_string(),
        });
    }

    for (name, _data) in package.rhai_scripts.iter() {
        validate_name(&name)?;
    }

    Ok(())
}
