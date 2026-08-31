//! Immutable and transactional text-preserving edits.

use std::{error::Error, fmt, ops::Range};

use crate::{
    Field, FieldName, FormatStyle, LineEnding, LogicalValue, Parse, SourceSpan, make, parse,
};

/// Failure to apply a structured edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The selected record does not exist.
    RecordOutOfBounds {
        /// Requested zero-based record index.
        index: usize,
    },
    /// No case-sensitive matching field exists.
    FieldNotFound {
        /// Requested case-sensitive field name.
        name: String,
    },
}

impl fmt::Display for EditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordOutOfBounds { index } => {
                write!(formatter, "record index {index} is out of bounds")
            }
            Self::FieldNotFound { name } => write!(formatter, "field {name:?} was not found"),
        }
    }
}

impl Error for EditError {}

impl Parse {
    /// Starts a transactional editing session.
    pub fn edit(&self) -> Editor {
        Editor {
            current: self.clone(),
        }
    }

    /// Replaces the last matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn replace_last(
        &self,
        record: usize,
        name: &str,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        replace(self, record, name, value, false)
    }

    /// Replaces all matching fields in one record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn replace_all(
        &self,
        record: usize,
        name: &str,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        replace(self, record, name, value, true)
    }

    /// Removes the last matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn remove_last(&self, record: usize, name: &str) -> Result<Self, EditError> {
        remove(self, record, name, false)
    }

    /// Removes all matching fields in one record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn remove_all(&self, record: usize, name: &str) -> Result<Self, EditError> {
        remove(self, record, name, true)
    }

    /// Inserts a new field after the last matching field in one record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or anchor field does not exist.
    pub fn insert_after(
        &self,
        record: usize,
        after: &str,
        name: &FieldName,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        let selected = self
            .records()
            .nth(record)
            .ok_or(EditError::RecordOutOfBounds { index: record })?;
        let anchor = selected
            .last_field(after)
            .ok_or_else(|| EditError::FieldNotFound {
                name: after.to_owned(),
            })?;
        let raw = anchor.raw_text();
        let ending = final_ending(&raw);
        let style = style_of(&anchor);
        let new_field = make::field(name, value, &style);
        let insertion = if ending.is_empty() {
            format!("{}{new_field}", style.line_ending.as_str())
        } else {
            format!("{new_field}{ending}")
        };
        let offset = anchor.source_range().end;
        Ok(parse(&replace_ranges(
            &self.to_string(),
            &[(SourceSpan::new(offset, offset), insertion)],
        )))
    }
}

/// A chainable editing session. Each operation is atomic; `finish` returns the
/// latest persistent parse.
#[derive(Debug, Clone)]
pub struct Editor {
    current: Parse,
}

impl Editor {
    /// Replaces the last matching field.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn replace_last(
        mut self,
        record: usize,
        name: &str,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        self.current = self.current.replace_last(record, name, value)?;
        Ok(self)
    }

    /// Replaces all matching fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn replace_all(
        mut self,
        record: usize,
        name: &str,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        self.current = self.current.replace_all(record, name, value)?;
        Ok(self)
    }

    /// Inserts a field after the last matching field.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or anchor field does not exist.
    pub fn insert_after(
        mut self,
        record: usize,
        after: &str,
        name: &FieldName,
        value: &LogicalValue,
    ) -> Result<Self, EditError> {
        self.current = self.current.insert_after(record, after, name, value)?;
        Ok(self)
    }

    /// Removes the last matching field.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn remove_last(mut self, record: usize, name: &str) -> Result<Self, EditError> {
        self.current = self.current.remove_last(record, name)?;
        Ok(self)
    }

    /// Removes all matching fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or field does not exist.
    pub fn remove_all(mut self, record: usize, name: &str) -> Result<Self, EditError> {
        self.current = self.current.remove_all(record, name)?;
        Ok(self)
    }

    /// Completes the transaction.
    pub fn finish(self) -> Parse {
        self.current
    }
}

