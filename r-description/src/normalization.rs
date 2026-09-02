//! Deterministic whole-document DESCRIPTION normalization.

use std::{collections::BTreeSet, error::Error, fmt};

use r_dcf_syntax::{FieldName, FormatStyle, LineEnding, LogicalValue, SourceSpan};
use r_metadata::{AdditionalRepositories, RelationList, RemoteList, UrlList};

use crate::{Description, Field, SyntaxDiagnosticKind, schema};

const WIDTH: usize = 80;

/// One reason a DESCRIPTION document could not be normalized safely.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizationDiagnostic {
    code: &'static str,
    message: String,
    span: SourceSpan,
}

impl NormalizationDiagnostic {
    /// Stable machine-readable diagnostic code.
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Human-readable explanation.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Half-open source byte span.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// Atomic whole-document normalization failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationError {
    diagnostics: Vec<NormalizationDiagnostic>,
}

impl NormalizationError {
    /// Returns all blocking diagnostics in deterministic order.
    pub fn diagnostics(&self) -> &[NormalizationDiagnostic] {
        &self.diagnostics
    }

    /// Consumes the error and returns all blocking diagnostics.
    pub fn into_diagnostics(self) -> Vec<NormalizationDiagnostic> {
        self.diagnostics
    }
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "DESCRIPTION cannot be normalized safely ({} diagnostic{})",
            self.diagnostics.len(),
            if self.diagnostics.len() == 1 { "" } else { "s" }
        )
    }
}

impl Error for NormalizationError {}

#[derive(Debug)]
struct InputField {
    name: String,
    value: String,
    span: SourceSpan,
    source_index: usize,
}

#[derive(Debug)]
struct OutputField {
    name: String,
    value: String,
    span: SourceSpan,
    source_index: usize,
}

impl Description {
    /// Normalizes the complete DESCRIPTION document into one canonical form.
    ///
    /// The operation is atomic. It returns diagnostics instead of output when
    /// malformed syntax, multiple records, or invalid structured collections
    /// make a semantics-preserving transformation unsafe.
    ///
    /// # Errors
    ///
    /// Returns every normalization blocker found in deterministic order.
    pub fn normalize(&self) -> Result<Self, NormalizationError> {
        let mut diagnostics = syntax_diagnostics(self);
        let records = self.records().collect::<Vec<_>>();
        if records.len() != 1 {
            diagnostics.push(NormalizationDiagnostic {
                code: "record-count",
                message: format!("expected exactly one record, found {}", records.len()),
                span: records
                    .first()
                    .map_or(SourceSpan::new(0, 0), crate::Record::source_range),
            });
        }

        let Some(record) = records.first() else {
            return Err(NormalizationError { diagnostics });
        };
        let fields = record
            .fields()
            .enumerate()
            .filter_map(|(source_index, field)| input_field(&field, source_index))
            .collect::<Vec<_>>();

        validate_values(&fields, &mut diagnostics);
        if !diagnostics.is_empty() {
            return Err(NormalizationError { diagnostics });
        }

        let mut output = normalize_fields(&fields);
        if output.is_empty() {
            return Err(NormalizationError {
                diagnostics: vec![NormalizationDiagnostic {
                    code: "empty-record",
                    message: "normalization would remove every field from the record".to_owned(),
                    span: record.source_range(),
                }],
            });
        }
        output.sort_by(|left, right| {
            schema::order(&left.name)
                .cmp(&schema::order(&right.name))
                .then_with(|| left.source_index.cmp(&right.source_index))
        });

        let style = FormatStyle {
            line_ending: LineEnding::Lf,
            continuation_indent: "    ".to_owned(),
            space_after_colon: true,
        };
        let mut rendered = Vec::with_capacity(output.len());
        for field in output {
            let Ok(name) = FieldName::new(&field.name) else {
                return Err(generated_value_error(
                    "invalid-normalized-field-name",
                    "normalized field name is not valid DCF syntax",
                    field.span,
                ));
            };
            let Ok(value) = LogicalValue::new(field.value) else {
                return Err(generated_value_error(
                    "invalid-normalized-value",
                    "normalized field value cannot be represented in DCF syntax",
                    field.span,
                ));
            };
            rendered.push(r_dcf_syntax::field(&name, &value, &style));
        }
        let mut source = r_dcf_syntax::record(&rendered, &style);
        source.push('\n');
        Ok(Self::parse(&source))
    }
}

fn syntax_diagnostics(description: &Description) -> Vec<NormalizationDiagnostic> {
    description
        .diagnostics()
        .iter()
        .map(|diagnostic| NormalizationDiagnostic {
            code: match diagnostic.kind() {
                SyntaxDiagnosticKind::MalformedField => "syntax.malformed-field",
                SyntaxDiagnosticKind::OrphanContinuation => "syntax.orphan-continuation",
                SyntaxDiagnosticKind::InvalidFieldName => "syntax.invalid-field-name",
            },
            message: diagnostic.message().to_owned(),
            span: diagnostic.span(),
        })
        .collect()
}

