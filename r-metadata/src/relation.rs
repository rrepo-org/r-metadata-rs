//! Package dependency relations.

use crate::{Span, Spanned, Version, VersionParseError};
use std::str::FromStr;

/// An R source-control revision such as `r56550`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Revision {
    original: Box<str>,
    number: u64,
}

impl Revision {
    /// Returns the exact spelling supplied to the parser.
    pub fn as_str(&self) -> &str {
        &self.original
    }
    /// Returns the numeric revision.
    pub const fn number(&self) -> u64 {
        self.number
    }
}

impl FromStr for Revision {
    type Err = RevisionParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let digits = input
            .strip_prefix('r')
            .filter(|digits| !digits.is_empty())
            .ok_or(RevisionParseError::Invalid)?;
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(RevisionParseError::Invalid);
        }
        let number = digits.parse().map_err(|_| RevisionParseError::Overflow)?;
        Ok(Self {
            original: input.into(),
            number,
        })
    }
}

/// Error parsing an R source revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RevisionParseError {
    /// The value is not `r` followed by one or more ASCII digits.
    #[error("revision must be r followed by ASCII digits")]
    Invalid,
    /// The numeric revision exceeds `u64`.
    #[error("revision number is too large")]
    Overflow,
}

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original)
    }
}

/// The right-hand operand of a version requirement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RequirementVersion {
    /// A numeric package version.
    Version(Version),
    /// An R source revision.
    Revision(Revision),
}

impl RequirementVersion {
    fn parse(input: &str) -> Result<Self, RequirementVersionParseError> {
        if input.starts_with('r') {
            return input
                .parse()
                .map(Self::Revision)
                .map_err(RequirementVersionParseError::Revision);
        }
        input
            .parse()
            .map(Self::Version)
            .map_err(RequirementVersionParseError::Version)
    }
}

impl FromStr for RequirementVersion {
    type Err = RequirementVersionParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

/// Error parsing a numeric version or an R source revision.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequirementVersionParseError {
    /// Invalid numeric package version.
    #[error("invalid package version: {0}")]
    Version(#[source] VersionParseError),
    /// Invalid R source revision.
    #[error("invalid R revision: {0}")]
    Revision(#[source] RevisionParseError),
}

impl From<Version> for RequirementVersion {
    fn from(value: Version) -> Self {
        Self::Version(value)
    }
}
impl From<Revision> for RequirementVersion {
    fn from(value: Revision) -> Self {
        Self::Revision(value)
    }
}
impl std::fmt::Display for RequirementVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Version(value) => value.fmt(f),
            Self::Revision(value) => value.fmt(f),
        }
    }
}

/// A package version constraint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VersionRequirement {
    /// No constraint.
    Any,
    /// `<`.
    LessThan(RequirementVersion),
    /// `<=`.
    LessThanEqual(RequirementVersion),
    /// `==`.
    Equal(RequirementVersion),
    /// `!=`.
    NotEqual(RequirementVersion),
    /// `>`.
    GreaterThan(RequirementVersion),
    /// `>=`.
    GreaterThanEqual(RequirementVersion),
}

impl VersionRequirement {
    /// Tests a numeric version. Revision requirements cannot match a package version.
    pub fn matches(&self, candidate: &Version) -> bool {
        let compare = |required: &RequirementVersion| match required {
            RequirementVersion::Version(version) => Some(candidate.cmp(version)),
            RequirementVersion::Revision(_) => None,
        };
        match self {
            Self::Any => true,
            Self::LessThan(v) => compare(v).is_some_and(std::cmp::Ordering::is_lt),
            Self::LessThanEqual(v) => compare(v).is_some_and(std::cmp::Ordering::is_le),
            Self::Equal(v) => compare(v).is_some_and(std::cmp::Ordering::is_eq),
            Self::NotEqual(v) => compare(v).is_some_and(|o| !o.is_eq()),
            Self::GreaterThan(v) => compare(v).is_some_and(std::cmp::Ordering::is_gt),
            Self::GreaterThanEqual(v) => compare(v).is_some_and(std::cmp::Ordering::is_ge),
        }
    }
}

/// A case-sensitive package name and optional version requirement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Relation {
    package: String,
    requirement: VersionRequirement,
}

impl Relation {
    /// Constructs a relation after checking list-syntax ambiguity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRelationPackage`] when the name is empty or contains
    /// whitespace, control characters, commas, or parentheses.
    pub fn new(
        package: impl Into<String>,
        requirement: VersionRequirement,
    ) -> Result<Self, InvalidRelationPackage> {
        let package = package.into();
        if !valid_package(&package) {
            return Err(InvalidRelationPackage { package });
        }
        Ok(Self {
            package,
            requirement,
        })
    }
    /// Constructs an unconstrained relation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRelationPackage`] when the name cannot be represented
    /// unambiguously in relation list syntax.
    pub fn any(package: impl Into<String>) -> Result<Self, InvalidRelationPackage> {
        Self::new(package, VersionRequirement::Any)
    }
    /// Returns the case-sensitive package name.
    pub fn package(&self) -> &str {
        &self.package
    }
    /// Returns the version requirement.
    pub const fn requirement(&self) -> &VersionRequirement {
        &self.requirement
    }
}

