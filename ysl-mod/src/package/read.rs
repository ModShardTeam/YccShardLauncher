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
    let name = package::name::validate_name(name_bytes)?;

    let data = String::from_utf8(rest.to_vec()).map_err(|e| {
        package::PackageRejection::RhaiInvalidScript {
            name: name.to_owned(),
            reason: format!("{e:#}"),
        }
    })?;

    Ok((name, data))
}

pub fn read<'a>(file: raw::File<'a>) -> Result<package::Package, package::PackageRejection> {
    let mut metadata_filter = file.chunks.iter().filter(|&c| {
        c.four_cc
            == <package::FourCC as std::convert::Into<[u8; 4]>>::into(package::FourCC::Metadata)
    });

    let rhai_filter = file.chunks.iter().filter(|&c| {
        c.four_cc
            == <package::FourCC as std::convert::Into<[u8; 4]>>::into(package::FourCC::RhaiScript)
    });

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
