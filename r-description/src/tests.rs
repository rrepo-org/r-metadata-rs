use static_assertions::assert_impl_all;

use crate::{Description, DescriptionBuilder, FieldName, FormatStyle, LogicalValue, Severity};

assert_impl_all!(Description: Clone, Send, Sync);

fn value(text: &str) -> LogicalValue {
    LogicalValue::new(text).unwrap()
}

#[test]
fn parsing_is_lossless_tolerant_and_case_sensitive() {
    let source = "Package: first\npackage: lower\nPackage: last\nbroken\n";
    let description = Description::parse(source);
    assert_eq!(description.to_string(), source);
    assert_eq!(description.package().unwrap().as_str(), "last");
    assert_eq!(description.fields("Package").count(), 2);
    assert_eq!(description.fields("package").count(), 1);
    assert_eq!(description.diagnostics().len(), 1);
    assert!(Description::parse_utf8(b"Package: ok").is_ok());
    assert!(Description::parse_utf8(&[0xff]).is_err());
}

#[test]
fn collections_merge_duplicates_and_recover_entries() {
    let description = Description::parse(
        "Depends: R (>= 4.0), broken (>=), good\nDepends: later, also-bad (wat 1.0)\n",
    );
    let parsed = description.depends_parsed();
    let names = parsed
        .entries()
        .iter()
        .map(|entry| entry.value.package())
        .collect::<Vec<_>>();
    assert_eq!(names, ["R", "good", "later"]);
    assert_eq!(parsed.issues().len(), 2);
}

#[test]
fn malformed_typed_fields_are_local() {
    let description =
        Description::parse("Version: nope\nLazyData: yes\nURL: not-a-url, https://r-project.org\n");
    assert!(description.version_parsed().unwrap().is_err());
    assert!(description.lazy_data_parsed().unwrap().unwrap().get());
    let urls = description.urls_parsed();
    assert_eq!(urls.entries().len(), 1);
    assert_eq!(urls.issues().len(), 1);
}

#[test]
fn validation_checks_structure_required_values_and_duplicates() {
    let description = Description::parse(
        "Package: a.\nVersion: bad\nTitle: One\nTitle: Two\nDescription: text\nLicense: MIT\nAuthors@R: person('A', 'B')\nOS_type: mac\n",
    );
    let validation = description.validate();
    let codes = validation
        .issues()
        .iter()
        .map(crate::ValidationIssue::code)
        .collect::<Vec<_>>();
    assert!(codes.contains(&"invalid-package-name"));
    assert!(codes.contains(&"invalid-version"));
    assert!(codes.contains(&"duplicate-scalar"));
    assert!(codes.contains(&"invalid-os-type"));
    assert!(validation.issues().iter().any(|issue| {
        issue.code() == "duplicate-scalar" && issue.severity() == Severity::Warning
    }));
    assert!(!validation.is_valid());
}

#[test]
fn builder_and_immutable_edits_preserve_structure() {
    let description = DescriptionBuilder::new(FormatStyle::default())
        .package(value("demo"))
        .version(value("1.0"))
        .title(value("Demo"))
        .description(value("First line\nSecond line"))
        .license(value("MIT"))
        .authors_at_r(value("person('A', 'B')"))
        .build();
    assert!(description.diagnostics().is_empty());
    assert_eq!(
        description.description().unwrap().as_str(),
        "First line\nSecond line"
    );

    let changed = description
        .replace_last("Title", &value("Changed"))
        .unwrap();
    assert_eq!(changed.title().unwrap().as_str(), "Changed");
    assert_eq!(description.title().unwrap().as_str(), "Demo");

    let name = FieldName::new("URL").unwrap();
    let inserted = changed
        .set_field(&name, &value("https://example.com"))
        .unwrap();
    assert_eq!(inserted.url().unwrap().as_str(), "https://example.com");
    let removed = inserted.remove_all("URL").unwrap();
    assert!(removed.url().is_none());
}
