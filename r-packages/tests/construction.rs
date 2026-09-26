//! Assembly of existing records without implicit transformations.

use r_description::{Description, LogicalValue};
use r_packages::{FormatStyle, LineEnding, PackageRecord, Packages, RecordBuilder};

fn record(text: &str) -> PackageRecord {
    PackageRecord::try_from(Description::parse(text)).unwrap()
}

#[test]
fn empty_and_single_inputs_preserve_document_boundaries() {
    let empty = Packages::from_records(Vec::<PackageRecord>::new());
    assert!(empty.is_empty());
    assert_eq!(empty.to_string(), "");
    assert_eq!(Packages::builder().build().to_string(), "");
    for ending in ["", "\n", "\r\n", "\r"] {
        let text = format!("Package: alpha\nVersion: 1.0{ending}");
        let input = record(&text);
        assert_eq!(Packages::from_records([&input]).to_string(), text);
        assert_eq!(Packages::from_records([input]).to_string(), text);
    }
}

#[test]
fn all_boundary_combinations_keep_records_and_values_separate() {
    for first_ending in ["", "\n", "\r\n", "\r"] {
        for last_ending in ["", "\n", "\r\n", "\r"] {
            for line_ending in [LineEnding::Lf, LineEnding::CrLf, LineEnding::Cr] {
                let first = format!("X-First: café  {first_ending}");
                let last = format!("X-Last:\t東京{last_ending}");
                let inputs = [record(&first), record(&last)];
                let packages = Packages::builder()
                    .existing_records(&inputs)
                    .format_style(FormatStyle {
                        line_ending,
                        ..FormatStyle::default()
                    })
                    .build();
                let separator = if first_ending.is_empty() {
                    line_ending.as_str().repeat(2)
                } else {
                    first_ending.to_owned()
                };
                assert_eq!(packages.to_string(), format!("{first}{separator}{last}"));
                assert_eq!(packages.len(), 2);
                for (actual, expected) in packages.records().zip(&inputs) {
                    assert_eq!(
                        actual
                            .all_fields()
                            .map(|field| field.value().to_string())
                            .collect::<Vec<_>>(),
                        expected
                            .all_fields()
                            .map(|field| field.value().to_string())
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn preserves_formatting_duplicates_dependencies_and_malformed_content() {
    let text = " orphan\r\nbroken\r\nBad Name: value\r\nPackage: zeta\r\nVersion: nope\r\nDepends:\tR (>= 4.0),\r\n\tfoo, bar\r\nX-Custom: café  \r\n .\r\n  東京\r\nX-Custom: second\r\nNeedsCompilation: perhaps\r\n";
    let inputs = [record(text), record("broken"), record(text)];
    let packages = Packages::from_records(&inputs);
    assert_eq!(packages.len(), 3);
    assert_eq!(packages.to_string(), format!("{text}\r\nbroken\n\n{text}"));
    assert!(!packages.validate().is_empty());
    for (actual, expected) in packages.records().zip(&inputs) {
        let actual = Description::from(actual);
        let expected = Description::from(expected);
        assert_eq!(actual.diagnostics(), expected.diagnostics());
        assert_eq!(actual.depends(), expected.depends());
        assert_eq!(
            actual.fields("X-Custom").count(),
            expected.fields("X-Custom").count()
        );
    }
    assert_eq!(inputs[0].to_string(), text);
}

#[test]
fn normalized_descriptions_compose_with_new_records() {
    let descriptions = ["zeta", "alpha"].map(|name| {
        Description::builder()
            .package(LogicalValue::new(name).unwrap())
            .version(LogicalValue::new("1.0").unwrap())
            .depends(LogicalValue::new("R (>= 4.0),\nfoo").unwrap())
            .build()
            .normalize()
            .unwrap()
    });
    let records = descriptions.map(|description| PackageRecord::try_from(description).unwrap());
    let assembled = Packages::from_records(&records);
    assert_eq!(assembled.len(), 2);
    let mixed = Packages::builder()
        .existing_record(&records[0])
        .record(RecordBuilder::new("middle", "2.0").unwrap())
        .existing_record(records[1].clone())
        .build();
    let names = mixed
        .records()
        .map(|record| record.package().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["zeta", "middle", "alpha"]);
    assert_eq!(mixed.record(0).unwrap().depends(), records[0].depends());
    assert_eq!(mixed.record(2).unwrap().to_string(), records[1].to_string());
}

#[test]
fn builder_formatting_applies_only_to_new_records() {
    let existing = record("X: preserved\n\tindent\n");
    let packages = Packages::builder()
        .record(RecordBuilder::new("alpha", "1.0").unwrap())
        .existing_record(existing)
        .record(RecordBuilder::new("beta", "2.0").unwrap())
        .format_style(FormatStyle {
            line_ending: LineEnding::CrLf,
            space_after_colon: false,
            ..FormatStyle::default()
        })
        .build();
    assert_eq!(
        packages.to_string(),
        "Package:alpha\r\nVersion:1.0\r\n\r\nX: preserved\n\tindent\n\nPackage:beta\r\nVersion:2.0"
    );
}

#[test]
fn assembles_repository_scale_inputs_in_order() {
    // Synthetic only: no corpus files or network access. Exercise increasing
    // sizes without platform-dependent timing assertions.
    for count in [10_000, 20_000, 40_000] {
        let packages = Packages::from_records(
            (0..count).map(|index| record(&format!("Package: pkg{index}\nVersion: 1.0\nX: café"))),
        );
        assert_eq!(packages.len(), count);
        for (index, record) in packages.records().enumerate() {
            assert_eq!(record.package().unwrap().as_str(), format!("pkg{index}"));
            assert_eq!(record.field("X").unwrap().as_str(), "café");
        }
    }
}
