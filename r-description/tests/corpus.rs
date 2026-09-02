//! Real-world DESCRIPTION corpus coverage.

use r_description::Description;

#[test]
fn real_description_roundtrips_and_exposes_custom_fields() {
    let source = include_str!("data/real-description.dcf");
    let document = Description::parse(source);

    assert_eq!(document.to_string(), source);
    assert!(document.diagnostics().is_empty());
    assert_eq!(document.package().unwrap().as_str(), "dplyr");
    assert_eq!(
        document
            .field("Config/testthat/edition")
            .unwrap()
            .value()
            .as_str(),
        "3"
    );
    assert!(document.validate().is_valid());
}
