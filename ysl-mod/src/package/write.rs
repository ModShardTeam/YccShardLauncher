use crate::package;

fn encode_rhai_payload(name: &str, data: &str) -> Result<Vec<u8>, package::RhaiRejection> {
    Ok([
        &[u8::try_from(name.len()).map_err(|_| {
            package::RhaiRejection::Name(package::NameRejection::TooLong {
                name: name.to_string(),
                len: name.len(),
            })
        })?],
        name.as_bytes(),
        data.as_bytes(),
    ]
    .concat())
}

// TODO: use struct first to ensure size of each element
pub fn write_chunk(c: &package::ChunkBuf) -> Result<Vec<u8>, package::ChunkRejection> {
    Ok([
        &c.four_cc as &[u8],
        &(u32::try_from(c.payload.len()).map_err(|_| package::ChunkRejection::TooBigChunk {
            declared: c.payload.len(),
        })?)
        .to_le_bytes(),
        &c.payload,
    ]
    .concat())
}

pub fn write(package: &package::Package, flags: u16) -> Result<Vec<u8>, package::PackageRejection> {
    package::validate::validate_flags(&flags)?;
    package::validate::validate_package(package)?;

    let meta_payload = match toml::to_string(&package.meta) {
        Err(e) => return Err(package::PackageRejection::InvalidSerMetadata(e)),
        Ok(meta) => meta,
    };

    let meta_chunk = package::ChunkBuf {
        four_cc: package::FourCC::Metadata.into(),
        payload: meta_payload.as_bytes().to_vec(),
    };
    package::validate::validate_size_chunk(&(&meta_chunk).into())?;

    let mut chunks: Vec<package::ChunkBuf> = vec![meta_chunk];

    for (n, d) in &package.rhai_scripts {
        let rhai_chunk = package::ChunkBuf {
            four_cc: package::FourCC::RhaiScript.into(),
            payload: encode_rhai_payload(n, d)?,
        };
        package::validate::validate_size_chunk(&(&rhai_chunk).into())?;
        chunks.push(rhai_chunk);
    }

    let mut header = [
        package::YFF_MAGIC as &[u8],
        &package::CURRENT_FORMAT_VERSION.to_le_bytes(),
        &flags.to_le_bytes(),
    ]
    .concat();

    let payload: Vec<u8> = chunks
        .iter()
        .map(write_chunk)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    let bytes = [header, payload].concat();

    if bytes.len() >= package::MAXIMUM_FILE_SIZE {
        return Err(package::PackageRejection::TooBigFile {
            declared: bytes.len(),
        });
    }
    Ok(bytes)
}