impl std::fmt::Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.package)?;
        let (op, version) = match &self.requirement {
            VersionRequirement::Any => return Ok(()),
            VersionRequirement::LessThan(v) => ("<", v),
            VersionRequirement::LessThanEqual(v) => ("<=", v),
            VersionRequirement::Equal(v) => ("==", v),
            VersionRequirement::NotEqual(v) => ("!=", v),
            VersionRequirement::GreaterThan(v) => (">", v),
            VersionRequirement::GreaterThanEqual(v) => (">=", v),
        };
        write!(f, " ({op} {version})")
    }
}

impl FromStr for Relation {
    type Err = PositionedRelationParseError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (value, mut issues) = parse_one(input, 0, input.len());
        value
            .filter(|_| issues.is_empty())
            .map(|(value, _)| value)
            .ok_or_else(|| {
                issues
                    .pop()
                    .unwrap_or_else(|| issue(RelationParseError::Empty, 0, 0))
            })
    }
}

/// Recovered result of parsing a comma-separated relation list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelationList {
    entries: Vec<Spanned<Relation>>,
    issues: Vec<PositionedRelationParseError>,
}

impl RelationList {
    /// Parses all entries, recovering after every comma.
    pub fn parse(input: &str) -> Self {
        let mut result = Self::default();
        for (start, end) in segments(input) {
            let (entry, entry_issues) = parse_one(input, start, end);
            if let Some((value, span)) = entry {
                result.entries.push(Spanned { value, span });
            }
            result.issues.extend(entry_issues);
        }
        result
    }
    /// Returns valid entries in source order.
    pub fn entries(&self) -> &[Spanned<Relation>] {
        &self.entries
    }
    /// Returns all issues in source order.
    pub fn issues(&self) -> &[PositionedRelationParseError] {
        &self.issues
    }
    /// Splits the recovered result into owned vectors.
    pub fn into_parts(self) -> (Vec<Spanned<Relation>>, Vec<PositionedRelationParseError>) {
        (self.entries, self.issues)
    }
}

/// Error constructing a relation from an ambiguous package name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid relation package name {package:?}")]
pub struct InvalidRelationPackage {
    package: String,
}
impl InvalidRelationPackage {
    /// Returns the rejected package name.
    pub fn package(&self) -> &str {
        &self.package
    }
}

/// A relation syntax error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RelationParseError {
    /// Empty comma-delimited entry.
    #[error("empty package relation")]
    Empty,
    /// Missing package name.
    #[error("package relation is missing a package name")]
    MissingPackage,
    /// Package name contains list syntax or control characters.
    #[error("invalid package name in relation")]
    InvalidPackage,
    /// Expected a parenthesized requirement.
    #[error("expected a parenthesized version requirement")]
    ExpectedVersionClause,
    /// Missing `)`.
    #[error("version requirement is missing a closing parenthesis")]
    MissingClosingParenthesis,
    /// Text follows `)`.
    #[error("unexpected text after package relation")]
    UnexpectedTrailingText,
    /// Missing operator.
    #[error("version requirement is missing a comparison operator")]
    MissingOperator,
    /// Unsupported operator.
    #[error("unsupported version comparison operator")]
    InvalidOperator,
    /// Operator is not followed by whitespace.
    #[error("version comparison operator must be followed by whitespace")]
    MissingOperatorWhitespace,
    /// Missing version operand.
    #[error("version requirement is missing a version")]
    MissingVersion,
    /// Invalid version operand.
    #[error("invalid required version: {0}")]
    InvalidVersion(#[source] RequirementVersionParseError),
}

