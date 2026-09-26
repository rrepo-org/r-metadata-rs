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
                    .records(&inputs)
                    .line_ending(line_ending)
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
        .record(&records[0])
        .record(RecordBuilder::new("middle", "2.0").unwrap().build())
        .record(records[1].clone())
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
fn record_formatting_is_independent_of_document_boundaries() {
    let existing = record("X: preserved\n\tindent\n");
    let first = PackageRecord::builder("alpha", "1.0")
        .unwrap()
        .format_style(FormatStyle {
            line_ending: LineEnding::CrLf,
            space_after_colon: false,
            ..FormatStyle::default()
        })
        .build();
    assert_eq!(first.to_string(), "Package:alpha\r\nVersion:1.0");
    let packages = Packages::builder()
        .record(&first)
        .record(existing)
        .record(PackageRecord::builder("beta", "2.0").unwrap().build())
        .line_ending(LineEnding::Cr)
        .build();
    assert_eq!(
        packages.to_string(),
        "Package:alpha\r\nVersion:1.0\r\rX: preserved\n\tindent\n\nPackage: beta\nVersion: 2.0"
    );
}

#[test]
fn standalone_builder_checks_structure_but_leaves_semantics_explicit() {
    assert!(PackageRecord::builder("bad name", "1.0").is_err());
    assert!(PackageRecord::builder("alpha", "nope").is_err());
    assert!(
        PackageRecord::builder("alpha", "1.0")
            .unwrap()
            .field("Bad Name", "value")
            .is_err()
    );
    let built = PackageRecord::builder("alpha", "1.0")
        .unwrap()
        .field("X-Custom", "café\n東京")
        .unwrap()
        .field("Version", "nope")
        .unwrap()
        .format_style(FormatStyle {
            continuation_indent: "invalid".to_owned(),
            ..FormatStyle::default()
        })
        .build();
    assert_eq!(built.fields("Version").count(), 2);
    assert_eq!(built.field("X-Custom").unwrap().as_str(), "café\n東京");
    assert!(built.parsed_version().unwrap().is_err());
    let description = Description::from(&built);
    assert!(description.diagnostics().is_empty());
    assert_eq!(
        PackageRecord::try_from(description).unwrap().to_string(),
        built.to_string()
    );
    assert!(!Packages::from_records([built]).validate().is_empty());
}

#[test]
fn append_and_assembly_share_boundaries_for_all_record_origins() {
    let inputs = [
        record(" orphan\nVersion: nope"),
        PackageRecord::builder("alpha", "1.0").unwrap().build(),
    ];
    for ending in ["", "\n", "\r\n", "\r"] {
        let text = format!("Package: first{ending}");
        let original = Packages::parse(&text);
        for line_ending in [LineEnding::Lf, LineEnding::CrLf, LineEnding::Cr] {
            for input in &inputs {
                let appended = original.append_record(input, line_ending);
                let assembled = Packages::builder()
                    .records(original.records())
                    .record(input)
                    .line_ending(line_ending)
                    .build();
                assert_eq!(appended.to_string(), assembled.to_string());
                assert_eq!(appended.len(), 2);
                assert_eq!(appended.record(1).unwrap().to_string(), input.to_string());
            }
        }
        assert_eq!(original.to_string(), text);
    }
}

#[test]
fn append_preserves_and_completes_existing_blank_separators() {
    let next = record("X: next");
    for (source, expected_prefix) in [
        ("", ""),
        ("\n", "\n"),
        (" \t", " \t\n"),
        ("X: old\n\n", "X: old\n\n"),
        ("X: old\r\n \t\r\n", "X: old\r\n \t\r\n"),
        ("X: old\r\r\r", "X: old\r\r\r"),
        ("X: old\r\n \t", "X: old\r\n \t\r\n"),
        ("X: old\r\u{c}", "X: old\r\u{c}\r"),
        ("X: old\n\r\n \t\n", "X: old\n\r\n \t\n"),
    ] {
        let original = Packages::parse(source);
        let appended = original.append_record(&next, LineEnding::Lf);
        assert_eq!(appended.to_string(), format!("{expected_prefix}X: next"));
        assert_eq!(appended.len(), original.len() + 1);
        assert_eq!(original.to_string(), source);
    }
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