fn replace(
    parse_result: &Parse,
    record_index: usize,
    name: &str,
    value: &LogicalValue,
    all: bool,
) -> Result<Parse, EditError> {
    let record = parse_result
        .records()
        .nth(record_index)
        .ok_or(EditError::RecordOutOfBounds {
            index: record_index,
        })?;
    let mut fields: Vec<_> = record.fields_named(name).collect();
    if fields.is_empty() {
        return Err(EditError::FieldNotFound {
            name: name.to_owned(),
        });
    }
    if !all {
        fields.drain(..fields.len() - 1);
    }
    let replacements: Vec<_> = fields
        .iter()
        .map(|field| (field.source_range(), styled_replacement(field, value)))
        .collect();
    Ok(parse(&replace_ranges(
        &parse_result.to_string(),
        &replacements,
    )))
}

fn remove(
    parse_result: &Parse,
    record_index: usize,
    name: &str,
    all: bool,
) -> Result<Parse, EditError> {
    let record = parse_result
        .records()
        .nth(record_index)
        .ok_or(EditError::RecordOutOfBounds {
            index: record_index,
        })?;
    let mut fields: Vec<_> = record.fields_named(name).collect();
    if fields.is_empty() {
        return Err(EditError::FieldNotFound {
            name: name.to_owned(),
        });
    }
    if !all {
        fields.drain(..fields.len() - 1);
    }
    let replacements: Vec<_> = fields
        .iter()
        .map(|field| (field.source_range(), String::new()))
        .collect();
    Ok(parse(&replace_ranges(
        &parse_result.to_string(),
        &replacements,
    )))
}

fn styled_replacement(field: &Field, value: &LogicalValue) -> String {
    let raw = field.raw_text();
    let ending = final_ending(&raw);
    let name = FieldName::new(field.name().expect("valid fields have names"))
        .expect("parser validates field names");
    let style = style_of(field);
    let first_body = raw.split(['\r', '\n']).next().unwrap_or_default();
    let after_colon = first_body.split_once(':').map_or("", |(_, text)| text);
    let prefix_len = after_colon
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let prefix = &after_colon[..prefix_len];
    let mut lines = value.as_str().split('\n');
    let first = lines.next().unwrap_or_default();
    let mut replacement = format!("{name}:{prefix}{first}");
    for line in lines {
        replacement.push_str(style.line_ending.as_str());
        replacement.push_str(&style.continuation_indent);
        replacement.push_str(if line.is_empty() { "." } else { line });
    }
    replacement.push_str(ending);
    replacement
}

fn style_of(field: &Field) -> FormatStyle {
    let raw = field.raw_text();
    let line_ending = if raw.contains("\r\n") {
        LineEnding::CrLf
    } else if raw.contains('\r') {
        LineEnding::Cr
    } else {
        LineEnding::Lf
    };
    let first_body = raw.split(['\r', '\n']).next().unwrap_or_default();
    let after_colon = first_body.split_once(':').map_or("", |(_, value)| value);
    FormatStyle {
        line_ending,
        continuation_indent: continuation_indent(&raw).unwrap_or_else(|| " ".to_owned()),
        space_after_colon: after_colon.starts_with([' ', '\t']),
    }
}

fn continuation_indent(raw: &str) -> Option<String> {
    raw.split(['\r', '\n']).skip(1).find_map(|line| {
        let length = line
            .bytes()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();
        (length > 0).then(|| line[..length].to_owned())
    })
}

fn final_ending(raw: &str) -> &str {
    if raw.ends_with("\r\n") {
        "\r\n"
    } else if raw.ends_with('\r') {
        "\r"
    } else if raw.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn replace_ranges(source: &str, replacements: &[(SourceSpan, String)]) -> String {
    let mut replacements = replacements.to_vec();
    replacements.sort_unstable_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut output = source.to_owned();
    for (span, replacement) in replacements {
        output.replace_range(Range::<usize>::from(span), &replacement);
    }
    output
}
