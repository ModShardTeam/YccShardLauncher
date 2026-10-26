mod common;
use std::{assert_matches, collections::BTreeMap};

use ysl_mod::package;

#[test]
fn reversible_read_write_from_lib() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .build();

    let package = ysl_mod::read(&bytes).expect("valid operation");
    let written_bytes = ysl_mod::write(&package, 0).expect("valid operation");
    let read_package = ysl_mod::read(&written_bytes).expect("valid operation");

    assert_eq!(package, read_package);
    assert_eq!(bytes, written_bytes);
}

#[test]
fn reversible_read_write() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
    let written_bytes = package::write::write(&package, 0).expect("valid operation");
    let read_package = package::read::read(&written_bytes).expect("valid operation");

    assert_eq!(package, read_package);
    assert_eq!(bytes, written_bytes);
}

#[test]
fn bad_magic_package() {
    let bytes = common::TestBytesBuilder::new()
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
    let bytes = common::TestBytesBuilder::new().format_version(0).build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnsupportedVersion { found: 0 }
    );
}

#[test]
fn version_from_the_future_package() {
    let bytes = common::TestBytesBuilder::new().format_version(u16::MAX).build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::UnsupportedVersion { found: u16::MAX }
    );
}

#[test]
fn unknown_low_byte_flag_package() {
    let bytes = common::TestBytesBuilder::new()
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
    let bytes = common::TestBytesBuilder::new()
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .len(250)
        .add_arbitrary_chunk(&common::TestChunkBuilder::new().id(b"TEST").build())
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .len(2)
        .add_arbitrary_chunk(&common::TestChunkBuilder::new().id(b"TEST").build())
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(None, "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
                .id(b"NOT\0")
                .payload(&[0xB0, 0xB1, 0xB2])
                .build(),
        )
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
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
                version: semver::Version::new(1, 0, 0),
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(None, "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
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
            available: 5
        })
    );
}

#[test]
fn overrun_chunk_package() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(None, "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
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
            declared: 100,
            available: 3
        })
    );
}

#[test]
fn valid_meta_package() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(1, 0, 0),
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_default_rhai("mod")
        .build();

    let rej: package::PackageRejection = package::read::read(&bytes).unwrap_err();
    assert_eq!(rej, package::PackageRejection::NoMetaChunk,)
}

#[test]
fn two_metas_package() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .build();

    let rej: package::PackageRejection = package::read::read(&bytes).unwrap_err();
    assert_eq!(rej, package::PackageRejection::MultipleMetaChunkFound,)
}

#[test]
fn valid_rhai_package() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("ab_5c"), "1.0.0", "=1.0")
        .add_default_rhai("ab_5c")
        .build();

    let package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(
        package.meta,
        package::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(1, 0, 0),
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(package::NameRejection::Null))
    )
}

#[test]
fn reserved_device_rhai_name_package() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_default_rhai("con")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("y;u")
        .add_default_rhai("mod")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_default_rhai("YUU")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some(""), "1.0.0", "=1.0")
        .add_default_rhai("mod")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("moš"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("ab_5c"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("AB_5C"), "1.0.0", "=1.0")
        .add_default_rhai("ab_5c")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(None, "1.0.0", "=1.0")
        .add_default_rhai("ab_5c")
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
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_default_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::DuplicateName {
            found: "mod".to_string(),
        })
    )
}

#[test]
fn unsupported_lua() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_arbitrary_chunk(&common::TestChunkBuilder::new().id(b"LUA\0").build())
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::KnownButUnsupportedId {
            found: package::FourCC::LuaScript,
        }
    )
}

#[test]
fn unsupported_png() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=1.0")
        .add_default_rhai("mod")
        .add_arbitrary_chunk(&common::TestChunkBuilder::new().id(b"PNG\0").build())
        .build();

    let rej = package::read::read(&bytes).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::KnownButUnsupportedId {
            found: package::FourCC::Png,
        }
    )
}

#[test]
fn invalid_version() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0", "=1.0")
        .add_default_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();

    assert_matches!(rej, package::PackageRejection::InvalidDeserMetadata(_));
    let package::PackageRejection::InvalidDeserMetadata(deser) = rej else {
        unreachable!()
    };
    assert_eq!(
        deser.message(),
        "unexpected end of input while parsing minor version number"
    )
}

#[test]
fn invalid_req() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_meta(Some("mod"), "1.0.0", "=>1")
        .add_default_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();

    assert_matches!(rej, package::PackageRejection::InvalidDeserMetadata(_));
    let package::PackageRejection::InvalidDeserMetadata(deser) = rej else {
        unreachable!()
    };
    assert_eq!(
        deser.message(),
        "unexpected character '>' while parsing major version number"
    )
}

#[test]
fn missing_required_fields() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
                .id(common::ID_META)
                .payload(b"mod_name = 'test_mod'")
                .build(),
        )
        .add_default_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();

    assert_matches!(rej, package::PackageRejection::InvalidDeserMetadata(_));
    let package::PackageRejection::InvalidDeserMetadata(deser) = rej else {
        unreachable!()
    };
    assert_eq!(deser.message(), "missing field `version`")
}

#[test]
fn unknown_field() {
    let bytes = common::TestBytesBuilder::new()
        .format_version(1)
        .add_arbitrary_chunk(
            &common::TestChunkBuilder::new()
                .id(common::ID_META)
                .payload(
                    br#"mod_name = "test_mod"
version = "1.0.0"
engine = "rhai"
game_version = "1.0.0"
author_name = "test_author"
short_desc = "this is a test"
extrafield = "test"

[dependencies]
yet_another_test = ">0.2"

"#,
                )
                .build(),
        )
        .add_default_rhai("mod")
        .build();

    let rej = package::read::read(&bytes).unwrap_err();

    assert_matches!(rej, package::PackageRejection::InvalidDeserMetadata(_));
    let package::PackageRejection::InvalidDeserMetadata(deser) = rej else {
        unreachable!()
    };
    assert_eq!(
        deser.message(),
        "unknown field `extrafield`, expected one of `mod_name`, `version`, `engine`, `entry_point`, `game_version`, `author_name`, `short_desc`, `dependencies`"
    )
}
