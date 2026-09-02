use std::{collections::BTreeSet, str::FromStr};

use static_assertions::assert_impl_all;

use crate::{
    Description, DescriptionBuilder, FieldName, FormatStyle, LogicalValue, Relation, Remote,
    Severity, Url,
};
use r_metadata::RemoteSource;

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

    let values = parsed.values().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        values.iter().map(Relation::package).collect::<Vec<_>>(),
        ["R", "good", "later"]
    );
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

#[test]
fn relation_setters_normalize_the_complete_collection() {
    let source =
        "Package: demo\r\nDepends: old\r\nbroken text\r\nDepends:\r\n old-two\r\nTitle: Kept";
    let description = Description::parse(source);
    let relations = [
        Relation::from_str("zeta (>= 2.0)").unwrap(),
        Relation::from_str("alpha").unwrap(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    let changed = description.set_depends(&relations).unwrap();
    assert_eq!(
        changed.to_string(),
        "Package: demo\r\nbroken text\r\nDepends:\r\n    alpha,\r\n    zeta (>= 2.0)\r\nTitle: Kept"
    );
    assert_eq!(description.to_string(), source);
    assert_eq!(changed.fields("Depends").count(), 1);
    assert_eq!(changed.depends_parsed().values().count(), 2);

    let one = Description::parse("Package: demo\nTitle: Kept")
        .set_imports([Relation::from_str("only").unwrap()])
        .unwrap();
    assert_eq!(
        one.to_string(),
        "Package: demo\nTitle: Kept\nImports:\n    only"
    );
}

#[test]
fn every_relation_collection_has_a_typed_setter() {
    let relation = Relation::from_str("pkg").unwrap();
    let description = Description::parse("Package: demo\n");

    assert!(
        description
            .set_depends([&relation])
            .unwrap()
            .depends()
            .is_some()
    );
    assert!(
        description
            .set_imports([&relation])
            .unwrap()
            .imports()
            .is_some()
    );
    assert!(
        description
            .set_suggests([&relation])
            .unwrap()
            .suggests()
            .is_some()
    );
    assert!(
        description
            .set_enhances([&relation])
            .unwrap()
            .enhances()
            .is_some()
    );
    assert!(
        description
            .set_linking_to([&relation])
            .unwrap()
            .linking_to()
            .is_some()
    );
    assert!(
        description
            .set_vignette_builder([&relation])
            .unwrap()
            .vignette_builder()
            .is_some()
    );
}

#[test]
fn remote_and_repository_setters_preserve_iteration_order() {
    let remotes = ["github::z/repo", "cran::alpha"]
        .into_iter()
        .map(Remote::from_str)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let repositories = [
        Url::parse("https://z.example/repo").unwrap(),
        Url::parse("https://a.example/repo").unwrap(),
    ];
    let description = Description::parse("Package: demo\nRemotes: old\nTail: kept\n");

    let changed = description
        .set_remotes(&remotes)
        .unwrap()
        .set_additional_repositories(&repositories)
        .unwrap();
    assert_eq!(
        changed.to_string(),
        "Package: demo\nRemotes:\n    github::z/repo,\n    cran::alpha\nTail: kept\nAdditional_repositories:\n    https://z.example/repo,\n    https://a.example/repo\n"
    );
    assert!(changed.remotes_parsed().issues().is_empty());
    assert!(changed.additional_repositories_parsed().issues().is_empty());

    let sorted = remotes.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(sorted.len(), 2);
}

#[test]
fn empty_collections_remove_duplicates_or_leave_absent_fields_unchanged() {
    let description = Description::parse("Package: demo\nDepends: one\nDepends: two\nTail: kept\n");
    let empty = Vec::<Relation>::new();
    let changed = description.set_depends(&empty).unwrap();
    assert_eq!(changed.to_string(), "Package: demo\nTail: kept\n");

    let unchanged = changed.set_imports(&empty).unwrap();
    assert_eq!(unchanged, changed);
}

#[test]
fn invalid_constructed_remote_returns_an_error_without_an_edit() {
    let description = Description::parse("Package: demo\nRemotes: cran::old\n");
    let remote = Remote {
        package: None,
        host: None,
        source: RemoteSource::Unspecified("contains whitespace".to_owned()),
    };

    let error = description.set_remotes([remote]).unwrap_err();
    assert!(error.to_string().contains("collection index 0"));
    assert_eq!(
        description.to_string(),
        "Package: demo\nRemotes: cran::old\n"
    );
}
