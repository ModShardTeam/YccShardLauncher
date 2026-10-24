use crate::package;

fn encode_rhai_payload(name: &str, data: &str) -> Vec<u8> {
    [&[name.len() as u8], name.as_bytes(), data.as_bytes()].concat()
}

// TODO: use struct first to ensure size of each element
pub fn write_chunk(c: &package::ChunkBuf) -> Vec<u8> {
    [
        &c.four_cc as &[u8],
        &(c.payload.len() as u32).to_le_bytes(),
        &c.payload,
    ]
    .concat()
}

pub fn write(package: &package::Package, flags: u16) -> Result<Vec<u8>, package::PackageRejection> {
    package::validate::validate(package)?;

    let meta_payload = match toml::to_string(&package.meta) {
        Err(e) => return Err(package::PackageRejection::InvalidSerMetadata(e)),
        Ok(meta) => meta,
    };

    let meta_chunk = package::ChunkBuf {
        four_cc: package::FourCC::Metadata.into(),
        payload: meta_payload.as_bytes().to_vec(),
    };

    let mut chunks: Vec<package::ChunkBuf> = vec![meta_chunk];
    chunks.extend(package.rhai_scripts.iter().map(|(n, d)| package::ChunkBuf {
        four_cc: package::FourCC::RhaiScript.into(),
        payload: encode_rhai_payload(n, d),
    }));

    let mut header = [
        package::YFF_MAGIC as &[u8],
        &package::CURRENT_FORMAT_VERSION.to_le_bytes(),
        &flags.to_le_bytes(),
    ]
    .concat();

    let payload = chunks.iter().flat_map(write_chunk).collect::<Vec<_>>();

    header.extend_from_slice(&(payload.len() as u32).to_le_bytes());

    Ok([header, payload].concat())
}
