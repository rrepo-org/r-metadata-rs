//! Real-world PACKAGES corpus coverage.

use r_packages::Packages;

#[test]
fn real_packages_index_roundtrips_and_validates() {
    let source = include_str!("../../testdata/real-packages.dcf");
    let packages = Packages::parse(source);

    assert_eq!(packages.to_string(), source);
    assert_eq!(packages.len(), 2);
    assert_eq!(
        packages.record(1).unwrap().package().unwrap().as_str(),
        "beta"
    );
    assert!(packages.validate().is_empty());
}
