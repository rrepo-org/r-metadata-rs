//! Regression coverage for lossless DESCRIPTION/PACKAGES record conversions.

use r_description::Description;
use r_packages::{PackageRecord, Packages, RecordConversionError};

#[test]
fn conversions_preserve_all_fields_and_exact_record_text() {
    let source = "Package: first\r\npackage: lowercase\r\nPackage:\tlast  \r\nVersion: 1.0\r\nDepends: R (>= 4.0), foo\r\nImports: bar\r\nX-Custom: café\r\n\tcontinued\r\n .\r\n  final line\r\nX-Custom: second\r\nEmpty:";
    let original = Description::parse(source);
    let record = PackageRecord::try_from(&original).unwrap();
    assert_eq!(record.to_string(), source);
    assert_eq!(record.fields("Package").count(), 2);
    assert_eq!(record.fields("X-Custom").count(), 2);
    assert_eq!(record.field("package").unwrap().as_str(), "lowercase");
    for field in original.fields_all() {
        let name = field.name().unwrap();
        assert_eq!(
            record.field(&name).unwrap(),
            original.field(&name).unwrap().value()
        );
    }

    let converted = Description::from(&record);
    assert_eq!(converted.to_string(), source);
    assert_eq!(converted.package(), original.package());
    assert_eq!(converted.fields("X-Custom").count(), 2);
    assert_eq!(Description::from(record).to_string(), source);
    assert_eq!(
        PackageRecord::try_from(original).unwrap().to_string(),
        source
    );
}

#[test]
fn selected_record_excludes_neighbors_and_document_blank_lines() {
    let source = "Package: selected\nX-Custom: kept\n";
    let packages = Packages::parse(&format!(
        "\nPackage: before\n\n{source}\n \t\nPackage: after\n\n"
    ));
    let description = Description::from(packages.record(1).unwrap());
    assert_eq!(description.to_string(), source);
    assert_eq!(description.records().count(), 1);

    let padded = Description::parse(&format!("\r\n \t\r\n{source}\n\n"));
    let record = PackageRecord::try_from(padded).unwrap();
    assert_eq!(record.to_string(), source);
    assert_eq!(Description::from(record).to_string(), source);
}

#[test]
fn malformed_metadata_survives_both_directions_without_validation() {
    for source in [
        " orphan\nbroken\nBad Name: value\nVersion: nope\nDepends: bad (>no)\nNeedsCompilation: perhaps\nX-Custom: kept\n",
        "broken",
        "X-Only: custom\r",
    ] {
        let description = Description::parse(source);
        let record = PackageRecord::try_from(&description).unwrap();
        assert_eq!(record.to_string(), source);
        assert!(record.package().is_none());
        let converted = Description::from(record);
        assert_eq!(converted.to_string(), source);
        assert_eq!(converted.diagnostics(), description.diagnostics());
        assert!(converted.title().is_none());
    }
}

#[test]
fn syntax_diagnostics_are_relative_to_the_selected_record() {
    let source = " orphan\nbroken\nBad Name: value\n";
    let packages = Packages::parse(&format!("Package: first\n\n{source}"));
    let description = Description::from(packages.record(1).unwrap());
    let expected = Description::parse(source);
    assert_eq!(description.diagnostics().len(), 3);
    assert_eq!(description.diagnostics(), expected.diagnostics());
    assert_eq!(description.diagnostics()[0].span().start, 0);
}

#[test]
fn rejects_documents_that_do_not_contain_exactly_one_record() {
    for source in ["", "\n\r\n \t\n", "\u{000c}"] {
        let description = Description::parse(source);
        assert_eq!(
            PackageRecord::try_from(&description),
            Err(RecordConversionError::NoRecords)
        );
        assert_eq!(
            PackageRecord::try_from(description),
            Err(RecordConversionError::NoRecords)
        );
    }
    for source in ["Package: first\n\nPackage: second", "broken\n\n orphan\n"] {
        let description = Description::parse(source);
        assert_eq!(
            PackageRecord::try_from(&description),
            Err(RecordConversionError::MultipleRecords)
        );
        assert_eq!(
            PackageRecord::try_from(description),
            Err(RecordConversionError::MultipleRecords)
        );
    }
}
