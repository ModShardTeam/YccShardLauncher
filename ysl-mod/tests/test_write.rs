mod common;
use std::collections::BTreeMap;

use ysl_mod::{package, raw};

#[test]
fn write_valid_package_file() {
    let package = package::Package {
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

    let file = package::write::write(package, 0).expect("valid operation");
    assert_eq!(
        file,
        raw::WFile {
            format_version: 1,
            flags: 0,
            chunks: vec![
                raw::WChunk {
                    four_cc: *common::ID_META,
                    payload: common::dummy_metadata("ab_5c").as_bytes().to_vec(),
                },
                raw::WChunk {
                    four_cc: *common::ID_RHAI,
                    payload: [vec![5], b"ab_5c".to_vec(), b"fn load(mod) { }".to_vec()].concat(),
                }
            ],
        },
    )
}

#[test]
fn reversible_valid_package() {
    let package = package::Package {
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

    let wfile = package::write::write(package.clone(), 0).expect("valid operation");
    let p = package::read::read((&wfile).into()).expect("valid operation");
    assert_eq!(package, p)
}
