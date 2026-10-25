mod common;
use std::collections::BTreeMap;

use ysl_mod::package;

#[test]
fn bad_magic_package() {
    let bytes = common::BytesBuilder::new()
        .magic(*b"BAD\0")
        .format_version(1)
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::BadMagic { found: *b"BAD\0" }
    );
}

#[test]
fn null_version_package() {
    let bytes = common::BytesBuilder::new().format_version(0).build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnsupportedVersion { found: 0 }
    );
}

#[test]
fn version_from_the_future_package() {
    let bytes = common::BytesBuilder::new().format_version(u16::MAX).build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnsupportedVersion { found: u16::MAX }
    );
}

#[test]
fn unknown_low_byte_flag_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .flags(0b0000_0000_1100_0000)
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnknownBits {
            found: 0b0000_0000_1100_0000
        }
    );
}

#[test]
fn unknown_high_byte_flag_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .flags(0b0000_1101_0000_0000)
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnknownBits {
            found: 0b0000_1101_0000_0000
        }
    );
}

#[test]
fn parse_too_long_len_file() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .len(250)
        .add_arbitrary_chunk(&common::ChunkBuilder::new().id(b"TEST").build())
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::InvalidSizeFile {
            declared: 250,
            available: 8
        }
    );
}

#[test]
fn parse_too_small_len_file() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .len(2)
        .add_arbitrary_chunk(&common::ChunkBuilder::new().id(b"TEST").build())
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::InvalidSizeFile {
            declared: 2,
            available: 8
        }
    );
}

#[test]
fn unknown_chunk_id_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(None)
        .add_rhai("mod")
        .add_arbitrary_chunk(
            &common::ChunkBuilder::new()
                .id(b"NOT\0")
                .payload(&[0xB0, 0xB1, 0xB2])
                .build(),
        )
        .add_arbitrary_chunk(
            &common::ChunkBuilder::new()
                .id(b"TEST")
                .payload(&[0xC0, 0xC1, 0xC2])
                .build(),
        )
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(
        package,
        package::Package {
            meta: package::Metadata {
                mod_name: "test_mod".to_string(),
                version: semver::Version::parse("0.1.0").expect("valid operation"),
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
            rhai_scripts: BTreeMap::from([("mod".to_string(), "fn load(mod) { }".to_string())])
        }
    );
}

#[test]
fn partial_header_chunk_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(None)
        .add_rhai("mod")
        .add_arbitrary_chunk(
            &common::ChunkBuilder::new()
                .id(common::ID_RHAI)
                .arbitrary_len(&[0x01])
                .build(),
        )
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Chunk(package::ChunkRejection::NotEnoughDataHeader {
            needed: 8,
            available: 4
        })
    );
}

#[test]
fn overrun_chunk_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(None)
        .add_rhai("mod")
        .add_arbitrary_chunk(
            &common::ChunkBuilder::new()
                .id(common::ID_RHAI)
                .len(100)
                .payload(&[0x01, b'a', 0x01])
                .build(),
        )
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Chunk(package::ChunkRejection::NotEnoughDataPayload {
            declared: 10,
            available: 3
        })
    );
}

#[test]
fn valid_meta_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("mod")
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: Some("mod".to_string()),
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
        BTreeMap::from([("mod".to_string(), "fn load(mod) { }".to_string())])
    )
}

#[test]
fn no_meta_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_rhai("mod")
        .build();

    let rej: package::PackageRejection = package::read::read(&bytes).unwrap_err();
    assert_eq!(rej, package::PackageRejection::NoMetaChunk,)
}

#[test]
fn two_metas_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_meta(Some("mod"))
        .add_rhai("mod")
        .build();

    let rej: package::PackageRejection = package::read::read(&bytes).unwrap_err();
    assert_eq!(rej, package::PackageRejection::MultipleMetaChunkFound,)
}

#[test]
fn valid_rhai_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("ab_5c"))
        .add_rhai("ab_5c")
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
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
fn null_rhai_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(package::NameRejection::Null))
    )
}

#[test]
fn reserved_device_rhai_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("mod")
        .add_rhai("con")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "con".to_string(),
                reason: "Reserved device name".to_string()
            }
        ))
    )
}

#[test]
fn invalid_char_rhai_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("y;u")
        .add_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "y;u".to_string(),
                reason: "Invalid character ';' found".to_string()
            }
        ))
    )
}

#[test]
fn uppercase_char_rhai_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("mod")
        .add_rhai("YUU")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "YUU".to_string(),
                reason: "Invalid character 'Y' found".to_string()
            }
        ))
    )
}

#[test]
fn empty_entry_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some(""))
        .add_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();

    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Null
        ))
    )
}

#[test]
fn invalid_entry_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("moš"))
        .add_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Invalid {
                name: "moš".to_string(),
                reason: "Invalid character 'š' found".to_string()
            }
        ))
    )
}

#[test]
fn missing_entry_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("ab_5c"))
        .add_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::None {
            expected: "ab_5c".to_string()
        })
    )
}

#[test]
fn uppercase_entry_name_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("AB_5C"))
        .add_rhai("ab_5c")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Invalid {
                name: "AB_5C".to_string(),
                reason: "Invalid character 'A' found".to_string()
            }
        ))
    );
}

#[test]
fn neither_entry_name_nor_mod_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(None)
        .add_rhai("ab_5c")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::None {
            expected: "mod".to_string()
        })
    )
}

#[test]
fn duplicate_names_rhai_script_package() {
    let bytes = common::BytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"))
        .add_rhai("mod")
        .add_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::DuplicateName {
            found: "mod".to_string(),
        })
    )
}