fn generated_value_error(
    code: &'static str,
    message: &'static str,
    span: SourceSpan,
) -> NormalizationError {
    NormalizationError {
        diagnostics: vec![NormalizationDiagnostic {
            code,
            message: message.to_owned(),
            span,
        }],
    }
}

fn input_field(field: &Field, source_index: usize) -> Option<InputField> {
    Some(InputField {
        name: field.name()?,
        value: field.value().into_string(),
        span: field.source_range(),
        source_index,
    })
}

fn validate_values(fields: &[InputField], diagnostics: &mut Vec<NormalizationDiagnostic>) {
    for field in fields {
        if LogicalValue::new(&field.value).is_err() {
            push_invalid(
                diagnostics,
                "invalid-logical-value",
                "field value cannot be represented canonically",
                field,
            );
            continue;
        }
        match schema::field_kind(&field.name) {
            Some(schema::FieldKind::Relations) => {
                for issue in RelationList::parse(&field.value).issues() {
                    push_invalid(diagnostics, "invalid-collection", issue.to_string(), field);
                }
            }
            Some(schema::FieldKind::Urls) => {
                for issue in UrlList::parse(&field.value).issues() {
                    push_invalid(diagnostics, "invalid-collection", issue.to_string(), field);
                }
            }
            Some(schema::FieldKind::Repositories) => {
                for issue in AdditionalRepositories::parse(&field.value).issues() {
                    push_invalid(diagnostics, "invalid-collection", issue.to_string(), field);
                }
            }
            Some(schema::FieldKind::Remotes) => {
                for issue in RemoteList::parse(&field.value).issues() {
                    push_invalid(diagnostics, "invalid-collection", issue.to_string(), field);
                }
            }
            Some(schema::FieldKind::Scalar | schema::FieldKind::Ordered) | None => {}
        }
    }
}

fn push_invalid(
    diagnostics: &mut Vec<NormalizationDiagnostic>,
    code: &'static str,
    message: impl Into<String>,
    field: &InputField,
) {
    diagnostics.push(NormalizationDiagnostic {
        code,
        message: format!("{}: {}", field.name, message.into()),
        span: field.span,
    });
}

fn normalize_fields(fields: &[InputField]) -> Vec<OutputField> {
    let mut output = Vec::new();
    let names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    for name in names {
        let declarations = fields
            .iter()
            .filter(|field| field.name == name)
            .collect::<Vec<_>>();
        match schema::field_kind(name) {
            Some(schema::FieldKind::Relations) => {
                let values = declarations
                    .iter()
                    .flat_map(|field| RelationList::parse(&field.value).into_parts().0)
                    .map(|entry| entry.value)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>();
                push_collection(&mut output, name, &values, &declarations);
            }
            Some(schema::FieldKind::Urls) => {
                let values = declarations
                    .iter()
                    .flat_map(|field| UrlList::parse(&field.value).into_parts().0)
                    .map(|entry| entry.value.to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                push_collection(&mut output, name, &values, &declarations);
            }
            Some(schema::FieldKind::Repositories) => {
                let values = declarations
                    .iter()
                    .flat_map(|field| AdditionalRepositories::parse(&field.value).into_parts().0)
                    .map(|entry| entry.value.to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                push_collection(&mut output, name, &values, &declarations);
            }
            Some(schema::FieldKind::Remotes) => {
                let values = declarations
                    .iter()
                    .flat_map(|field| RemoteList::parse(&field.value).into_parts().0)
                    .map(|entry| entry.value.to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                push_collection(&mut output, name, &values, &declarations);
            }
            Some(schema::FieldKind::Scalar | schema::FieldKind::Ordered) => {
                if let Some(field) = declarations.last() {
                    push_scalar(&mut output, field);
                }
            }
            None => {
                for field in declarations {
                    push_scalar(&mut output, field);
                }
            }
        }
    }
    output
}

fn push_collection(
    output: &mut Vec<OutputField>,
    name: &str,
    values: &[String],
    declarations: &[&InputField],
) {
    if values.is_empty() {
        return;
    }
    let Some(declaration) = declarations.last() else {
        return;
    };
    output.push(OutputField {
        name: name.to_owned(),
        value: format!("\n{}", values.join(",\n")),
        span: declaration.span,
        source_index: declaration.source_index,
    });
}

fn push_scalar(output: &mut Vec<OutputField>, field: &InputField) {
    if field.value.is_empty() && schema::remove_when_empty(&field.name) {
        return;
    }
    output.push(OutputField {
        name: field.name.clone(),
        value: if schema::is_prose(&field.name) {
            wrap(&field.name, &field.value)
        } else {
            field.value.clone()
        },
        span: field.span,
        source_index: field.source_index,
    });
}

fn wrap(name: &str, value: &str) -> String {
    let words = value.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return String::new();
    }
    let first_width = WIDTH.saturating_sub(name.len() + 2).max(1);
    let continued_width = WIDTH.saturating_sub(4).max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut width = first_width;
    for word in words {
        if word != "."
            && !line.is_empty()
            && line.chars().count() + 1 + word.chars().count() > width
        {
            lines.push(std::mem::take(&mut line));
            width = continued_width;
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    lines.join("\n")
}
