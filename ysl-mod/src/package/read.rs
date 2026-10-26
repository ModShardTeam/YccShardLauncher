use std::collections::{BTreeMap, btree_map::Entry};

use crate::package;

#[repr(C)]
#[derive(Debug, PartialEq)]
struct ChunkHeader {
    four_cc: [u8; 4],
    len: [u8; 4],
}

const _: () = assert!(size_of::<ChunkHeader>() == 8);

impl ChunkHeader {
    fn read(bytes: &[u8]) -> Option<(ChunkHeader, &[u8])> {
        let (&four_cc, rest) = bytes.split_first_chunk()?;
        let (&len, rest) = rest.split_first_chunk()?;
        Some((ChunkHeader { four_cc, len }, rest))
    }
}
pub struct ChunkIter<'a> {
    state: &'a [u8],
}

impl<'a> From<&'a [u8]> for ChunkIter<'a> {
    fn from(value: &'a [u8]) -> Self {
        ChunkIter { state: value }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<package::Chunk<'a>, package::ChunkRejection>;

    fn next(&mut self) -> Option<Result<package::Chunk<'a>, package::ChunkRejection>> {
        let val = self.state;
        if val.is_empty() {
            return None;
        }

        match parse_chunk(val) {
            Ok((rc, r)) => {
                self.state = r;
                Some(Ok(rc))
            }

            Err(e) => {
                self.state = &[];
                Some(Err(e))
            }
        }
    }
}

pub fn parse_chunk<'a>(
    bytes: &'a [u8],
) -> Result<(package::Chunk<'a>, &'a [u8]), package::ChunkRejection> {
    let Some((chunk_header, rest)) = ChunkHeader::read(bytes) else {
        return Err(package::ChunkRejection::NotEnoughDataHeader {
            needed: size_of::<ChunkHeader>(),
            available: bytes.len(),
        });
    };

    let len: u32 = u32::from_le_bytes(chunk_header.len);
    let Some((payload, rest)) = rest.split_at_checked(len as usize) else {
        return Err(package::ChunkRejection::NotEnoughDataPayload {
            declared: len,
            available: rest.len(),
        });
    };

    let chunk = package::Chunk {
        four_cc: chunk_header.four_cc,
        payload,
    };
    package::validate::validate_size_chunk(&chunk)?;

    Ok((chunk, rest))
}

fn decode_rhai_payload(payload: &[u8]) -> Result<(String, String), package::RhaiRejection> {
    let Some((&name_len, rest)) = payload.split_first() else {
        return Err(package::RhaiRejection::NullPayload);
    };

    if name_len == 0 {
        return Err(package::RhaiRejection::Name(package::NameRejection::Empty));
    }

    let Some((name_bytes, rest)) = rest.split_at_checked(name_len as usize) else {
        return Err(package::RhaiRejection::Name(
            package::NameRejection::NameLenPastEnd {
                declared: name_len as usize,
                available: rest.len(),
            },
        ));
    };
    let name = String::from_utf8(name_bytes.to_vec()).map_err(|e| {
        package::RhaiRejection::InvalidByteName {
            name: name_bytes.to_vec(),
            reason: format!("{e:#}"),
        }
    })?;

    let data =
        String::from_utf8(rest.to_vec()).map_err(|e| package::RhaiRejection::InvalidScript {
            name: name.to_owned(),
            reason: format!("{e:#}"),
        })?;

    Ok((name, data))
}

pub fn read(bytes: &[u8]) -> Result<package::Package, package::PackageRejection> {
    if bytes.len() >= package::MAXIMUM_FILE_SIZE {
        return Err(package::PackageRejection::TooBigFile {
            declared: bytes.len(),
        });
    }

    let Some((raw_header, raw_chunks)) = package::Header::read(bytes) else {
        return Err(package::PackageRejection::NotEnoughDataHeader {
            needed: size_of::<package::Header>(),
            available: bytes.len(),
        });
    };
    package::validate::validate_header_values(&raw_header, raw_chunks.len())?;
    let chunks = ChunkIter::from(raw_chunks).collect::<Result<Vec<_>, _>>()?;

    let mut metadata: Option<&package::Chunk> = None;
    let mut rhai_scripts: BTreeMap<String, String> = BTreeMap::new();

    // read all chunks
    for c in &chunks {
        match c.four_cc.try_into() {
            Ok(package::FourCC::Metadata) => match metadata {
                None => metadata = Some(c),
                Some(_) => return Err(package::PackageRejection::MultipleMetaChunkFound),
            },
            Ok(package::FourCC::RhaiScript) => {
                let (name, data) = decode_rhai_payload(c.payload)?;
                match rhai_scripts.entry(name) {
                    Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(data);
                    }
                    Entry::Occupied(occupied_entry) => {
                        return Err(package::PackageRejection::Rhai(
                            package::RhaiRejection::DuplicateName {
                                found: occupied_entry.key().to_owned(),
                            },
                        ));
                    }
                }
            }
            Ok(k @ (package::FourCC::LuaScript | package::FourCC::Png)) => {
                return Err(package::PackageRejection::KnownButUnsupportedId { found: k });
            }
            Err(_unknown) => {} // totally unknown ids are skipped
        }
    }

    let Some(meta_chunk) = metadata else {
        return Err(package::PackageRejection::NoMetaChunk);
    };

    let meta: package::Metadata = match toml::from_slice(meta_chunk.payload) {
        Err(e) => return Err(package::PackageRejection::InvalidDeserMetadata(e)),
        Ok(meta) => meta,
    };

    let package = package::Package { meta, rhai_scripts };
    package::validate::validate_package(&package)?;
    Ok(package)
}
