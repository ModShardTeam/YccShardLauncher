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
    if payload.len() > package::MAXIMUM_PAYLOAD_SIZE {
        return Err(package::ChunkRejection::TooBigPayload {
            declared: payload.len(),
        });
    }

    Ok((
        package::Chunk {
            four_cc: chunk_header.four_cc,
            payload,
        },
        rest,
    ))
}

pub fn parse_header(bytes: &[u8]) -> Result<(package::Header, &[u8]), package::PackageRejection> {
    let Some((header, rest)) = package::Header::read(bytes) else {
        return Err(package::PackageRejection::NotEnoughDataHeader {
            needed: size_of::<package::Header>(),
            available: bytes.len(),
        });
    };

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
    if (flags & !package::KNOWN_MASK) != 0 {
        return Err(package::PackageRejection::UnknownBits { found: flags });
    }

    let len: u32 = u32::from_le_bytes(header.len);
    // exclude the first 12 bytes, RIFF-style
    if rest.len() != len as usize {
        return Err(package::PackageRejection::InvalidSizeFile {
            declared: len,
            available: rest.len(),
        });
    }

    Ok((header, rest))
}

fn decode_rhai_payload(payload: &[u8]) -> Result<(String, String), package::RhaiRejection> {
    let Some((&name_len, rest)) = payload.split_first() else {
        return Err(package::RhaiRejection::NullPayload);
    };

    if name_len == 0 {
        return Err(package::RhaiRejection::Name(package::NameRejection::Null));
    }

    let Some((name_bytes, rest)) = rest.split_at_checked(name_len as usize) else {
        return Err(package::RhaiRejection::Name(
            package::NameRejection::InvalidLen,
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

    let (raw_header, raw_chunks) = parse_header(bytes)?;
    let format_version: u16 = u16::from_le_bytes(raw_header.format_version);
    let flags: u16 = u16::from_le_bytes(raw_header.flags);

    let mut metadata_filter = ChunkIter::from(raw_chunks).filter(|r| {
        r.as_ref().is_ok_and(|c| {
            c.four_cc
                == <package::FourCC as std::convert::Into<[u8; 4]>>::into(package::FourCC::Metadata)
        })
    });

    let rhai_filter = ChunkIter::from(raw_chunks).filter(|r| {
        r.as_ref().is_ok_and(|c| {
            c.four_cc
                == <package::FourCC as std::convert::Into<[u8; 4]>>::into(
                    package::FourCC::RhaiScript,
                )
        })
    });

    let Some(Ok(meta_chunk)) = metadata_filter.next() else {
        return Err(package::PackageRejection::NoMetaChunk);
    };
    if metadata_filter.next().is_some() {
        return Err(package::PackageRejection::MultipleMetaChunkFound);
    }
    let meta: package::Metadata = match toml::from_slice(meta_chunk.payload) {
        Err(e) => return Err(package::PackageRejection::InvalidDeserMetadata(e)),
        Ok(meta) => meta,
    };

    let mut rhai_scripts: BTreeMap<String, String> = BTreeMap::new();
    for c in rhai_filter {
        let (name, data) = decode_rhai_payload(c?.payload)?;
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

    let package = package::Package { meta, rhai_scripts };
    package::validate::validate(&package)?;
    Ok(package)
}
