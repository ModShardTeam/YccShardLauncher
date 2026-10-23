mod common;
use std::collections::BTreeMap;

use ysl_mod::package;
use ysl_mod::raw;

#[test]
fn convert_skips_unknown_id() {
    let raw_file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            raw::Chunk {
                four_cc: *b"BAD\0",
                payload: &[0xB0, 0xB1, 0xB2, 0xB3],
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &[0x03, b'm', b'o', b'd'],
            },
        ],
    };

    let package = package::read::read(raw_file).expect("valid operation");
    assert_eq!(
        package.rhai_scripts,
        BTreeMap::from([("mod".to_string(), "".to_string())])
    )
}

#[test]
fn process_valid_meta_package() {
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &[0x03, b'm', b'o', b'd'],
            },
        ],
    };

    let package = package::read::read(file).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
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
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![raw::Chunk {
            four_cc: *common::ID_RHAI,
            payload: &[0xC0, 0xC1, 0xC2, 0xC3],
        }],
    };

    let rej: package::PackageRejection = package::read::read(file).unwrap_err();
    assert_eq!(rej, package::PackageRejection::NoMetaChunk,)
}

#[test]
fn convert_two_metas_package() {
    let f_binding = common::dummy_metadata("mod");
    let s_binding = common::dummy_metadata("mod");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: f_binding.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: s_binding.as_bytes(),
            },
        ],
    };

    let rej: package::PackageRejection = package::read::read(file).unwrap_err();
    assert_eq!(rej, package::PackageRejection::MultipleMetaChunkFound,)
}

#[test]
fn process_valid_rhai_package() {
    let meta_payload = common::dummy_metadata("ab_5c");
    let rhai_payload = common::dummy_rhai_payload(b"ab_5c");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let package = package::read::read(file).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
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
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(rej, package::PackageRejection::RhaiNullName)
}

#[test]
fn process_reserved_device_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let rhai_payload = common::dummy_rhai_payload(b"con");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::RhaiInvalidName {
            name: "con".to_string(),
            reason: "Reserved device name".to_string()
        }
    )
}

#[test]
fn process_invalid_char_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let rhai_payload = common::dummy_rhai_payload(b"y;u");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::RhaiInvalidByteName {
            name: b"y;u".to_vec(),
            reason: "Invalid character ';' (0x3b) found in name".to_string()
        }
    )
}

#[test]
fn process_uppercase_char_rhai_name_package() {
    let meta_payload = common::dummy_metadata("mod");
    let rhai_payload = common::dummy_rhai_payload(b"YUU");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::RhaiInvalidByteName {
            name: b"YUU".to_vec(),
            reason: "Invalid character 'Y' (0x59) found in name".to_string()
        }
    )
}

#[test]
fn process_empty_entry_name_package() {
    let meta_payload = common::dummy_metadata("");
    let rhai_payload = common::dummy_rhai_payload(b"mod");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();

    let package::PackageRejection::InvalidMetadata(e) = rej else {
        panic!("expected InvalidMetadata, got {rej:?}");
    };
    assert_eq!(e.message(), "empty entry point")
}

#[test]
fn process_invalid_entry_name_package() {
    let meta_payload = common::dummy_metadata("moš");
    let rhai_payload = common::dummy_rhai_payload(b"mod");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();

    let package::PackageRejection::InvalidMetadata(e) = rej else {
        panic!("expected InvalidMetadata, got {rej:?}");
    };
    assert_eq!(e.message(), "Invalid character 'š' found in entry_name")
}

#[test]
fn process_missing_entry_name_package() {
    let meta_payload = common::dummy_metadata("ab_5c");
    let rhai_payload = common::dummy_rhai_payload(b"mod");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::NoEntryPoint {
            expected: "ab_5c".to_string()
        }
    )
}

#[test]
fn process_wrong_case_entry_name_package() {
    let meta_payload = common::dummy_metadata("AB_5C");
    let rhai_payload = common::dummy_rhai_payload(b"ab_5c");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: meta_payload.as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let package = package::read::read(file).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
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
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &rhai_payload,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::NoEntryPoint {
            expected: "mod".to_string()
        }
    )
}

#[test]
fn process_duplicate_names_rhai_script_package() {
    let f_binding = common::dummy_rhai_payload(b"mod");
    let s_binding = common::dummy_rhai_payload(b"mod");
    let file = raw::File {
        format_version: 1,
        flags: 0,
        chunks: vec![
            raw::Chunk {
                four_cc: *common::ID_META,
                payload: common::dummy_no_entry_metadata().as_bytes(),
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &f_binding,
            },
            raw::Chunk {
                four_cc: *common::ID_RHAI,
                payload: &s_binding,
            },
        ],
    };

    let rej = package::read::read(file).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::RhaiDuplicateName {
            found: "mod".to_string(),
        }
    )
}
