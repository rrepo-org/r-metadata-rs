//! Package remote source specifications.

use crate::{Span, Spanned, Url};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

/// A parsed remote specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Remote {
    /// Optional package alias.
    pub package: Option<String>,
    /// Optional custom host qualifier.
    pub host: Option<String>,
    /// Package source.
    pub source: RemoteSource,
}

/// Supported remote source forms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RemoteSource {
    /// An implicit package name.
    Unspecified(String),
    /// CRAN package.
    Cran(CranRemote),
    /// GitHub repository, including implicit `owner/repository` syntax.
    GitHub(HostedGitRemote),
    /// GitLab repository.
    GitLab(HostedGitRemote),
    /// Bitbucket repository.
    Bitbucket(HostedGitRemote),
    /// Generic Git repository.
    Git(GenericGitRemote),
    /// Package archive URL.
    Url(UrlRemote),
    /// Local path.
    Local(LocalRemote),
    /// Subversion repository.
    Svn(SvnRemote),
    /// Bioconductor Git source.
    Bioconductor(BioconductorRemote),
    /// Unrecognized explicit source type.
    Unknown(UnknownRemote),
}

/// CRAN remote data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CranRemote {
    /// Package name.
    pub package: String,
}
/// Hosted Git remote data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostedGitRemote {
    /// Owner or top-level namespace.
    pub owner: String,
    /// Repository name; nested GitLab groups are included.
    pub repository: String,
    /// Package subdirectory, when supported.
    pub subdirectory: Option<String>,
    /// Opaque branch, tag, commit, or pull-request reference.
    pub reference: Option<String>,
}
/// Generic Git remote data. Debug output redacts recognized credentials.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GenericGitRemote {
    /// Clone URL, including scp syntax.
    pub url: String,
    /// Optional opaque reference.
    pub reference: Option<String>,
}
/// Typed URL remote data. Debug output redacts URL credentials.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrlRemote {
    /// Archive URL.
    pub url: Url,
}
/// Local remote data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalRemote {
    /// Filesystem path.
    pub path: String,
}
/// Subversion remote data. Debug output redacts URL credentials.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SvnRemote {
    /// Repository URL.
    pub url: Url,
}
/// Bioconductor credentials. Debug output always redacts both fields.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BioconductorCredentials {
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}
/// Bioconductor remote data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BioconductorRemote {
    /// Optional credentials.
    pub credentials: Option<BioconductorCredentials>,
    /// Optional release name or version.
    pub release: Option<String>,
    /// Package name.
    pub package: String,
    /// Optional repository reference.
    pub reference: Option<String>,
}
/// Unknown explicit remote data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnknownRemote {
    /// Source kind with original case.
    pub kind: String,
    /// Uninterpreted payload.
    pub payload: String,
}

impl std::fmt::Debug for GenericGitRemote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericGitRemote")
            .field("url", &RedactedText(&self.url))
            .field("reference", &self.reference)
            .finish()
    }
}
impl std::fmt::Debug for UrlRemote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UrlRemote")
            .field("url", &RedactedUrl(&self.url))
            .finish()
    }
}
impl std::fmt::Debug for SvnRemote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvnRemote")
            .field("url", &RedactedUrl(&self.url))
            .finish()
    }
}
impl std::fmt::Debug for BioconductorCredentials {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BioconductorCredentials")
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .finish()
    }
}
struct RedactedUrl<'a>(&'a Url);
impl std::fmt::Debug for RedactedUrl<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut url = self.0.clone();
        if !url.username().is_empty() || url.password().is_some() {
            let _ = url.set_username("[REDACTED]");
            let _ = url.set_password(Some("[REDACTED]"));
        }
        std::fmt::Debug::fmt(url.as_str(), f)
    }
}
struct RedactedText<'a>(&'a str);
impl std::fmt::Debug for RedactedText<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(url) = Url::parse(self.0) {
            return std::fmt::Debug::fmt(&RedactedUrl(&url), f);
        }
        if let Some((credentials, remote)) = self
            .0
            .split_once('@')
            .filter(|(left, _)| left.contains(':'))
        {
            let _ = credentials;
            return std::fmt::Debug::fmt(&format!("[REDACTED]@{remote}"), f);
        }
        std::fmt::Debug::fmt(self.0, f)
    }
}

