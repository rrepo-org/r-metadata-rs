//! URL collection values.

use crate::{Span, Spanned, Url};

/// A URL and the span of its source entry.
pub type SpannedUrl = Spanned<Url>;

/// Recovered `URL` field value, where commas or whitespace delimit entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UrlList {
    entries: Vec<SpannedUrl>,
    issues: Vec<PositionedUrlParseError>,
}

impl UrlList {
    /// Parses a `URL` value and retains valid entries alongside all issues.
    pub fn parse(input: &str) -> Self {
        parse_urls(input, true).into()
    }
    /// Returns valid URLs in source order.
    pub fn entries(&self) -> &[SpannedUrl] {
        &self.entries
    }
    /// Returns positioned issues in source order.
    pub fn issues(&self) -> &[PositionedUrlParseError] {
        &self.issues
    }
    /// Splits this result into owned valid entries and issues.
    pub fn into_parts(self) -> (Vec<SpannedUrl>, Vec<PositionedUrlParseError>) {
        (self.entries, self.issues)
    }
}

/// Recovered `Additional_repositories` value, where only commas delimit entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdditionalRepositories {
    entries: Vec<SpannedUrl>,
    issues: Vec<PositionedUrlParseError>,
}

impl AdditionalRepositories {
    /// Parses an `Additional_repositories` value.
    pub fn parse(input: &str) -> Self {
        parse_urls(input, false).into()
    }
    /// Returns valid repository URLs in source order.
    pub fn entries(&self) -> &[SpannedUrl] {
        &self.entries
    }
    /// Returns positioned issues in source order.
    pub fn issues(&self) -> &[PositionedUrlParseError] {
        &self.issues
    }
    /// Splits this result into owned valid entries and issues.
    pub fn into_parts(self) -> (Vec<SpannedUrl>, Vec<PositionedUrlParseError>) {
        (self.entries, self.issues)
    }
}

impl From<(Vec<SpannedUrl>, Vec<PositionedUrlParseError>)> for UrlList {
    fn from((entries, issues): (Vec<SpannedUrl>, Vec<PositionedUrlParseError>)) -> Self {
        Self { entries, issues }
    }
}
impl From<(Vec<SpannedUrl>, Vec<PositionedUrlParseError>)> for AdditionalRepositories {
    fn from((entries, issues): (Vec<SpannedUrl>, Vec<PositionedUrlParseError>)) -> Self {
        Self { entries, issues }
    }
}

/// A URL collection syntax error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlParseError {
    /// A comma-delimited entry is empty.
    #[error("empty URL entry")]
    Empty,
    /// Whitespace was used where only a comma is permitted.
    #[error("repository URLs must be separated by commas")]
    UnexpectedWhitespace,
    /// A control character occurs in an entry.
    #[error("URL contains a control character")]
    ControlCharacter,
    /// The entry is not an absolute URL.
    #[error("invalid URL: {0}")]
    Invalid(#[source] url::ParseError),
}

/// A URL issue paired with its byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedUrlParseError {
    error: UrlParseError,
    span: Span,
}
impl PositionedUrlParseError {
    /// Returns the typed error.
    pub const fn error(&self) -> &UrlParseError {
        &self.error
    }
    /// Returns the relative input span.
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Alias for [`Self::span`].
    pub const fn range(&self) -> Span {
        self.span
    }
}
impl std::fmt::Display for PositionedUrlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for PositionedUrlParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

fn parse_urls(input: &str, whitespace: bool) -> (Vec<SpannedUrl>, Vec<PositionedUrlParseError>) {
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    let mut start = 0;
    let ends = input
        .match_indices(',')
        .map(|(i, _)| i)
        .chain(std::iter::once(input.len()))
        .collect::<Vec<_>>();
    for (index, &end) in ends.iter().enumerate() {
        if input[start..end].trim().is_empty() && (ends.len() == 1 || index == ends.len() - 1) {
            break;
        }
        if whitespace {
            parse_ws_segment(input, start, end, &mut entries, &mut issues);
        } else {
            parse_comma_segment(input, start, end, &mut entries, &mut issues);
        }
        start = end.saturating_add(1);
    }
    (entries, issues)
}

fn parse_ws_segment(
    input: &str,
    start: usize,
    end: usize,
    entries: &mut Vec<SpannedUrl>,
    issues: &mut Vec<PositionedUrlParseError>,
) {
    let mut token = None;
    for (offset, character) in input[start..end].char_indices() {
        let position = start + offset;
        if character.is_whitespace() {
            if let Some(token_start) = token.take() {
                parse_token(input, token_start, position, entries, issues);
            }
        } else if token.is_none() {
            token = Some(position);
        }
    }
    if let Some(token_start) = token {
        parse_token(input, token_start, end, entries, issues);
    } else if input[start..end].trim().is_empty() {
        let point = start + input[start..end].len() - input[start..end].trim_start().len();
        issues.push(url_issue(UrlParseError::Empty, point, point));
    }
}

fn parse_comma_segment(
    input: &str,
    start: usize,
    end: usize,
    entries: &mut Vec<SpannedUrl>,
    issues: &mut Vec<PositionedUrlParseError>,
) {
    let text = &input[start..end];
    let left = start + text.len() - text.trim_start().len();
    let right = end - (text.len() - text.trim_end().len());
    if left == right {
        issues.push(url_issue(UrlParseError::Empty, left, left));
        return;
    }
    if let Some((offset, c)) = input[left..right]
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
    {
        let at = left + offset;
        issues.push(url_issue(
            UrlParseError::UnexpectedWhitespace,
            at,
            at + c.len_utf8(),
        ));
        return;
    }
    parse_token(input, left, right, entries, issues);
}

fn parse_token(
    input: &str,
    start: usize,
    end: usize,
    entries: &mut Vec<SpannedUrl>,
    issues: &mut Vec<PositionedUrlParseError>,
) {
    if let Some((offset, c)) = input[start..end]
        .char_indices()
        .find(|(_, c)| c.is_control())
    {
        let at = start + offset;
        issues.push(url_issue(
            UrlParseError::ControlCharacter,
            at,
            at + c.len_utf8(),
        ));
        return;
    }
    match Url::parse(&input[start..end]) {
        Ok(value) => entries.push(Spanned {
            value,
            span: Span::new(start, end),
        }),
        Err(error) => issues.push(url_issue(UrlParseError::Invalid(error), start, end)),
    }
}
fn url_issue(error: UrlParseError, start: usize, end: usize) -> PositionedUrlParseError {
    PositionedUrlParseError {
        error,
        span: Span::new(start, end),
    }
}
