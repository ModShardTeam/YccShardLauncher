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
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &[0x03, b'm', b'o', b'd'],
            },
        ],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package.meta,
        process_data::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: None,
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
    );
    assert_eq!(
        package.rhai_scripts,
        BTreeMap::from([("mod".to_string(), "".to_string())])
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
    let f_binding = common::dummy_metadata("mod");
    let s_binding = common::dummy_metadata("mod");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: f_binding.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: s_binding.as_bytes(),
            },
        ],
    };

    let rej: process_data::PackageRejection = process_data::process_package(file).unwrap_err();
    assert_eq!(rej, process_data::PackageRejection::MultipleMetaChunkFound,)
}

#[test]
fn process_valid_rhai_package() {
    let meta_payload = common::dummy_metadata("ab_5c");
    let rhai_payload = common::dummy_rhai_payload(b"ab_5c");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package.meta,
        process_data::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: Some("ab_5c".to_string()),
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
    );
    assert_eq!(
        package.rhai_scripts,
        BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string(),)])
    )
}

#[test]
fn process_null_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let mut rhai_payload: Vec<u8> = vec![];
    rhai_payload.push(0x00);
    rhai_payload.push(0x00);
    rhai_payload.extend_from_slice("fn load(mod) { }".as_bytes());
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(rej, process_data::PackageRejection::RhaiNullName)
}

#[test]
fn process_reserved_device_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let rhai_payload = common::dummy_rhai_payload(b"con");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(
        rej,
        process_data::PackageRejection::RhaiInvalidName {
            name: "con".to_string(),
            reason: "Reserved device name".to_string()
        }
    )
}

#[test]
fn process_invalid_char_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let rhai_payload = common::dummy_rhai_payload(b"Y;U");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(
        rej,
        process_data::PackageRejection::RhaiInvalidByteName {
            name: [89, 59, 85].to_vec(),
            reason: "Invalid character 'Y' (0x59) found in name".to_string()
        }
    )
}

#[test]
fn process_missing_entry_name_package() {
    let meta_payload = common::dummy_metadata("ab_5c");
    let rhai_payload = common::dummy_rhai_payload(b"mod");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(
        rej,
        process_data::PackageRejection::NoEntryPoint {
            expected: "ab_5c".to_string()
        }
    )
}

#[test]
fn process_wrong_case_entry_name_package() {
    let meta_payload = common::dummy_metadata("AB_5C");
    let rhai_payload = common::dummy_rhai_payload(b"ab_5c");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: meta_payload.as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let package = process_data::process_package(file).expect("valid operation");
    assert_eq!(
        package.meta,
        process_data::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: Some("ab_5c".to_string()),
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
    );
    assert_eq!(
        package.rhai_scripts,
        BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string(),)])
    )
}

#[test]
fn process_neither_entry_name_nor_mod_package() {
    let rhai_payload = common::dummy_rhai_payload(b"ab_5c");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(
        rej,
        process_data::PackageRejection::NoEntryPoint {
            expected: "mod".to_string()
        }
    )
}

#[test]
fn process_duplicate_names_rhai_script_package() {
    let f_binding = common::dummy_rhai_payload(b"mod");
    let s_binding = common::dummy_rhai_payload(b"mod");
    let file = process_data::File {
        header: process_data::FileHeader {
            format_version: 1,
            flags: 0,
        },
        chunks: vec![
            process_data::Chunk {
                four_cc: process_data::FourCC::Metadata,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &f_binding,
            },
            process_data::Chunk {
                four_cc: process_data::FourCC::RhaiScript,
                payload: &s_binding,
            },
        ],
    };

    let rej = process_data::process_package(file).unwrap_err();
    assert_eq!(
        rej,
        process_data::PackageRejection::RhaiDuplicateName {
            found: "mod".to_string(),
        }
    )
}