/// A remote parse reason. Variants never retain the remote source text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RemoteParseError {
    /// Empty entry.
    #[error("empty remote specification")]
    EmptySpecification,
    /// Whitespace inside an entry.
    #[error("whitespace is not allowed in a remote specification")]
    Whitespace,
    /// A standalone parser received a comma.
    #[error("multiple remote specifications found")]
    MultipleSpecifications,
    /// Control character inside an entry.
    #[error("remote specification contains a control character")]
    ControlCharacter,
    /// Malformed explicit prefix.
    #[error("malformed remote prefix")]
    MalformedPrefix,
    /// Missing explicit source kind.
    #[error("remote prefix is missing a source kind")]
    MissingKind,
    /// Missing custom host.
    #[error("remote prefix is missing a host")]
    MissingHost,
    /// Missing source payload.
    #[error("remote specification is missing its payload")]
    MissingPayload,
    /// Invalid package alias.
    #[error("invalid remote package alias")]
    InvalidAlias,
    /// Missing hosted owner/repository pair.
    #[error("remote is missing a repository owner or name")]
    MissingRepository,
    /// Empty hosted path component.
    #[error("remote repository path contains an empty component")]
    MissingPathComponent,
    /// Reference delimiter without a reference.
    #[error("remote reference is missing")]
    MissingReference,
    /// Missing Git URL.
    #[error("Git remote is missing its URL")]
    MissingGitUrl,
    /// Invalid absolute URL. `url::ParseError` contains no input text.
    #[error("invalid remote URL: {0}")]
    InvalidUrl(#[source] url::ParseError),
    /// Malformed Bioconductor reference.
    #[error("malformed Bioconductor reference")]
    MalformedBioconductorReference,
    /// Malformed Bioconductor credentials.
    #[error("malformed Bioconductor credentials")]
    MalformedBioconductorCredentials,
    /// Malformed release/package source.
    #[error("malformed Bioconductor source")]
    MalformedBioconductorSource,
    /// Malformed Bioconductor package.
    #[error("malformed Bioconductor package")]
    MalformedBioconductorPackage,
}

/// A safe remote parse reason and its relative byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedRemoteParseError {
    error: RemoteParseError,
    span: Span,
}
impl PositionedRemoteParseError {
    /// Returns the typed reason.
    pub const fn error(&self) -> &RemoteParseError {
        &self.error
    }
    /// Returns the input span.
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Alias for [`Self::span`].
    pub const fn range(&self) -> Span {
        self.span
    }
}
impl Display for PositionedRemoteParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for PositionedRemoteParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Recovered comma-separated remote collection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteList {
    entries: Vec<Spanned<Remote>>,
    issues: Vec<PositionedRemoteParseError>,
}
impl RemoteList {
    /// Parses all entries and recovers at each comma.
    pub fn parse(input: &str) -> Self {
        if input.trim().is_empty() {
            return Self::default();
        }
        let mut result = Self::default();
        let mut start = 0;
        for end in input
            .match_indices(',')
            .map(|(i, _)| i)
            .chain(std::iter::once(input.len()))
        {
            let segment = &input[start..end];
            let left = start + segment.len() - segment.trim_start().len();
            let right = end - (segment.len() - segment.trim_end().len());
            if left == right {
                result.issues.push(remote_issue(
                    RemoteParseError::EmptySpecification,
                    left,
                    left,
                ));
            } else {
                match parse_remote(&input[left..right]) {
                    Ok(value) => result.entries.push(Spanned {
                        value,
                        span: Span::new(left, right),
                    }),
                    Err(error) => result.issues.push(PositionedRemoteParseError {
                        error: error.error,
                        span: Span::new(left + error.span.start(), left + error.span.end()),
                    }),
                }
            }
            start = end.saturating_add(1);
        }
        result
    }
    /// Returns valid entries in source order.
    pub fn entries(&self) -> &[Spanned<Remote>] {
        &self.entries
    }
    /// Returns all issues in source order.
    pub fn issues(&self) -> &[PositionedRemoteParseError] {
        &self.issues
    }
    /// Splits into owned entries and issues.
    pub fn into_parts(self) -> (Vec<Spanned<Remote>>, Vec<PositionedRemoteParseError>) {
        (self.entries, self.issues)
    }
}

