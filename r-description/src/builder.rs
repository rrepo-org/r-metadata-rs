use crate::{Description, FieldName, FormatStyle, LogicalValue};

/// Canonical single-record `DESCRIPTION` builder.
#[derive(Debug, Clone, Default)]
pub struct DescriptionBuilder {
    style: FormatStyle,
    fields: Vec<(FieldName, LogicalValue)>,
}

impl DescriptionBuilder {
    /// Creates a builder using `style`.
    pub const fn new(style: FormatStyle) -> Self {
        Self {
            style,
            fields: Vec::new(),
        }
    }

    /// Appends a validated field. Duplicate declarations are retained.
    pub fn field(mut self, name: FieldName, value: LogicalValue) -> Self {
        self.fields.push((name, value));
        self
    }

    /// Appends `Package`.
    pub fn package(self, value: LogicalValue) -> Self {
        self.common("Package", value)
    }
    /// Appends `Version`.
    pub fn version(self, value: LogicalValue) -> Self {
        self.common("Version", value)
    }
    /// Appends `Title`.
    pub fn title(self, value: LogicalValue) -> Self {
        self.common("Title", value)
    }
    /// Appends `Description`.
    pub fn description(self, value: LogicalValue) -> Self {
        self.common("Description", value)
    }
    /// Appends `License`.
    pub fn license(self, value: LogicalValue) -> Self {
        self.common("License", value)
    }
    /// Appends `Authors@R`.
    pub fn authors_at_r(self, value: LogicalValue) -> Self {
        self.common("Authors@R", value)
    }
    /// Appends `Author`.
    pub fn author(self, value: LogicalValue) -> Self {
        self.common("Author", value)
    }
    /// Appends `Maintainer`.
    pub fn maintainer(self, value: LogicalValue) -> Self {
        self.common("Maintainer", value)
    }
    /// Appends `Depends`.
    pub fn depends(self, value: LogicalValue) -> Self {
        self.common("Depends", value)
    }
    /// Appends `Imports`.
    pub fn imports(self, value: LogicalValue) -> Self {
        self.common("Imports", value)
    }
    /// Appends `Suggests`.
    pub fn suggests(self, value: LogicalValue) -> Self {
        self.common("Suggests", value)
    }
    /// Appends `URL`.
    pub fn url(self, value: LogicalValue) -> Self {
        self.common("URL", value)
    }
    /// Appends `Remotes`.
    pub fn remotes(self, value: LogicalValue) -> Self {
        self.common("Remotes", value)
    }

    fn common(self, name: &str, value: LogicalValue) -> Self {
        self.field(
            FieldName::new(name).expect("constant field name is valid"),
            value,
        )
    }

    /// Builds a lossless parse with no structural syntax diagnostics.
    pub fn build(self) -> Description {
        let fields = self
            .fields
            .iter()
            .map(|(name, value)| r_dcf_syntax::make::field(name, value, &self.style))
            .collect::<Vec<_>>();
        let record = r_dcf_syntax::make::record(&fields, &self.style);
        Description::parse(&record)
    }
}
