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
        ["good", "later", "R"]
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

#[test]
fn whole_document_normalization_is_canonical_and_idempotent() {
    let source = concat!(
        "X-Z: one  \r\n",
        "Title: Old\r\n",
        "Package: demo\r\n",
        "Title: New\r\n",
        "Version: 1.0\r\n",
        "Imports: zed\r\n",
        "Imports: alpha, zed\r\n",
        "Description: Demo\r\n",
        "License: MIT\r\n",
        "Encoding:\r\n",
        "Authors@R: person(\"A\", \"B\")\r\n",
        "X-Z: two\r\n",
        "Collate: 'b.R' 'a.R'\r\n",
    );
    let normalized = Description::parse(source).normalize().unwrap();
    assert_eq!(
        normalized.to_string(),
        concat!(
            "Package: demo\n",
            "Title: New\n",
            "Version: 1.0\n",
            "Authors@R: person(\"A\", \"B\")\n",
            "Description: Demo\n",
            "License: MIT\n",
            "Imports:\n",
            "    alpha,\n",
            "    zed\n",
            "X-Z: one\n",
            "X-Z: two\n",
            "Collate: 'b.R' 'a.R'\n",
        )
    );
    assert_eq!(normalized.normalize().unwrap(), normalized);
}

#[test]
fn normalization_sorts_package_relations_case_insensitively() {
    let normalized = Description::parse("Imports: XML, curl, Rcpp, askpass, xml2\n")
        .normalize()
        .unwrap();

    assert_eq!(
        normalized.to_string(),
        concat!(
            "Imports:\n",
            "    askpass,\n",
            "    curl,\n",
            "    Rcpp,\n",
            "    XML,\n",
            "    xml2\n",
        )
    );
    assert_eq!(normalized.normalize().unwrap(), normalized);
}

#[test]
fn normalization_wraps_prose_and_retains_empty_required_fields() {
    let text = "word ".repeat(30);
    let source = format!("Description: {text}\nTitle:\nEncoding:\n");
    let normalized = Description::parse(&source).normalize().unwrap();
    let rendered = normalized.to_string();

    assert!(rendered.contains("Title:\n"));
    assert!(!rendered.contains("Encoding:"));
    assert!(rendered.lines().all(|line| line.len() <= 80));
    assert!(rendered.contains("\n    word"));
}

#[test]
fn normalization_reports_unsafe_content_atomically() {
    let source = "Package: demo\nImports: valid, broken (>=)\nbroken line\n";
    let description = Description::parse(source);
    let error = description.normalize().unwrap_err();
    let codes = error
        .diagnostics()
        .iter()
        .map(crate::NormalizationDiagnostic::code)
        .collect::<Vec<_>>();

    assert!(codes.contains(&"syntax.malformed-field"));
    assert!(codes.contains(&"invalid-collection"));
    assert!(
        error
            .diagnostics()
            .iter()
            .all(|item| !item.message().is_empty())
    );
    assert_eq!(description.to_string(), source);
}

#[test]
fn normalization_rejects_multiple_records() {
    let error = Description::parse("Package: first\n\nPackage: second\n")
        .normalize()
        .unwrap_err();
    assert_eq!(error.diagnostics()[0].code(), "record-count");
}

#[test]
fn normalization_rejects_a_record_that_would_become_empty() {
    let error = Description::parse("Encoding:\n").normalize().unwrap_err();
    assert_eq!(error.diagnostics()[0].code(), "empty-record");
}

#[test]
fn normalization_preserves_repository_and_remote_priority_order() {
    let source = concat!(
        "Remotes: github::z/repo, cran::alpha\n",
        "Additional_repositories: https://z.example/repo\n",
        "Remotes: github::z/repo\n",
        "Additional_repositories: https://a.example/repo, https://z.example/repo\n",
    );
    let normalized = Description::parse(source).normalize().unwrap();

    assert_eq!(
        normalized.to_string(),
        concat!(
            "Remotes:\n",
            "    github::z/repo,\n",
            "    cran::alpha\n",
            "Additional_repositories:\n",
            "    https://z.example/repo,\n",
            "    https://a.example/repo\n",
        )
    );
    assert_eq!(normalized.normalize().unwrap(), normalized);
}
