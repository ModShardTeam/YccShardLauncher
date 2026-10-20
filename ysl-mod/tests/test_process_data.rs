mod common;
use ysl_mod::process_data;
use ysl_mod::raw_parser;

#[test]
fn convert_valid_id_chunk() {
    let chunk = process_data::Chunk::from_raw(raw_parser::RawChunk {
        four_cc: *common::ID_RHAI,
        payload: &[0x00, 0x01, 0x02, 0x03],
    })
    .expect("valid operation");
    assert_eq!(
        chunk,
        process_data::Chunk {
            four_cc: process_data::FourCC::RhaiScript,
            payload: &[0x00, 0x01, 0x02, 0x03]
        },
    )
}

#[test]
fn convert_invalid_id_chunk() {
    let rej = process_data::Chunk::from_raw(raw_parser::RawChunk {
        four_cc: *b"BAD\0",
        payload: &[0x00, 0x01, 0x02, 0x03],
    });
    assert_eq!(rej, None,)
}

#[test]
fn convert_skips_unknown_id() {
    let raw_file = raw_parser::RawFile {
        header: raw_parser::RawFileHeader {
            magic: *b"YFF\0",
            len: [0x18, 0x00, 0x00, 0x00],
            flags: [0x00, 0x00],
            format_version: [0x01, 0x00],
        },
        chunks: vec![
            raw_parser::RawChunk {
                four_cc: *common::ID_META,
                payload: &[0xA0, 0xA1, 0xA2, 0xA3],
            },
            raw_parser::RawChunk {
                four_cc: *b"BAD\0",
                payload: &[0xB0, 0xB1, 0xB2, 0xB3],
            },
            raw_parser::RawChunk {
                four_cc: *common::ID_RHAI,
                payload: &[0xC0, 0xC1, 0xC2, 0xC3],
            },
        ],
    };

    let file = process_data::File::from_raw(raw_file);
    assert_eq!(
        file,
        process_data::File {
            header: process_data::FileHeader {
                format_version: 1,
                flags: 0
            },
            chunks: vec![
                process_data::Chunk {
                    four_cc: process_data::FourCC::Metadata,
                    payload: &[0xA0, 0xA1, 0xA2, 0xA3]
                },
                process_data::Chunk {
                    four_cc: process_data::FourCC::RhaiScript,
                    payload: &[0xC0, 0xC1, 0xC2, 0xC3]
                },
            ]
        }
    )
}

#[test]
fn process_valid_meta_package() {
    let toml_metadata = r"
mod_name = 'test_mod'
version = '0.1.0'
engine = 'rhai'
game_version = '1.0.0'
author_name = 'test_author'
short_desc = 'this is a test'
dependencies = [
    'other_test',
    'yet_another_test',
   ]
";
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![process_data::Chunk {
            four_cc: process_data::FourCC::Metadata,
            payload: toml_metadata.as_bytes(),
        }],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package,
        process_data::Package {
            meta: process_data::Metadata {
                mod_name: "test_mod".to_string(),
                version: "0.1.0".to_string(),
                engine: "rhai".to_string(),
                game_version: "1.0.0".to_string(),
                author_name: "test_author".to_string(),
                short_desc: Some("this is a test".to_string()),
                dependencies: vec!["other_test".to_string(), "yet_another_test".to_string()],
            }
        }
    )
}
