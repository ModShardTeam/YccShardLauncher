mod common;
use std::collections::BTreeMap;

use ysl_mod::{package, raw};

#[test]
fn write_valid_package_file() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                            pre: semver::Prerelease::EMPTY,
                        }],
                    },
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq {
                        comparators: vec![semver::Comparator {
                            op: semver::Op::Greater,
                            major: 0,
                            minor: Some(2),
                            patch: None,
                            pre: semver::Prerelease::EMPTY,
                        }],
                    },
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string())]),
    };

    let file = package::write::write(&package_to_write, 0).expect("valid operation");
    assert_eq!(
        file,
        raw::FileBuf {
            format_version: 1,
            flags: 0,
            chunks: vec![
                raw::ChunkBuf {
                    four_cc: *common::ID_META,
                    payload: common::dummy_metadata("ab_5c").as_bytes().to_vec(),
                },
                raw::ChunkBuf {
                    four_cc: *common::ID_RHAI,
                    payload: [vec![5], b"ab_5c".to_vec(), b"fn load(mod) { }".to_vec()].concat(),
                }
            ],
        },
    )
}

#[test]
fn reversible_valid_package_to_file() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string())]),
    };

    let file_buff = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read((&file_buff).into()).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn invalid_serialization_metadata() {
    /* TODO: find a way to do this test, but I believe I cant */
}

#[test]
fn reversible_valid_package_to_bytes() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string())]),
    };

    let file_buff = package::write::write(&package_to_write, 0).expect("valid operation");
    let bytes = raw::write::write_file(&file_buff).expect("valid operation");

    let file = raw::read::parse_file(&bytes).expect("valid operation");
    let read_package = package::read::read(file).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn non_ascii_declared_entry_point() {
    let package_to_write = package::Package {
        meta: package::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: Some("moš".to_string()),
            game_version: "1.0.0".to_string(),
            author_name: "test_author".to_string(),
            short_desc: Some("this is a test".to_string()),
            dependencies: BTreeMap::from([
                (
                    "other_test".to_string(),
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("moš".to_string(), "fn load(mod) { }".to_string())]),
    };

    let rej = package::write::write(&package_to_write, 0).unwrap_err();

    let package::PackageRejection::InvalidSerMetadata(e) = rej else {
        panic!("expected InvalidDeserMetadata, got {rej:?}");
    };
    assert_eq!(
        format!("{:?}", e),
        "Invalid character 'š' found in entry_name"
    )
}

#[test]
fn uppercase_declared_entry_point() {
    let package_to_write = package::Package {
        meta: package::Metadata {
            mod_name: "test_mod".to_string(),
            version: semver::Version::new(0, 1, 0),
            engine: "rhai".to_string(),
            entry_point: Some("AB_5C".to_string()),
            game_version: "1.0.0".to_string(),
            author_name: "test_author".to_string(),
            short_desc: Some("this is a test".to_string()),
            dependencies: BTreeMap::from([
                (
                    "other_test".to_string(),
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string())]),
    };

    let file_buff = package::write::write(&package_to_write, 0).expect("valid operation");
    let bytes = raw::write::write_file(&file_buff).expect("valid operation");

    let file = raw::read::parse_file(&bytes).expect("valid operation");
    let read_package = package::read::read(file).expect("valid operation");

    assert_eq!(
        read_package,
        package::Package {
            meta: package::Metadata {
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
                        semver::VersionReq::parse("=1.0").unwrap(),
                    ),
                    (
                        "yet_another_test".to_string(),
                        semver::VersionReq::parse(">0.2").unwrap(),
                    ),
                ]),
            },

            rhai_scripts: BTreeMap::from([("ab_5c".to_string(), "fn load(mod) { }".to_string())]),
        }
    )
}

#[test]
fn missing_custom_entry_point() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([]),
    };

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::NoEntryPoint {
            expected: "ab_5c".to_string()
        }
    );
}

#[test]
fn missing_default_entry_point() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([]),
    };

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::NoEntryPoint {
            expected: "mod".to_string()
        }
    );
}

#[test]
fn rhai_null_name() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([
            ("mod".to_string(), "fn load(mod) { }".to_string()),
            ("".to_string(), "fn load(mod) { }".to_string()),
        ]),
    };

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(rej, package::PackageRejection::RhaiNullName);
}

#[test]
fn rhai_non_ascii_name() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([
            ("mod".to_string(), "fn load(mod) { }".to_string()),
            ("moš".to_string(), "fn load(mod) { }".to_string()),
        ]),
    };

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        // TODO: Not the correct rejection, in the bytes -> Package it makes sense, not here
        package::PackageRejection::RhaiInvalidByteName {
            name: "moš".as_bytes().to_vec(),
            reason: "Invalid character 'š' found in name".to_string()
        }
    );
}

#[test]
fn rhai_uppercase_name() {
    let package_to_write = package::Package {
        meta: package::Metadata {
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
                    semver::VersionReq::parse("=1.0").unwrap(),
                ),
                (
                    "yet_another_test".to_string(),
                    semver::VersionReq::parse(">0.2").unwrap(),
                ),
            ]),
        },

        rhai_scripts: BTreeMap::from([("MoD".to_string(), "fn load(mod) { }".to_string())]),
    };

    let file_buff = package::write::write(&package_to_write, 0).expect("valid operation");
    let bytes = raw::write::write_file(&file_buff).expect("valid operation");

    let file = raw::read::parse_file(&bytes).expect("valid operation");
    let read_package = package::read::read(file).expect("valid operation");
    assert_eq!(
        read_package,
        package::Package {
            meta: package::Metadata {
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
                        semver::VersionReq::parse("=1.0").unwrap(),
                    ),
                    (
                        "yet_another_test".to_string(),
                        semver::VersionReq::parse(">0.2").unwrap(),
                    ),
                ]),
            },

            rhai_scripts: BTreeMap::from([("mod".to_string(), "fn load(mod) { }".to_string())]),
        }
    )
}
