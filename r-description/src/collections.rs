//! Typed whole-field collection edits.

use std::{borrow::Borrow, fmt::Display, str::FromStr};

use r_metadata::{PositionedRemoteParseError, Relation, Remote, Url};

use crate::{
    Description, EditError, FieldName, FormatStyle, InvalidFieldName, InvalidLogicalValue,
    LineEnding, LogicalValue,
};

/// Failure to serialize or apply a typed collection edit.
#[derive(Debug, thiserror::Error)]
pub enum CollectionEditError {
    /// A structured DCF edit failed.
    #[error(transparent)]
    Edit(#[from] EditError),
    /// An internal constant field name was rejected.
    #[error(transparent)]
    FieldName(#[from] InvalidFieldName),
    /// A serialized collection cannot be represented as logical DCF text.
    #[error(transparent)]
    Value(#[from] InvalidLogicalValue),
    /// A constructed remote serialized to invalid remote syntax.
    #[error("remote at collection index {index} is invalid: {source}")]
    Remote {
        /// Zero-based collection index.
        index: usize,
        /// Typed parse failure for the serialized remote.
        #[source]
        source: PositionedRemoteParseError,
    },
}

impl Description {
    /// Sets the complete `Depends` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_depends<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("Depends", values)
    }

    /// Sets the complete `Imports` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_imports<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("Imports", values)
    }

    /// Sets the complete `Suggests` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_suggests<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("Suggests", values)
    }

    /// Sets the complete `Enhances` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_enhances<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("Enhances", values)
    }

    /// Sets the complete `LinkingTo` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_linking_to<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("LinkingTo", values)
    }

    /// Sets the complete `VignetteBuilder` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_vignette_builder<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_relations("VignetteBuilder", values)
    }

    /// Sets the complete `Additional_repositories` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection cannot be serialized or the field
    /// cannot be edited.
    pub fn set_additional_repositories<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Url>,
    {
        let rendered = values
            .into_iter()
            .map(|value| value.borrow().to_string())
            .collect::<Vec<_>>();
        self.set_rendered_collection("Additional_repositories", &rendered)
    }

    /// Sets the complete `Remotes` collection in iteration order.
    ///
    /// # Errors
    ///
    /// Returns an error if a remote serializes to invalid syntax or the field
    /// cannot be edited.
    pub fn set_remotes<I, T>(&self, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Remote>,
    {
        let mut rendered = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let value = value.borrow().to_string();
            Remote::from_str(&value)
                .map_err(|source| CollectionEditError::Remote { index, source })?;
            rendered.push(value);
        }
        self.set_rendered_collection("Remotes", &rendered)
    }

    fn set_relations<I, T>(&self, name: &str, values: I) -> Result<Self, CollectionEditError>
    where
        I: IntoIterator<Item = T>,
        T: Borrow<Relation>,
    {
        self.set_rendered_collection(name, &render(values))
    }

    fn set_rendered_collection(
        &self,
        name: &str,
        values: &[String],
    ) -> Result<Self, CollectionEditError> {
        if values.is_empty() {
            return if self.field(name).is_some() {
                self.remove_all(name).map_err(Into::into)
            } else {
                Ok(self.clone())
            };
        }

        let value = LogicalValue::new(format!("\n{}", values.join(",\n")))?;
        let name = FieldName::new(name)?;
        let style = FormatStyle {
            line_ending: self.collection_line_ending(name.as_str()),
            continuation_indent: "    ".to_owned(),
            space_after_colon: false,
        };
        self.parse
            .set_unique(0, &name, &value, &style)
            .map(Self::from_parse)
            .map_err(Into::into)
    }

    fn collection_line_ending(&self, name: &str) -> LineEnding {
        let raw = self
            .field(name)
            .or_else(|| self.primary_record()?.fields().last())
            .map(|field| field.raw_text());
        let source = raw
            .filter(|raw| raw.contains(['\r', '\n']))
            .unwrap_or_else(|| self.to_string());
        match source.as_str() {
            raw if raw.contains("\r\n") => LineEnding::CrLf,
            raw if raw.contains('\r') => LineEnding::Cr,
            _ => LineEnding::Lf,
        }
    }
}

fn render<I, T, V>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = T>,
    T: Borrow<V>,
    V: Display + ?Sized,
{
    values
        .into_iter()
        .map(|value| value.borrow().to_string())
        .collect()
}
