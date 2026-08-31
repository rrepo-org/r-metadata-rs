use static_assertions::assert_impl_all;

use crate::{FindingKind, FormatStyle, Packages, PackagesBuilder, RecordBuilder};

assert_impl_all!(Packages: Clone, Send, Sync);

#[test]
fn parses_multiple_records_and_preserves_exact_text() {
    let source = "Package: alpha\nVersion: 1.0\n\nPackage: beta\r\nVersion: 2.0\r\n";
    let packages = Packages::parse(source);
    assert_eq!(packages.len(), 2);
    assert_eq!(packages.to_string(), source);
    assert_eq!(
        packages.record(1).unwrap().package().unwrap().as_str(),
        "beta"
    );
}

#[test]
fn malformed_records_remain_accessible() {
    let packages = Packages::parse("Package: alpha\nbroken\nVersion: no\n");
    let record = packages.record(0).unwrap();
    assert_eq!(record.package().unwrap().as_str(), "alpha");
    let kinds = packages
        .validate()
        .into_iter()
        .map(|item| item.kind())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&FindingKind::MalformedField));
    assert!(kinds.contains(&FindingKind::InvalidVersion));
}

#[test]
fn duplicate_lookup_is_last_wins_and_case_sensitive() {
    let packages = Packages::parse("Package: first\npackage: lower\nPackage: last\nVersion: 1.0");
    let record = packages.record(0).unwrap();
    assert_eq!(record.package().unwrap().as_str(), "last");
    assert_eq!(record.field("package").unwrap().as_str(), "lower");
    assert_eq!(record.fields("Package").count(), 2);
}

#[test]
fn typed_fields_fail_locally_and_relations_recover() {
    let packages = Packages::parse(
        "Package: alpha\nVersion: nope\nDepends: good (>= 1.0), bad (>no), other\nNeedsCompilation: perhaps",
    );
    let record = packages.record(0).unwrap();
    assert!(record.parsed_version().unwrap().is_err());
    let relations = record.parsed_depends().unwrap();
    assert_eq!(relations.entries().len(), 2);
    assert!(!relations.issues().is_empty());
    assert!(record.parsed_needs_compilation().unwrap().is_err());
}

#[test]
fn builder_emits_structurally_clean_records() {
    let first = RecordBuilder::new("alpha", "1.0")
        .unwrap()
        .field("Title", "One\nTwo")
        .unwrap();
    let second = RecordBuilder::new("beta", "2.0").unwrap();
    let packages = PackagesBuilder::new().record(first).record(second).build();
    assert_eq!(packages.len(), 2);
    assert!(packages.validate().is_empty());
    assert_eq!(
        packages.record(0).unwrap().title().unwrap().as_str(),
        "One\nTwo"
    );

    let unsafe_style = FormatStyle {
        continuation_indent: String::new(),
        ..FormatStyle::default()
    };
    let clean = PackagesBuilder::new()
        .format_style(unsafe_style)
        .record(
            RecordBuilder::new("alpha", "1.0")
                .unwrap()
                .field("Title", "One\nTwo")
                .unwrap(),
        )
        .build();
    assert!(clean.validate().is_empty());
}

#[test]
fn edits_are_immutable_and_record_scoped() {
    let source =
        "Package: alpha\nVersion: 1.0\nTitle: old\nTitle: last\n\nPackage: beta\nVersion: 2.0";
    let packages = Packages::parse(source);
    let changed = packages.replace_last(0, "Title", "new").unwrap();
    assert_eq!(
        packages.record(0).unwrap().title().unwrap().as_str(),
        "last"
    );
    assert_eq!(changed.record(0).unwrap().title().unwrap().as_str(), "new");
    assert_eq!(
        changed.record(1).unwrap().version().unwrap().as_str(),
        "2.0"
    );
    let removed = changed
        .remove_all(0, "Title")
        .unwrap()
        .remove_record(1)
        .unwrap();
    assert_eq!(removed.len(), 1);
    let appended = removed.append_record(
        &RecordBuilder::new("gamma", "3.0").unwrap(),
        &FormatStyle::default(),
    );
    assert_eq!(appended.len(), 2);

    let trailing_blank = Packages::parse("Package: alpha\nVersion: 1.0\n\n");
    let appended = trailing_blank.append_record(
        &RecordBuilder::new("beta", "2.0").unwrap(),
        &FormatStyle::default(),
    );
    assert_eq!(appended.to_string().matches("\n\n\n").count(), 0);
}

#[test]
fn empty_and_utf8_inputs() {
    assert!(Packages::parse("").is_empty());
    assert!(Packages::parse_utf8(b"Package: ok\nVersion: 1.0").is_ok());
    assert!(Packages::parse_utf8(&[0xff]).is_err());
}
