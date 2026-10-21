mod common;
use std::collections::BTreeMap;

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
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![process_data::Chunk {
            four_cc: process_data::FourCC::Metadata,
            payload: common::dummy_metadata().as_bytes(),
        }],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package,
        process_data::Package {
            meta: process_data::Metadata {
                mod_name: "test_mod".to_string(),
                version: semver::Version::new(0, 1, 0),
                engine: "rhai".to_string(),
                game_version: "1.0.0".to_string(),
                author_name: "test_author".to_string(),
                short_desc: Some("this is a test".to_string()),
                dependencies: BTreeMap::from([
                    (
                        "other_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Exact,
                                major: 1,
                                minor: Some(0),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    ),
                    (
                        "yet_another_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Greater,
                                major: 0,
                                minor: Some(2),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    )
                ]),
            },
            rhai_scripts: vec![],
        }
    )
}

#[test]
fn convert_no_meta_package() {
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![process_data::Chunk {
            four_cc: process_data::FourCC::RhaiScript,
            payload: &[0xC0, 0xC1, 0xC2, 0xC3],
        }],
    };

    let rej: process_data::PackageRejection = process_data::process_package(file).unwrap_err();
    assert_eq!(rej, process_data::PackageRejection::NoMetaChunk,)
}

#[test]
fn convert_two_metas_package() {
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_metadata().as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_metadata().as_bytes(),
            },
        ],
    };

    let rej: process_data::PackageRejection = process_data::process_package(file).unwrap_err();
    assert_eq!(rej, process_data::PackageRejection::MultipleMetaChunkFound,)
}

#[test]
fn process_valid_rhai_package() {
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![process_data::Chunk {
            four_cc: process_data::FourCC::Metadata,
            payload: common::dummy_metadata().as_bytes(),
        },
        process_data::Chunk {
            four_cc: process_data::FourCC::RhaiScript,
            payload: "fn load(mod) { mod.load_script(\"gml_Object_o_player_Step_0\").replace_constant(250.0, 10.0).save(); }".as_bytes(),
        }],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package,
        process_data::Package {
            meta: process_data::Metadata {
                mod_name: "test_mod".to_string(),
                version: semver::Version::new(0, 1, 0),
                engine: "rhai".to_string(),
                game_version: "1.0.0".to_string(),
                author_name: "test_author".to_string(),
                short_desc: Some("this is a test".to_string()),
                dependencies: BTreeMap::from([
                    (
                        "other_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Exact,
                                major: 1,
                                minor: Some(0),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    ),
                    (
                        "yet_another_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Greater,
                                major: 0,
                                minor: Some(2),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    )
                ]),
            },
            rhai_scripts: vec!["fn load(mod) { mod.load_script(\"gml_Object_o_player_Step_0\").replace_constant(250.0, 10.0).save(); }".to_string()],
        }
    )
}

#[test]
fn process_valid_compressed_rhai_package() {
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 1,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_metadata().as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &[
                    0x28, 0xb5, 0x2f, 0xfd, 0x0, 0x58, 0x81, 0x0, 0x0, 0x66, 0x6e, 0x20, 0x6c,
                    0x6f, 0x61, 0x64, 0x28, 0x6d, 0x6f, 0x64, 0x29, 0x20, 0x7b, 0x20, 0x7d,
                ],
            },
        ],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package,
        process_data::Package {
            meta: process_data::Metadata {
                mod_name: "test_mod".to_string(),
                version: semver::Version::new(0, 1, 0),
                engine: "rhai".to_string(),
                game_version: "1.0.0".to_string(),
                author_name: "test_author".to_string(),
                short_desc: Some("this is a test".to_string()),
                dependencies: BTreeMap::from([
                    (
                        "other_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Exact,
                                major: 1,
                                minor: Some(0),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    ),
                    (
                        "yet_another_test".to_string(),
                        semver::VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Greater,
                                major: 0,
                                minor: Some(2),
                                patch: None,
                                pre: semver::Prerelease::EMPTY
                            }]
                        }
                    )
                ]),
            },
            rhai_scripts: vec!["fn load(mod) { }".to_string()],
        }
    )
}
