mod common;
use std::collections::BTreeMap;

use ysl_mod::package;

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

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
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

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");

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
    assert_eq!(
        rej,
        package::PackageRejection::RhaiRejection(package::RhaiRejection::NullName)
    );
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
        package::PackageRejection::RhaiRejection(package::RhaiRejection::InvalidByteName {
            name: "moš".as_bytes().to_vec(),
            reason: "Invalid character 'š' found in name".to_string()
        })
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

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
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
