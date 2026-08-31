use std::fmt;

use r_dcf_syntax::{DiagnosticKind, SourceSpan};
use r_metadata::RelationList;

use crate::Packages;

/// A stable category of syntax or package-record validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingKind {
    /// A field line has no colon.
    MalformedField,
    /// A continuation has no preceding field.
    OrphanContinuation,
    /// A DCF field name is invalid.
    InvalidFieldName,
    /// A required `Package` field is absent.
    MissingPackage,
    /// A required `Version` field is absent.
    MissingVersion,
    /// A scalar field occurs more than once.
    DuplicateScalarField,
    /// A `Package` value is not a valid package name.
    InvalidPackageName,
    /// A `Version` value is not a valid package version.
    InvalidVersion,
    /// A logical field has an unknown value.
    InvalidLogical,
    /// A `Priority` value is unknown.
    InvalidPriority,
    /// An `OS_type` value is unknown.
    InvalidOsType,
    /// A package relation contains invalid syntax.
    InvalidRelation,
}

/// One record-indexed validation finding and its document byte span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Finding {
    kind: FindingKind,
    record_index: usize,
    span: SourceSpan,
    field_name: Option<String>,
}

impl Finding {
    /// Returns the stable finding category.
    pub const fn kind(&self) -> FindingKind {
        self.kind
    }

    /// Returns the zero-based record index.
    pub const fn record_index(&self) -> usize {
        self.record_index
    }

    /// Returns the document-relative byte span.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the related case-sensitive field name, when applicable.
    pub fn field_name(&self) -> Option<&str> {
        self.field_name.as_deref()
    }

    /// Returns a stable human-readable description.
    pub const fn message(&self) -> &'static str {
        match self.kind {
            FindingKind::MalformedField => "field line is missing a colon",
            FindingKind::OrphanContinuation => "continuation line has no preceding field",
            FindingKind::InvalidFieldName => "invalid field name",
            FindingKind::MissingPackage => "required Package field is missing",
            FindingKind::MissingVersion => "required Version field is missing",
            FindingKind::DuplicateScalarField => "scalar field is duplicated",
            FindingKind::InvalidPackageName => "invalid package name",
            FindingKind::InvalidVersion => "invalid package version",
            FindingKind::InvalidLogical => "invalid logical value",
            FindingKind::InvalidPriority => "invalid package priority",
            FindingKind::InvalidOsType => "invalid OS type",
            FindingKind::InvalidRelation => "invalid package relation",
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "record {}: {} at {}..{}",
            self.record_index,
            self.message(),
            self.span.start,
            self.span.end
        )
    }
}

const RELATION_FIELDS: &[&str] = &["Depends", "Imports", "Suggests", "Enhances", "LinkingTo"];
const LOGICAL_FIELDS: &[&str] = &[
    "NeedsCompilation",
    "License_is_FOSS",
    "License_restricts_use",
    "ByteCompile",
    "KeepSource",
];
const NON_SCALAR_FIELDS: &[&str] = RELATION_FIELDS;

impl Packages {
    /// Validates syntax and field-local repository metadata in source order.
    ///
    /// This performs no CRAN policy checks. Every finding is independent, so a
    /// malformed value in one field does not prevent validation of neighbours.
    pub fn validate(&self) -> Vec<Finding> {
        let records = self.parse.records().collect::<Vec<_>>();
        let mut findings = Vec::new();

        for diagnostic in self.parse.diagnostics() {
            let record_index = record_for_span(&records, diagnostic.span());
            findings.push(Finding {
                kind: match diagnostic.kind() {
                    DiagnosticKind::MalformedField => FindingKind::MalformedField,
                    DiagnosticKind::OrphanContinuation => FindingKind::OrphanContinuation,
                    DiagnosticKind::InvalidFieldName => FindingKind::InvalidFieldName,
                },
                record_index,
                span: diagnostic.span(),
                field_name: None,
            });
        }

        for (record_index, record) in records.iter().enumerate() {
            let point = SourceSpan::new(record.source_range().start, record.source_range().start);
            for (name, kind) in [
                ("Package", FindingKind::MissingPackage),
                ("Version", FindingKind::MissingVersion),
            ] {
                if record.last_field(name).is_none() {
                    findings.push(finding(kind, record_index, point, Some(name)));
                }
            }

            let fields = record.fields().collect::<Vec<_>>();
            for (position, field) in fields.iter().enumerate() {
                let Some(name) = field.name() else { continue };
                let value = field.value();
                if !NON_SCALAR_FIELDS.contains(&name.as_str())
                    && fields[..position]
                        .iter()
                        .any(|previous| previous.name().as_deref() == Some(name.as_str()))
                {
                    findings.push(finding(
                        FindingKind::DuplicateScalarField,
                        record_index,
                        field.source_range(),
                        Some(&name),
                    ));
                }
                validate_value(&mut findings, record_index, &name, &value);
            }
        }
        findings.sort_by_key(|item| (item.span.start, item.span.end, item.kind as u8));
        findings
    }
}

fn validate_value(
    findings: &mut Vec<Finding>,
    record_index: usize,
    name: &str,
    value: &r_dcf_syntax::ValueText,
) {
    let span = value.source_range();
    let invalid = match name {
        "Package" if !valid_package_name(value.as_str()) => Some(FindingKind::InvalidPackageName),
        "Version" if value.as_str().parse::<r_metadata::Version>().is_err() => {
            Some(FindingKind::InvalidVersion)
        }
        "Priority" if value.as_str().parse::<r_metadata::Priority>().is_err() => {
            Some(FindingKind::InvalidPriority)
        }
        "OS_type" if value.as_str().parse::<r_metadata::OsType>().is_err() => {
            Some(FindingKind::InvalidOsType)
        }
        name if LOGICAL_FIELDS.contains(&name)
            && value.as_str().parse::<r_metadata::Logical>().is_err() =>
        {
            Some(FindingKind::InvalidLogical)
        }
        _ => None,
    };
    if let Some(kind) = invalid {
        findings.push(finding(kind, record_index, span, Some(name)));
    }
    if RELATION_FIELDS.contains(&name) {
        for issue in RelationList::parse(value.as_str()).issues() {
            let relative = issue.span();
            findings.push(finding(
                FindingKind::InvalidRelation,
                record_index,
                SourceSpan::new(span.start + relative.start(), span.start + relative.end()),
                Some(name),
            ));
        }
    }
}

fn finding(
    kind: FindingKind,
    record_index: usize,
    span: SourceSpan,
    name: Option<&str>,
) -> Finding {
    Finding {
        kind,
        record_index,
        span,
        field_name: name.map(str::to_owned),
    }
}

fn record_for_span(records: &[r_dcf_syntax::Record], span: SourceSpan) -> usize {
    records
        .iter()
        .position(|record| {
            let range = record.source_range();
            span.start >= range.start && span.start <= range.end
        })
        .unwrap_or_else(|| records.len().saturating_sub(1))
}

pub(crate) fn valid_package_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 2
        && bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'.')
}
