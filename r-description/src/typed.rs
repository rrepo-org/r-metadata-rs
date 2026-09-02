//! Field-local semantic parsing.

use std::str::FromStr;

use r_metadata::{
    AdditionalRepositories, Logical, LogicalParseError, OsType, OsTypeParseError,
    PositionedRelationParseError, PositionedRemoteParseError, PositionedUrlParseError, Priority,
    PriorityParseError, Relation, RelationList, Remote, RemoteList, Span, Spanned, Url, UrlList,
    Version, VersionParseError,
};

use crate::{Description, SourceSpan};

/// One valid collection entry, retaining its declaration and relative spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionEntry<T> {
    /// Parsed value.
    pub value: T,
    /// Span of the complete DCF field declaration.
    pub field_span: SourceSpan,
    /// Span relative to the unfolded field value.
    pub value_span: Span,
}

/// One collection issue, retaining its declaration and relative spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionIssue<E> {
    /// Typed parser issue.
    pub error: E,
    /// Span of the complete DCF field declaration.
    pub field_span: SourceSpan,
    /// Span relative to the unfolded field value.
    pub value_span: Span,
}

/// Recovered semantic values merged from duplicate declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionResult<T, E> {
    entries: Vec<CollectionEntry<T>>,
    issues: Vec<CollectionIssue<E>>,
}

impl<T, E> Default for CollectionResult<T, E> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            issues: Vec::new(),
        }
    }
}

impl<T, E> CollectionResult<T, E> {
    /// Returns valid entries in declaration and entry source order.
    pub fn entries(&self) -> &[CollectionEntry<T>] {
        &self.entries
    }
    /// Iterates over parsed values without hiding the separately retained issues.
    pub fn values(&self) -> impl ExactSizeIterator<Item = &T> + DoubleEndedIterator {
        self.entries.iter().map(|entry| &entry.value)
    }
    /// Returns issues in declaration and issue source order.
    pub fn issues(&self) -> &[CollectionIssue<E>] {
        &self.issues
    }
    /// Splits the aggregate into valid entries and issues.
    pub fn into_parts(self) -> (Vec<CollectionEntry<T>>, Vec<CollectionIssue<E>>) {
        (self.entries, self.issues)
    }
}

impl Description {
    /// Parses the last `Version` declaration.
    pub fn version_parsed(&self) -> Option<Result<Version, VersionParseError>> {
        self.version()
            .map(|value| Version::from_str(value.as_str()))
    }