/// A relation error and its precise input span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedRelationParseError {
    error: RelationParseError,
    span: Span,
}
impl PositionedRelationParseError {
    /// Returns the typed issue.
    pub const fn error(&self) -> &RelationParseError {
        &self.error
    }
    /// Returns the relative byte span.
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Alias for [`Self::span`].
    pub const fn range(&self) -> Span {
        self.span
    }
}
impl std::fmt::Display for PositionedRelationParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for PositionedRelationParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[allow(clippy::too_many_lines)]
fn parse_one(
    input: &str,
    raw_start: usize,
    raw_end: usize,
) -> (Option<(Relation, Span)>, Vec<PositionedRelationParseError>) {
    let (start, end) = trim_bounds(input, raw_start, raw_end);
    if start == end {
        return (None, vec![issue(RelationParseError::Empty, start, end)]);
    }
    let text = &input[start..end];
    let package_len = text
        .char_indices()
        .find_map(|(i, c)| (c.is_whitespace() || matches!(c, '(' | ')' | ',')).then_some(i))
        .unwrap_or(text.len());
    if package_len == 0 {
        return (
            None,
            vec![issue(RelationParseError::MissingPackage, start, start)],
        );
    }
    let package = &text[..package_len];
    if !valid_package(package) {
        return (
            None,
            vec![issue(
                RelationParseError::InvalidPackage,
                start,
                start + package_len,
            )],
        );
    }
    let mut cursor = skip_ws(input, start + package_len, end);
    if cursor == end {
        return (
            Some((
                Relation {
                    package: package.into(),
                    requirement: VersionRequirement::Any,
                },
                Span::new(start, end),
            )),
            vec![],
        );
    }
    if input.as_bytes()[cursor] != b'(' {
        return (
            None,
            vec![issue(
                RelationParseError::ExpectedVersionClause,
                cursor,
                char_end(input, cursor),
            )],
        );
    }
    let open = cursor;
    let mut issues = Vec::new();
    let close = input[open + 1..end].find(')').map(|i| open + 1 + i);
    let clause_end = close.unwrap_or(end);
    if close.is_none() {
        issues.push(issue(
            RelationParseError::MissingClosingParenthesis,
            end,
            end,
        ));
    }
    if let Some(close) = close {
        cursor = skip_ws(input, close + 1, end);
        if cursor != end {
            issues.push(issue(
                RelationParseError::UnexpectedTrailingText,
                cursor,
                end,
            ));
        }
    }
    let inner = skip_ws(input, open + 1, clause_end);
    let Some((op, op_len)) = parse_operator(&input[inner..clause_end]) else {
        let op_end = input[inner..clause_end]
            .find(char::is_whitespace)
            .map_or(clause_end, |i| inner + i);
        issues.push(issue(
            if inner == clause_end {
                RelationParseError::MissingOperator
            } else {
                RelationParseError::InvalidOperator
            },
            inner,
            op_end,
        ));
        return (None, issues);
    };
    let op_end = inner + op_len;
    if op_end == clause_end {
        issues.push(issue(RelationParseError::MissingVersion, op_end, op_end));
        return (None, issues);
    }
    if !input[op_end..clause_end]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        issues.push(issue(
            RelationParseError::MissingOperatorWhitespace,
            op_end,
            char_end(input, op_end),
        ));
    }
    let version_start = skip_ws(input, op_end, clause_end);
    let version_end = clause_end
        - (input[version_start..clause_end].len()
            - input[version_start..clause_end].trim_end().len());
    if version_start == version_end {
        issues.push(issue(
            RelationParseError::MissingVersion,
            version_start,
            version_start,
        ));
        return (None, issues);
    }
    let value = match RequirementVersion::parse(&input[version_start..version_end]) {
        Ok(value) => value,
        Err(error) => {
            let span = match &error {
                RequirementVersionParseError::Version(error) => error.span(),
                RequirementVersionParseError::Revision(_) => None,
            }
            .map_or(Span::new(version_start, version_end), |s| {
                Span::new(version_start + s.start(), version_start + s.end())
            });
            issues.push(PositionedRelationParseError {
                error: RelationParseError::InvalidVersion(error),
                span,
            });
            return (None, issues);
        }
    };
    if !issues.is_empty() {
        return (None, issues);
    }
    let requirement = match op {
        "<" => VersionRequirement::LessThan(value),
        "<=" => VersionRequirement::LessThanEqual(value),
        "==" => VersionRequirement::Equal(value),
        "!=" => VersionRequirement::NotEqual(value),
        ">" => VersionRequirement::GreaterThan(value),
        ">=" => VersionRequirement::GreaterThanEqual(value),
        _ => unreachable!(),
    };
    (
        Some((
            Relation {
                package: package.into(),
                requirement,
            },
            Span::new(start, end),
        )),
        issues,
    )
}

fn segments(input: &str) -> Vec<(usize, usize)> {
    let mut start = 0;
    let mut segments = input
        .match_indices(',')
        .map(|(end, _)| {
            let segment = (start, end);
            start = end + 1;
            segment
        })
        .collect::<Vec<_>>();
    segments.push((start, input.len()));
    segments
}
fn trim_bounds(input: &str, start: usize, end: usize) -> (usize, usize) {
    let s = &input[start..end];
    (
        start + s.len() - s.trim_start().len(),
        end - (s.len() - s.trim_end().len()),
    )
}
fn skip_ws(input: &str, mut at: usize, end: usize) -> usize {
    while at < end {
        let c = input[at..].chars().next().expect("cursor is in bounds");
        if !c.is_whitespace() {
            break;
        }
        at += c.len_utf8();
    }
    at
}
fn char_end(input: &str, at: usize) -> usize {
    at + input[at..].chars().next().map_or(0, char::len_utf8)
}
fn issue(error: RelationParseError, start: usize, end: usize) -> PositionedRelationParseError {
    PositionedRelationParseError {
        error,
        span: Span::new(start, end),
    }
}
fn parse_operator(input: &str) -> Option<(&'static str, usize)> {
    [">=", "<=", "==", "!=", ">", "<"]
        .into_iter()
        .find(|op| input.starts_with(op))
        .map(|op| (op, op.len()))
}
fn valid_package(package: &str) -> bool {
    !package.is_empty()
        && !package
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, ',' | '(' | ')'))
}
