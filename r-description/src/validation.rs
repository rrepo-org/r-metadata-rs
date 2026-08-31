//! General DESCRIPTION validation, separate from parsing.

use std::fmt::Display;

use crate::{Description, SourceSpan, SyntaxDiagnosticKind};

const COLLECTION_FIELDS: &[&str] = &[
    "Depends",
    "Imports",
    "Suggests",
    "Enhances",
    "LinkingTo",
    "URL",
    "Additional_repositories",
    "Remotes",
    "VignetteBuilder",
];

/// Validation issue severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// The document does not meet a DESCRIPTION requirement.
    Error,
    /// Suspicious but still interpretable input.
    Warning,
}

/// A validation issue with stable identity and source location.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidationIssue {
    code: &'static str,
    severity: Severity,
    message: String,
    span: SourceSpan,
}

impl ValidationIssue {
    /// Stable machine-readable code.
    pub const fn code(&self) -> &'static str {
        self.code
    }
    /// Severity.
    pub const fn severity(&self) -> Severity {
        self.severity
    }
    /// Human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Half-open source byte span.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// Complete validation result in deterministic source/check order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Validation {
    issues: Vec<ValidationIssue>,
}

impl Validation {
    /// Returns all validation issues.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }
    /// Returns whether no error-severity issue exists.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }
    /// Consumes the result and returns its issues.
    pub fn into_issues(self) -> Vec<ValidationIssue> {
        self.issues
    }
}

impl Description {
    /// Validates DESCRIPTION structure and documented metadata value syntax.
    ///
    /// This deliberately excludes CRAN policy, style, and package-directory
    /// checks.
    pub fn validate(&self) -> Validation {
        let mut issues = Vec::new();
        for diagnostic in self.diagnostics() {
            let code = match diagnostic.kind() {
                SyntaxDiagnosticKind::MalformedField => "syntax.malformed-field",
                SyntaxDiagnosticKind::OrphanContinuation => "syntax.orphan-continuation",
                SyntaxDiagnosticKind::InvalidFieldName => "syntax.invalid-field-name",
            };
            push(
                &mut issues,
                code,
                Severity::Error,
                diagnostic.message(),
                diagnostic.span(),
            );
        }
        let records = self.records().collect::<Vec<_>>();
        if records.len() != 1 {
            push(
                &mut issues,
                "record-count",
                Severity::Error,
                format!("expected exactly one record, found {}", records.len()),
                records
                    .first()
                    .map_or(SourceSpan::new(0, 0), crate::Record::source_range),
            );
        }
        let Some(record) = records.first() else {
            return Validation { issues };
        };

        for name in ["Package", "Version", "Title", "Description", "License"] {
            if record.last_field(name).is_none() {
                push(
                    &mut issues,
                    "missing-field",
                    Severity::Error,
                    format!("required field {name} is missing"),
                    record.source_range(),
                );
            }
        }
        if record.last_field("Authors@R").is_none() {
            for name in ["Author", "Maintainer"] {
                if record.last_field(name).is_none() {
                    push(
                        &mut issues,
                        "missing-field",
                        Severity::Error,
                        format!("required field {name} is missing when Authors@R is absent"),
                        record.source_range(),
                    );
                }
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        for field in record.fields() {
            if let Some(name) = field.name()
                && !COLLECTION_FIELDS.contains(&name.as_str())
                && !seen.insert(name.clone())
            {
                push(
                    &mut issues,
                    "duplicate-scalar",
                    Severity::Warning,
                    format!("scalar field {name} is declared more than once"),
                    field.source_range(),
                );
            }
        }

        if let Some(field) = record.last_field("Package") {
            let value = field.value();
            if !valid_package_name(value.as_str()) {
                push(
                    &mut issues,
                    "invalid-package-name",
                    Severity::Error,
                    "package name must have at least two ASCII characters, start with a letter, contain only letters, digits, or '.', and not end with '.'",
                    value.source_range(),
                );
            }
        }
        if let Some(Err(error)) = self.version_parsed() {
            typed(
                &mut issues,
                "invalid-version",
                error,
                record.last_field("Version"),
            );
        }
        validate_collections(self, &mut issues);
        validate_scalars(self, record, &mut issues);
        Validation { issues }
    }
}

fn validate_collections(description: &Description, issues: &mut Vec<ValidationIssue>) {
    collection_issues(
        issues,
        "invalid-relation",
        description.depends_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-relation",
        description.imports_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-relation",
        description.suggests_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-relation",
        description.enhances_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-relation",
        description.linking_to_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-relation",
        description.vignette_builder_parsed().issues(),
    );
    collection_issues(issues, "invalid-url", description.urls_parsed().issues());
    collection_issues(
        issues,
        "invalid-url",
        description.bug_reports_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-url",
        description.additional_repositories_parsed().issues(),
    );
    collection_issues(
        issues,
        "invalid-remote",
        description.remotes_parsed().issues(),
    );
}

fn validate_scalars(
    description: &Description,
    record: &crate::Record,
    issues: &mut Vec<ValidationIssue>,
) {
    macro_rules! scalar {
        ($name:literal, $code:literal, $value:expr) => {
            if let Some(Err(error)) = $value {
                typed(issues, $code, error, record.last_field($name));
            }
        };
    }
    scalar!(
        "NeedsCompilation",
        "invalid-logical",
        description.needs_compilation_parsed()
    );
    scalar!("Biarch", "invalid-logical", description.biarch_parsed());
    scalar!(
        "LazyData",
        "invalid-logical",
        description.lazy_data_parsed()
    );
    scalar!(
        "LazyLoad",
        "invalid-logical",
        description.lazy_load_parsed()
    );
    scalar!(
        "ByteCompile",
        "invalid-logical",
        description.byte_compile_parsed()
    );
    scalar!(
        "KeepSource",
        "invalid-logical",
        description.keep_source_parsed()
    );
    scalar!("UseLTO", "invalid-logical", description.use_lto_parsed());
    scalar!(
        "StagedInstall",
        "invalid-logical",
        description.staged_install_parsed()
    );
    scalar!("ZipData", "invalid-logical", description.zip_data_parsed());
    scalar!(
        "BuildVignettes",
        "invalid-logical",
        description.build_vignettes_parsed()
    );
    scalar!(
        "License_is_FOSS",
        "invalid-logical",
        description.license_is_foss_parsed()
    );
    scalar!(
        "License_restricts_use",
        "invalid-logical",
        description.license_restricts_use_parsed()
    );
    scalar!("OS_type", "invalid-os-type", description.os_type_parsed());
    scalar!(
        "Priority",
        "invalid-priority",
        description.priority_parsed()
    );
}

fn valid_package_name(name: &str) -> bool {
    name.len() >= 2
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
        && !name.ends_with('.')
}

fn collection_issues<E: Display>(
    output: &mut Vec<ValidationIssue>,
    code: &'static str,
    issues: &[crate::CollectionIssue<E>],
) {
    for issue in issues {
        push(
            output,
            code,
            Severity::Error,
            issue.error.to_string(),
            issue.field_span,
        );
    }
}

fn typed<E: Display>(
    output: &mut Vec<ValidationIssue>,
    code: &'static str,
    error: E,
    field: Option<crate::Field>,
) {
    push(
        output,
        code,
        Severity::Error,
        error.to_string(),
        field.map_or(SourceSpan::new(0, 0), |value| value.value().source_range()),
    );
}

fn push(
    output: &mut Vec<ValidationIssue>,
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    span: SourceSpan,
) {
    output.push(ValidationIssue {
        code,
        severity,
        message: message.into(),
        span,
    });
}