    /// Parses and merges all `Depends` declarations.
    pub fn depends_parsed(&self) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("Depends")
    }
    /// Parses and merges all `Imports` declarations.
    pub fn imports_parsed(&self) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("Imports")
    }
    /// Parses and merges all `Suggests` declarations.
    pub fn suggests_parsed(&self) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("Suggests")
    }
    /// Parses and merges all `Enhances` declarations.
    pub fn enhances_parsed(&self) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("Enhances")
    }
    /// Parses and merges all `LinkingTo` declarations.
    pub fn linking_to_parsed(&self) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("LinkingTo")
    }
    /// Parses and merges all `VignetteBuilder` declarations.
    pub fn vignette_builder_parsed(
        &self,
    ) -> CollectionResult<Relation, PositionedRelationParseError> {
        self.relations("VignetteBuilder")
    }

    /// Parses and merges all `URL` declarations.
    pub fn urls_parsed(&self) -> CollectionResult<Url, PositionedUrlParseError> {
        self.urls("URL", false)
    }
    /// Parses and merges all `BugReports` declarations.
    pub fn bug_reports_parsed(&self) -> CollectionResult<Url, PositionedUrlParseError> {
        self.urls("BugReports", false)
    }
    /// Parses and merges all `Additional_repositories` declarations.
    pub fn additional_repositories_parsed(&self) -> CollectionResult<Url, PositionedUrlParseError> {
        self.urls("Additional_repositories", true)
    }
    /// Parses and merges all `Remotes` declarations.
    pub fn remotes_parsed(&self) -> CollectionResult<Remote, PositionedRemoteParseError> {
        let mut result = CollectionResult::default();
        for field in self.fields("Remotes") {
            let span = field.source_range();
            let (entries, issues) = RemoteList::parse(field.value().as_str()).into_parts();
            append_entries(&mut result.entries, entries, span);
            result
                .issues
                .extend(issues.into_iter().map(|error| CollectionIssue {
                    value_span: error.span(),
                    error,
                    field_span: span,
                }));
        }
        result
    }

    /// Parses the last `NeedsCompilation` declaration.
    pub fn needs_compilation_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("NeedsCompilation")
    }
    /// Parses the last `Biarch` declaration.
    pub fn biarch_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("Biarch")
    }
    /// Parses the last `LazyData` declaration.
    pub fn lazy_data_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("LazyData")
    }
    /// Parses the last `LazyLoad` declaration.
    pub fn lazy_load_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("LazyLoad")
    }
    /// Parses the last `ByteCompile` declaration.
    pub fn byte_compile_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("ByteCompile")
    }
    /// Parses the last `KeepSource` declaration.
    pub fn keep_source_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("KeepSource")
    }
    /// Parses the last `UseLTO` declaration.
    pub fn use_lto_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("UseLTO")
    }
    /// Parses the last `StagedInstall` declaration.
    pub fn staged_install_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("StagedInstall")
    }
    /// Parses the last `ZipData` declaration.
    pub fn zip_data_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("ZipData")
    }
    /// Parses the last `BuildVignettes` declaration.
    pub fn build_vignettes_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("BuildVignettes")
    }
    /// Parses the last `License_is_FOSS` declaration.
    pub fn license_is_foss_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("License_is_FOSS")
    }
    /// Parses the last `License_restricts_use` declaration.
    pub fn license_restricts_use_parsed(&self) -> Option<Result<Logical, LogicalParseError>> {
        self.logical("License_restricts_use")
    }
    /// Parses the last `OS_type` declaration.
    pub fn os_type_parsed(&self) -> Option<Result<OsType, OsTypeParseError>> {
        self.os_type().map(|value| value.as_str().parse())
    }
    /// Parses the last `Priority` declaration.
    pub fn priority_parsed(&self) -> Option<Result<Priority, PriorityParseError>> {
        self.priority().map(|value| value.as_str().parse())
    }

    fn logical(&self, name: &str) -> Option<Result<Logical, LogicalParseError>> {
        self.raw(name).map(|value| value.as_str().parse())
    }

    fn relations(&self, name: &str) -> CollectionResult<Relation, PositionedRelationParseError> {
        let mut result = CollectionResult::default();
        for field in self.fields(name) {
            let span = field.source_range();
            let (entries, issues) = RelationList::parse(field.value().as_str()).into_parts();
            append_entries(&mut result.entries, entries, span);
            result
                .issues
                .extend(issues.into_iter().map(|error| CollectionIssue {
                    value_span: error.span(),
                    error,
                    field_span: span,
                }));
        }
        result
    }

    fn urls(
        &self,
        name: &str,
        repositories: bool,
    ) -> CollectionResult<Url, PositionedUrlParseError> {
        let mut result = CollectionResult::default();
        for field in self.fields(name) {
            let span = field.source_range();
            let (entries, issues) = if repositories {
                AdditionalRepositories::parse(field.value().as_str()).into_parts()
            } else {
                UrlList::parse(field.value().as_str()).into_parts()
            };
            append_entries(&mut result.entries, entries, span);
            result
                .issues
                .extend(issues.into_iter().map(|error| CollectionIssue {
                    value_span: error.span(),
                    error,
                    field_span: span,
                }));
        }
        result
    }
}

fn append_entries<T>(
    target: &mut Vec<CollectionEntry<T>>,
    entries: Vec<Spanned<T>>,
    span: SourceSpan,
) {
    target.extend(entries.into_iter().map(|entry| CollectionEntry {
        value: entry.value,
        value_span: entry.span,
        field_span: span,
    }));
}
