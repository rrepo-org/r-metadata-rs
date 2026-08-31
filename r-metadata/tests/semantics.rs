//! Integration tests for public semantic values and collection recovery.

use r_metadata::{
    AdditionalRepositories, Logical, PositionedRelationParseError, PositionedRemoteParseError,
    PositionedUrlParseError, Relation, RelationList, Remote, RemoteList, RemoteParseError,
    RemoteSource, RequirementVersion, Span, UrlList, Version, VersionRequirement,
};
use static_assertions::assert_impl_all;
use std::collections::HashSet;

assert_impl_all!(Version: Send, Sync);
assert_impl_all!(Relation: Send, Sync);
assert_impl_all!(RelationList: Send, Sync);
assert_impl_all!(PositionedRelationParseError: Send, Sync);
assert_impl_all!(Remote: Send, Sync);
assert_impl_all!(RemoteList: Send, Sync);
assert_impl_all!(PositionedRemoteParseError: Send, Sync);
assert_impl_all!(UrlList: Send, Sync);
assert_impl_all!(PositionedUrlParseError: Send, Sync);

#[test]
fn versions_preserve_spelling_with_numeric_identity() {
    let first: Version = "01.2-0".parse().unwrap();
    let second: Version = "1.2.0.0".parse().unwrap();
    assert_eq!(first.as_str(), "01.2-0");
    assert_eq!(first.components(), &[1, 2, 0]);
    assert_eq!(first, second);
    assert_eq!(HashSet::from([first, second]).len(), 1);
    assert!("1".parse::<Version>().is_err());
    assert!("1.é".parse::<Version>().unwrap_err().span().is_some());
}

#[test]
fn relations_support_every_operator_and_r_revisions() {
    for operator in ["<", "<=", "==", "!=", ">", ">="] {
        let text = format!("R ({operator} r56550)");
        let relation: Relation = text.parse().unwrap();
        assert_eq!(relation.to_string(), text);
        let operand = match relation.requirement() {
            VersionRequirement::LessThan(value)
            | VersionRequirement::LessThanEqual(value)
            | VersionRequirement::Equal(value)
            | VersionRequirement::NotEqual(value)
            | VersionRequirement::GreaterThan(value)
            | VersionRequirement::GreaterThanEqual(value) => value,
            VersionRequirement::Any => panic!("expected constrained relation"),
        };
        assert!(
            matches!(operand, RequirementVersion::Revision(revision) if revision.number() == 56_550)
        );
    }
}

#[test]
fn relation_lists_recover_valid_entries_and_precise_issues() {
    let input = "Cli, bad (= 1.0), cli (>= 2.0),";
    let parsed = RelationList::parse(input);
    assert_eq!(parsed.entries().len(), 2);
    assert_eq!(parsed.entries()[0].value().package(), "Cli");
    assert_eq!(parsed.entries()[1].value().package(), "cli");
    assert_eq!(parsed.entries()[0].span(), Span::new(0, 3));
    assert_eq!(
        &input[parsed.issues()[0].span().start()..parsed.issues()[0].span().end()],
        "="
    );
    assert!(parsed.issues().last().unwrap().span().is_empty());
}

#[test]
fn url_lists_apply_field_specific_delimiters_and_recover() {
    let input = "https://one.example bad https://two.example, still-bad";
    let urls = UrlList::parse(input);
    assert_eq!(urls.entries().len(), 2);
    assert_eq!(urls.issues().len(), 2);
    assert_eq!(urls.entries()[0].span(), Span::new(0, 19));

    let repositories =
        AdditionalRepositories::parse("https://one.example https://two.example,https://ok.example");
    assert_eq!(repositories.entries().len(), 1);
    assert_eq!(repositories.issues().len(), 1);
}

#[test]
fn remotes_cover_sources_aliases_hosts_and_references() {
    let cases = [
        "dplyr",
        "alias=owner/repository/subdir@main",
        "cran::dplyr",
        "github@ghe.example::owner/repository#pull/1",
        "gitlab::group/nested/repository@main",
        "bitbucket::owner/repository/subdir@stable",
        "git::ssh://git@example.com/team/repository.git@v1",
        "url::https://example.com/package.tar.gz",
        "local::../package",
        "svn::https://example.com/svn/trunk",
        "bioc::3.20/SummarizedExperiment#abc",
        "forge@packages.example::owner/package",
    ];
    for input in cases {
        let remote: Remote = input.parse().unwrap();
        assert_eq!(
            remote.to_string().parse::<Remote>().unwrap(),
            remote,
            "{input}"
        );
    }
    assert!(matches!(
        "owner/repo".parse::<Remote>().unwrap().source,
        RemoteSource::GitHub(_)
    ));
    assert!(matches!(
        "forge::thing".parse::<Remote>().unwrap().source,
        RemoteSource::Unknown(_)
    ));
}

#[test]
fn remote_collections_recover_and_errors_do_not_retain_secrets() {
    let input = "owner/good, bad remote,gitlab::group/repo,github::owner/repo@";
    let parsed = RemoteList::parse(input);
    assert_eq!(parsed.entries().len(), 2);
    assert_eq!(parsed.issues().len(), 2);

    let secret = "very-secret-password";
    let error = format!("bioc:::{secret}@package")
        .parse::<Remote>()
        .unwrap_err();
    assert!(matches!(
        error.error(),
        RemoteParseError::MalformedBioconductorCredentials
    ));
    assert!(!format!("{error:?}{error}").contains(secret));
}

#[test]
fn semantic_debug_redacts_credentials() {
    for input in [
        "bioc::secret-user:secret-pass@release/Package",
        "url::https://secret-user:secret-pass@example.com/archive.tar.gz",
        "git::https://secret-user:secret-pass@example.com/repository.git@main",
        "svn::https://secret-user:secret-pass@example.com/trunk",
    ] {
        let debug = format!("{:?}", input.parse::<Remote>().unwrap());
        assert!(!debug.contains("secret-user"));
        assert!(!debug.contains("secret-pass"));
        assert!(debug.contains("REDACTED"));
    }
}

#[test]
fn logical_parser_accepts_documented_spellings() {
    assert!("YES".parse::<Logical>().unwrap().get());
    assert!("true".parse::<Logical>().unwrap().get());
    assert!(!"No".parse::<Logical>().unwrap().get());
    assert!("1".parse::<Logical>().is_err());
}