impl FromStr for Remote {
    type Err = PositionedRemoteParseError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        parse_remote(input)
    }
}

fn parse_remote(input: &str) -> Result<Remote, PositionedRemoteParseError> {
    if input.is_empty() {
        return Err(remote_issue(RemoteParseError::EmptySpecification, 0, 0));
    }
    if let Some((i, c)) = input.char_indices().find(|(_, c)| c.is_whitespace()) {
        return Err(remote_issue(
            RemoteParseError::Whitespace,
            i,
            i + c.len_utf8(),
        ));
    }
    if let Some(i) = input.find(',') {
        return Err(remote_issue(
            RemoteParseError::MultipleSpecifications,
            i,
            i + 1,
        ));
    }
    if let Some((i, c)) = input.char_indices().find(|(_, c)| c.is_control()) {
        return Err(remote_issue(
            RemoteParseError::ControlCharacter,
            i,
            i + c.len_utf8(),
        ));
    }
    let (package, source_start) = parse_alias(input)?;
    let (kind, host, payload, payload_start) = parse_explicit(input, source_start)?;
    let source = match kind.map(str::to_ascii_lowercase).as_deref() {
        None if payload.contains('/') => {
            RemoteSource::GitHub(parse_hosted(payload, payload_start, Hosted::GitHub)?)
        }
        None => RemoteSource::Unspecified(payload.into()),
        Some("cran") => RemoteSource::Cran(CranRemote {
            package: payload.into(),
        }),
        Some("github") => {
            RemoteSource::GitHub(parse_hosted(payload, payload_start, Hosted::GitHub)?)
        }
        Some("gitlab") => {
            RemoteSource::GitLab(parse_hosted(payload, payload_start, Hosted::GitLab)?)
        }
        Some("bitbucket") => {
            RemoteSource::Bitbucket(parse_hosted(payload, payload_start, Hosted::Bitbucket)?)
        }
        Some("git") => RemoteSource::Git(parse_git(payload, payload_start)?),
        Some("url") => RemoteSource::Url(UrlRemote {
            url: parse_url(payload, payload_start)?,
        }),
        Some("local") => RemoteSource::Local(LocalRemote {
            path: payload.into(),
        }),
        Some("svn") => RemoteSource::Svn(SvnRemote {
            url: parse_url(payload, payload_start)?,
        }),
        Some("bioc") => RemoteSource::Bioconductor(parse_bioc(payload, payload_start)?),
        Some(_) => RemoteSource::Unknown(UnknownRemote {
            kind: kind.unwrap_or_default().into(),
            payload: payload.into(),
        }),
    };
    Ok(Remote {
        package,
        host,
        source,
    })
}

fn parse_alias(input: &str) -> Result<(Option<String>, usize), PositionedRemoteParseError> {
    let Some(equal) = input.find('=') else {
        return Ok((None, 0));
    };
    let syntax = input
        .char_indices()
        .find_map(|(i, c)| matches!(c, '/' | ':' | '@' | '#').then_some(i))
        .unwrap_or(input.len());
    if equal > syntax {
        return Ok((None, 0));
    }
    let alias = &input[..equal];
    let mut chars = alias.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '.')
    {
        return Err(remote_issue(RemoteParseError::InvalidAlias, 0, equal));
    }
    Ok((Some(alias.into()), equal + 1))
}

type Explicit<'a> = (Option<&'a str>, Option<String>, &'a str, usize);
fn parse_explicit(input: &str, start: usize) -> Result<Explicit<'_>, PositionedRemoteParseError> {
    let source = &input[start..];
    let Some(relative) = source.find("::") else {
        if source.is_empty() {
            return Err(remote_issue(RemoteParseError::MissingPayload, start, start));
        }
        return Ok((None, None, source, start));
    };
    let separator = start + relative;
    let prefix = &input[start..separator];
    let payload_start = separator + 2;
    let payload = &input[payload_start..];
    if prefix.is_empty() {
        return Err(remote_issue(RemoteParseError::MissingKind, start, start));
    }
    if payload.is_empty() {
        return Err(remote_issue(
            RemoteParseError::MissingPayload,
            payload_start,
            payload_start,
        ));
    }
    let mut parts = prefix.split('@');
    let kind = parts.next().unwrap_or_default();
    let host = parts.next();
    if parts.next().is_some() {
        return Err(remote_issue(
            RemoteParseError::MalformedPrefix,
            start,
            separator,
        ));
    }
    if kind.is_empty() {
        return Err(remote_issue(RemoteParseError::MissingKind, start, start));
    }
    let host = match host {
        Some("") => {
            return Err(remote_issue(
                RemoteParseError::MissingHost,
                separator,
                separator,
            ));
        }
        Some(value) => Some(value.into()),
        None => None,
    };
    Ok((Some(kind), host, payload, payload_start))
}

