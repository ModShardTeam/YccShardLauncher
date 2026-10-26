mod common;

use ysl_mod::package;

#[test]
fn reversible_valid_package_default_entry_point() {
    let package_to_write = common::TestPackageBuilder::new().build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn reversible_valid_package_custom_entry_point() {
    let package_to_write = common::TestPackageBuilder::new()
        .entry_point("ab_5c")
        .clear_rhai()
        .add_default_rhai("ab_5c")
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn reversible_valid_package_multiple_scripts() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai("ab_5c")
        .add_default_rhai("frt")
        .add_default_rhai("ghty")
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn reversible_valid_package_no_short_desc() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai("ab_5c")
        .clear_short_desc()
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn reversible_valid_package_empty_rhai() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_rhai("ab_5c", "")
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn reversible_valid_package_with_dependencies() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_dependencies("dep0", "=1.0")
        .add_dependencies("dep1", "~0.1")
        .add_dependencies("dep2", ">2.7")
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn invalid_flags() {
    let package_to_write = common::TestPackageBuilder::new().build();

    let rej = package::write::write(&package_to_write, 190).unwrap_err();
    assert_eq!(rej, package::PackageRejection::UnknownBits { found: 190 })
}

#[test]
fn non_ascii_declared_entry_point() {
    let package_to_write = common::TestPackageBuilder::new()
        .entry_point("moš")
        .clear_rhai()
        .add_default_rhai("moš")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();

    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Invalid {
                name: "moš".to_string(),
                reason: "character 'š' found".to_string()
            }
        ))
    )
}

#[test]
fn uppercase_declared_entry_point() {
    let package_to_write = common::TestPackageBuilder::new()
        .entry_point("AB_5C")
        .clear_rhai()
        .add_default_rhai("ab_5c")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();

    assert_eq!(
        rej,
        package::PackageRejection::Entry(package::EntryRejection::Name(
            package::NameRejection::Invalid {
                name: "AB_5C".to_string(),
                reason: "character 'A' found".to_string()
            }
        ))
    )
}

#[test]
fn missing_custom_entry_point() {
    let package_to_write = common::TestPackageBuilder::new()
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
    let package_to_write = common::TestPackageBuilder::new().clear_rhai().build();

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
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai("")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Empty
        ))
    );
}

#[test]
fn rhai_non_ascii_name() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai("moš")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "moš".to_string(),
                reason: "character 'š' found".to_string()
            }
        ))
    );
}

#[test]
fn rhai_uppercase_name() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai("MoD")
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::Invalid {
                name: "MoD".to_string(),
                reason: "character 'M' found".to_string()
            }
        ))
    )
}

#[test]
fn rhai_name_max_len() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai(&"m".repeat(255))
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn rhai_name_not_u8() {
    let package_to_write = common::TestPackageBuilder::new()
        .add_default_rhai(&"m".repeat(256))
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Rhai(package::RhaiRejection::Name(
            package::NameRejection::TooLong {
                name: "m".repeat(256),
                len: 256
            }
        ))
    )
}

#[test]
fn rhai_bigger_than_10kib() {
    let package_to_write = common::TestPackageBuilder::new()
        .clear_rhai()
        .add_rhai("mod", &"a".repeat(10 * 1024 - 7))
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Chunk(package::ChunkRejection::TooBigChunk {
            declared: 10 * 1024 + 1
        })
    )
}

#[test]
fn rhai_maximum_size() {
    let package_to_write = common::TestPackageBuilder::new()
        .clear_rhai()
        .add_rhai("mod", &"a".repeat(10 * 1024 - 8))
        .build();

    let bytes = package::write::write(&package_to_write, 0).expect("valid operation");
    let read_package = package::read::read(&bytes).expect("valid operation");
    assert_eq!(package_to_write, read_package)
}

#[test]
fn package_too_big() {
    let mut builder = common::TestPackageBuilder::new();

    for i in 0..100 {
        builder.add_rhai(&format!("mod{i}"), &"a".repeat(9 * 1024));
    }
    let package_to_write = builder.build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::TooBigFile { declared: 923191 }
    )
}

#[test]
fn meta_too_big() {
    let package_to_write = common::TestPackageBuilder::new()
        .short_desc(&"a".repeat(11 * 1024))
        .build();

    let rej = package::write::write(&package_to_write, 0).unwrap_err();
    assert_eq!(
        rej,
        package::PackageRejection::Chunk(package::ChunkRejection::TooBigChunk { declared: 11407 })
    )
}
