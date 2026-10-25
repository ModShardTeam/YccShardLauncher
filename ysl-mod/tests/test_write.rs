mod common;

use ysl_mod::package;

#[test]
fn invalid_serialization_metadata() {
    /* TODO: find a way to do this test, but I believe I cant */
}

#[test]
fn reversible_valid_package_to_bytes() {
    let package_to_write = common::PackageBuilder::new()
        .entry_point("ab_5c")
        .clear_rhai()
        .add_rhai("ab_5c")
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn invalid_flags() {
    let package_to_write = common::PackageBuilder::new().build();

    let rej = package::write::write(&package_to_write, 190).unwrap_err();
    assert_eq!(rej, package::PackageRejection::UnknownBits { found: 190 })
}

#[test]
fn non_ascii_declared_entry_point() {
    let package_to_write = common::PackageBuilder::new()
        .entry_point("moš")
        .clear_rhai()
        .add_rhai("moš")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();

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
fn uppercase_declared_entry_point() {
    let package_to_write = common::PackageBuilder::new()
        .entry_point("AB_5C")
        .clear_rhai()
        .add_rhai("ab_5c")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();

    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Invalid {
                name: "AB_5C".to_string(),
                reason: "Invalid character 'A' found".to_string()
            }
        ))
    )
}

#[test]
fn missing_custom_entry_point() {
    let package_to_write = common::PackageBuilder::new()
        .entry_point("ab_5c")
        .clear_rhai()
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::None {
            expected: "ab_5c".to_string()
        })
    );
}

#[test]
fn missing_default_entry_point() {
    let package_to_write = common::PackageBuilder::new().clear_rhai().build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::None {
            expected: "mod".to_string()
        })
    );
}

#[test]
fn rhai_null_name() {
    let package_to_write = common::PackageBuilder::new().add_rhai("").build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(package::NameRejection::Null))
    );
}

#[test]
fn rhai_non_ascii_name() {
    let package_to_write = common::PackageBuilder::new().add_rhai("moš").build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "moš".to_string(),
                reason: "Invalid character 'š' found".to_string()
            }
        ))
    );
}

#[test]
fn rhai_uppercase_name() {
    let package_to_write = common::PackageBuilder::new().add_rhai("MoD").build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "MoD".to_string(),
                reason: "Invalid character 'M' found".to_string()
            }
        ))
    )
}