#[derive(Clone, Copy)]
enum Hosted {
    GitHub,
    GitLab,
    Bitbucket,
}
fn parse_hosted(
    payload: &str,
    start: usize,
    kind: Hosted,
) -> Result<HostedGitRemote, PositionedRemoteParseError> {
    let delimiter = payload.char_indices().find(|(_, c)| matches!(c, '@' | '#'));
    let (path, reference) = match delimiter {
        Some((i, _)) if i + 1 < payload.len() => (&payload[..i], Some(payload[i + 1..].into())),
        Some((i, _)) => {
            return Err(remote_issue(
                RemoteParseError::MissingReference,
                start + i + 1,
                start + i + 1,
            ));
        }
        None => (payload, None),
    };
    let path = path.trim_end_matches('/');
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() < 2 {
        return Err(remote_issue(
            RemoteParseError::MissingRepository,
            start,
            start + path.len(),
        ));
    }
    if let Some((index, _)) = parts.iter().enumerate().find(|(_, part)| part.is_empty()) {
        let offset = parts[..index]
            .iter()
            .map(|part| part.len() + 1)
            .sum::<usize>();
        return Err(remote_issue(
            RemoteParseError::MissingPathComponent,
            start + offset,
            start + offset,
        ));
    }
    let (repository, subdirectory) = match kind {
        Hosted::GitLab => (parts[1..].join("/"), None),
        Hosted::GitHub | Hosted::Bitbucket => (
            parts[1].into(),
            (parts.len() > 2).then(|| parts[2..].join("/")),
        ),
    };
    Ok(HostedGitRemote {
        owner: parts[0].into(),
        repository,
        subdirectory,
        reference,
    })
}

fn parse_git(payload: &str, start: usize) -> Result<GenericGitRemote, PositionedRemoteParseError> {
    if payload.is_empty() {
        return Err(remote_issue(RemoteParseError::MissingGitUrl, start, start));
    }
    let path_start = if let Some(scheme) = payload.find("://") {
        payload[scheme + 3..]
            .find('/')
            .map_or(payload.len(), |i| scheme + 3 + i)
    } else if let Some(auth) = payload.find('@') {
        payload[auth + 1..].find(':').map_or(0, |i| auth + 2 + i)
    } else {
        0
    };
    let split = payload[path_start..].find('@').map(|i| path_start + i);
    if split == Some(0) {
        return Err(remote_issue(RemoteParseError::MissingGitUrl, start, start));
    }
    if split == Some(payload.len() - 1) {
        return Err(remote_issue(
            RemoteParseError::MissingReference,
            start + payload.len(),
            start + payload.len(),
        ));
    }
    let (url, reference) = split.map_or_else(
        || (payload.into(), None),
        |i| (payload[..i].into(), Some(payload[i + 1..].into())),
    );
    Ok(GenericGitRemote { url, reference })
}
fn parse_url(payload: &str, start: usize) -> Result<Url, PositionedRemoteParseError> {
    Url::parse(payload).map_err(|error| {
        remote_issue(
            RemoteParseError::InvalidUrl(error),
            start,
            start + payload.len(),
        )
    })
}

