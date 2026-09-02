//! Shared DESCRIPTION field classifications.

/// Semantic treatment used when normalizing duplicate declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Scalar,
    Relations,
    Urls,
    Repositories,
    Remotes,
    Ordered,
}

const FIRST_FIELDS: &[&str] = &[
    "Type",
    "Package",
    "Title",
    "Version",
    "Date",
    "Authors@R",
    "Author",
    "Maintainer",
    "Description",
    "License",
    "URL",
    "BugReports",
    "Depends",
    "Imports",
    "Suggests",
    "Enhances",
    "LinkingTo",
    "VignetteBuilder",
    "RdMacros",
    "Remotes",
];

const KNOWN_SCALAR_FIELDS: &[&str] = &[
    "Type",
    "Package",
    "Title",
    "Version",
    "Date",
    "Date/Publication",
    "Authors@R",
    "Author",
    "Maintainer",
    "Copyright",
    "Description",
    "License",
    "BugReports",
    "BuildKeepEmpty",
    "BuildManual",
    "BuildResaveData",
    "SystemRequirements",
    "Encoding",
    "Repository",
    "Contact",
    "MailingList",
    "Note",
    "NeedsCompilation",
    "OS_type",
    "Priority",
    "Archs",
    "Biarch",
    "Classification/ACM",
    "Classification/ACM-2012",
    "Classification/JEL",
    "Classification/MSC",
    "Classification/MSC-2010",
    "Language",
    "LazyData",
    "LazyDataCompression",
    "LazyLoad",
    "ByteCompile",
    "KeepSource",
    "UseLTO",
    "StagedInstall",
    "ZipData",
    "BuildVignettes",
    "License_is_FOSS",
    "License_restricts_use",
    "RoxygenNote",
    "Roxygen",
    "RcmdrModels",
    "RcppModules",
    "SysDataCompression",
    "Path",
    "LastChangedDate",
    "LastChangedRevision",
    "Revision",
    "Acknowledgements",
    "Acknowledgments",
    "biocViews",
    "MD5sum",
    "Packaged",
    "Built",
];

const REQUIRED_OR_CONDITIONAL_FIELDS: &[&str] = &[
    "Package",
    "Version",
    "Title",
    "Description",
    "License",
    "Authors@R",
    "Author",
    "Maintainer",
];

pub(crate) fn field_kind(name: &str) -> Option<FieldKind> {
    match name {
        "Depends" | "Imports" | "Suggests" | "Enhances" | "LinkingTo" | "VignetteBuilder"
        | "RdMacros" => Some(FieldKind::Relations),
        "URL" => Some(FieldKind::Urls),
        "Additional_repositories" => Some(FieldKind::Repositories),
        "Remotes" => Some(FieldKind::Remotes),
        "Collate" | "Collate.unix" | "Collate.windows" => Some(FieldKind::Ordered),
        name if KNOWN_SCALAR_FIELDS.contains(&name) => Some(FieldKind::Scalar),
        _ => None,
    }
}

pub(crate) fn permits_duplicate_declarations(name: &str) -> bool {
    matches!(
        field_kind(name),
        Some(FieldKind::Relations | FieldKind::Urls | FieldKind::Repositories | FieldKind::Remotes)
    )
}

pub(crate) fn remove_when_empty(name: &str) -> bool {
    field_kind(name).is_some() && !REQUIRED_OR_CONDITIONAL_FIELDS.contains(&name)
}

pub(crate) fn order(name: &str) -> (u8, usize, &str) {
    if let Some(index) = FIRST_FIELDS.iter().position(|field| *field == name) {
        (0, index, name)
    } else if matches!(name, "Collate" | "Collate.unix" | "Collate.windows") {
        (2, 0, name)
    } else {
        (1, 0, name)
    }
}

pub(crate) fn is_prose(name: &str) -> bool {
    matches!(
        name,
        "Title" | "Description" | "SystemRequirements" | "Note"
    )
}
