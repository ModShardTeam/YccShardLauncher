mod common;
use std::collections::BTreeMap;

use ysl_mod::package;

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
