use std::collections::{BTreeMap, btree_map::Entry};

use crate::{package, raw};

fn decode_rhai_payload(payload: &[u8]) -> Result<(String, String), package::PackageRejection> {
    let Some((&name_len, rest)) = payload.split_first() else {
        return Err(package::PackageRejection::RhaiNullPayload);
    };

    if name_len == 0 {
        return Err(package::PackageRejection::RhaiNullName);
    }

    let Some((name_bytes, rest)) = rest.split_at_checked(name_len as usize) else {
        return Err(package::PackageRejection::RhaiInvalidNameLen);
    };

    let name: String = name_bytes
        .iter()
        .map(|&b| {
            if package::name::is_valid_name_byte(b) {
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
    if package::name::RESERVED_WINDOWS_DEVICE_NAME.contains(&uppercase) {
        return Err(package::PackageRejection::RhaiInvalidName {
            name: name.to_string(),
            reason: "Reserved device name".to_string(),
        });
    }

    let data = String::from_utf8(rest.to_vec()).map_err(|e| {
        package::PackageRejection::RhaiInvalidScript {
            name: name.to_string(),
            reason: format!("{e:#}"),
        }
    })?;

    Ok((name.to_string(), data))
}

pub fn read<'a>(file: raw::File<'a>) -> Result<package::Package, package::PackageRejection> {
    let chunks = file
        .chunks
        .iter()
        .filter_map(|&rc| package::Chunk::from_raw(rc))
        .collect::<Vec<_>>();
    let mut metadata_filter = chunks
        .iter()
        .filter(|&c| c.four_cc == package::FourCC::Metadata);

    let rhai_filter = chunks
        .iter()
        .filter(|&c| c.four_cc == package::FourCC::RhaiScript);

    let Some(meta_chunk) = metadata_filter.next() else {
        return Err(package::PackageRejection::NoMetaChunk);
    };
    if metadata_filter.next().is_some() {
        return Err(package::PackageRejection::MultipleMetaChunkFound);
    }
    let meta: package::Metadata = match toml::from_slice(meta_chunk.payload) {
        Err(e) => return Err(package::PackageRejection::InvalidMetadata(e)),
        Ok(meta) => meta,
    };

    let mut rhai_scripts: BTreeMap<String, String> = BTreeMap::new();
    for c in rhai_filter {
        let (name, data) = decode_rhai_payload(c.payload)?;
        match rhai_scripts.entry(name) {
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(data);
            }
            Entry::Occupied(occupied_entry) => {
                return Err(package::PackageRejection::RhaiDuplicateName {
                    found: occupied_entry.key().to_owned(),
                });
            }
        }
    }

    let expected_entry_point = meta.entry();
    if !rhai_scripts.contains_key(expected_entry_point) {
        return Err(package::PackageRejection::NoEntryPoint {
            expected: expected_entry_point.to_string(),
        });
    }

    Ok(package::Package { meta, rhai_scripts })
}