fn parse_bioc(
    payload: &str,
    start: usize,
) -> Result<BioconductorRemote, PositionedRemoteParseError> {
    let (source, reference) = match payload.split_once('#') {
        Some((source, reference)) if !source.is_empty() && !reference.is_empty() => {
            (source, Some(reference.into()))
        }
        Some((source, _)) => {
            return Err(remote_issue(
                RemoteParseError::MalformedBioconductorReference,
                start + source.len(),
                start + payload.len(),
            ));
        }
        None => (payload, None),
    };
    let (credentials, source) = match source.find('@') {
        Some(at) if source[..at].contains(':') => {
            let (username, password) = source[..at].split_once(':').expect("colon was found");
            if username.is_empty() || password.is_empty() {
                return Err(remote_issue(
                    RemoteParseError::MalformedBioconductorCredentials,
                    start,
                    start + at,
                ));
            }
            (
                Some(BioconductorCredentials {
                    username: username.into(),
                    password: password.into(),
                }),
                &source[at + 1..],
            )
        }
        _ => (None, source),
    };
    let source_start = start + payload.len()
        - reference
            .as_ref()
            .map_or(0, |value: &String| value.len() + 1)
        - source.len();
    let (release, package) = match source.split_once('/') {
        Some((release, package)) if !release.is_empty() && !package.is_empty() => {
            (Some(release.into()), package)
        }
        Some(_) => {
            return Err(remote_issue(
                RemoteParseError::MalformedBioconductorSource,
                source_start,
                source_start + source.len(),
            ));
        }
        None => (None, source),
    };
    if package.is_empty() || package.contains('/') {
        let at = source_start + release.as_ref().map_or(0, |value: &String| value.len() + 1);
        return Err(remote_issue(
            RemoteParseError::MalformedBioconductorPackage,
            at,
            at + package.len(),
        ));
    }
    Ok(BioconductorRemote {
        credentials,
        release,
        package: package.into(),
        reference,
    })
}

impl Display for Remote {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(package) = &self.package {
            write!(f, "{package}=")?;
        }
        match &self.source {
            RemoteSource::Unspecified(value) => f.write_str(value),
            RemoteSource::Cran(value) => {
                prefix(f, "cran", self.host.as_deref())?;
                f.write_str(&value.package)
            }
            RemoteSource::GitHub(value) => {
                prefix(f, "github", self.host.as_deref())?;
                hosted(f, value)
            }
            RemoteSource::GitLab(value) => {
                prefix(f, "gitlab", self.host.as_deref())?;
                hosted(f, value)
            }
            RemoteSource::Bitbucket(value) => {
                prefix(f, "bitbucket", self.host.as_deref())?;
                hosted(f, value)
            }
            RemoteSource::Git(value) => {
                prefix(f, "git", self.host.as_deref())?;
                f.write_str(&value.url)?;
                reference(f, value.reference.as_deref(), '@')
            }
            RemoteSource::Url(value) => {
                prefix(f, "url", self.host.as_deref())?;
                value.url.fmt(f)
            }
            RemoteSource::Local(value) => {
                prefix(f, "local", self.host.as_deref())?;
                f.write_str(&value.path)
            }
            RemoteSource::Svn(value) => {
                prefix(f, "svn", self.host.as_deref())?;
                value.url.fmt(f)
            }
            RemoteSource::Bioconductor(value) => {
                prefix(f, "bioc", self.host.as_deref())?;
                if let Some(credentials) = &value.credentials {
                    write!(f, "{}:{}@", credentials.username, credentials.password)?;
                }
                if let Some(release) = &value.release {
                    write!(f, "{release}/")?;
                }
                f.write_str(&value.package)?;
                reference(f, value.reference.as_deref(), '#')
            }
            RemoteSource::Unknown(value) => {
                f.write_str(&value.kind)?;
                if let Some(host) = &self.host {
                    write!(f, "@{host}")?;
                }
                write!(f, "::{}", value.payload)
            }
        }
    }
}
fn prefix(f: &mut Formatter<'_>, kind: &str, host: Option<&str>) -> std::fmt::Result {
    f.write_str(kind)?;
    if let Some(host) = host {
        write!(f, "@{host}")?;
    }
    f.write_str("::")
}
fn hosted(f: &mut Formatter<'_>, value: &HostedGitRemote) -> std::fmt::Result {
    write!(f, "{}/{}", value.owner, value.repository)?;
    if let Some(subdirectory) = &value.subdirectory {
        write!(f, "/{subdirectory}")?;
    }
    reference(f, value.reference.as_deref(), '@')
}
fn reference(f: &mut Formatter<'_>, value: Option<&str>, delimiter: char) -> std::fmt::Result {
    if let Some(value) = value {
        write!(f, "{delimiter}{value}")?;
    }
    Ok(())
}
fn remote_issue(error: RemoteParseError, start: usize, end: usize) -> PositionedRemoteParseError {
    PositionedRemoteParseError {
        error,
        span: Span::new(start, end),
    }
}
