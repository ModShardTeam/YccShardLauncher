use crate::{package, raw};

fn encode_rhai_payload(name: &String, data: &String) -> Vec<u8> {
    [&[name.len() as u8], name.as_bytes(), data.as_bytes()].concat()
}

pub fn write(
    package: package::Package,
    flags: u16,
) -> Result<raw::WFile, package::PackageRejection> {
    let meta_payload = match toml::to_string(&package.meta) {
        Err(e) => return Err(package::PackageRejection::InvalidSerMetadata(e)),
        Ok(meta) => meta,
    };

    let meta_chunk = raw::WChunk {
        four_cc: package::FourCC::Metadata.into(),
        payload: meta_payload.as_bytes().to_vec(),
    };

    let mut chunks: Vec<raw::WChunk> = vec![meta_chunk];
    chunks.extend(package.rhai_scripts.iter().map(|(n, d)| raw::WChunk {
        four_cc: package::FourCC::RhaiScript.into(),
        payload: encode_rhai_payload(n, d),
    }));

    Ok(raw::WFile {
        format_version: raw::CURRENT_FORMAT_VERSION,
        flags,
        chunks,
    })
}
