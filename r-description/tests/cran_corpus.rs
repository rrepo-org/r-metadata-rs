//! Strict regression audit over the vendored latest CRAN DESCRIPTION snapshot.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
};

use r_description::{CollectionResult, Description};
use serde::Deserialize;

const MAX_EXAMPLES: usize = 8;

#[test]
fn every_latest_cran_description_is_valid_and_accessor_safe() {
    let root = corpus_root();
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(root.join("snapshot.json")).unwrap()).unwrap();
    assert_eq!(manifest.fixture_count, manifest.entries.len());

    let mut failures = Failures::default();
    for entry in &manifest.entries {
        let identity = format!("{} {}", entry.package, entry.version);
        let bytes = match fs::read(root.join(&entry.path)) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.record("fixture.read", &identity, error);
                continue;
            }
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            audit_description(&bytes, entry, &identity, &mut failures);
        }));
        if result.is_err() {
            failures.record("panic", &identity, "parser or accessor panicked");
        }
    }

    assert!(failures.is_empty(), "{}", failures.report());
}

fn audit_description(bytes: &[u8], entry: &ManifestEntry, identity: &str, failures: &mut Failures) {
    let document = match Description::parse_utf8(bytes) {
        Ok(document) => document,
        Err(error) => {
            failures.record("encoding.invalid-utf8", identity, error);
            return;
        }
    };

    if document.to_string().as_bytes() != bytes {
        failures.record("roundtrip.mismatch", identity, "rendered bytes differ");
    }
    let _ = document.diagnostics();
    let _ = document.validate();
    let _ = document.package();
    let _ = document.version();
    let _ = (&entry.package, &entry.version);

    touch_raw_accessors(&document);
    touch_syntax_accessors(&document);
    touch_typed_accessors(&document);
}

#[allow(clippy::too_many_lines)]
fn touch_raw_accessors(document: &Description) {
    macro_rules! touch {
        ($($accessor:ident),+ $(,)?) => {
            $(let _ = document.$accessor();)+
        };
    }
    touch! {
        package, type_, package_type, title, version, date, date_publication,
        authors_at_r, author, maintainer, copyright, description, license,
        depends, imports, suggests, enhances, linking_to, system_requirements,
        url, bug_reports, additional_repositories, remotes, encoding,
        repository, contact, mailing_list, note, needs_compilation, os_type,
        priority, archs, biarch, classification_acm, classification_acm_2012,
        classification_jel, classification_msc, classification_msc_2010,
        collate, lazy_data, lazy_load, byte_compile, keep_source, use_lto,
        staged_install, zip_data, build_vignettes, license_is_foss,
        license_restricts_use, vignette_builder, roxygen_note, rd_macros,
        packaged, built,
    }
}

fn touch_syntax_accessors(document: &Description) {
    let _ = document.as_parse().green();
    let _ = document.as_parse().root().source_range();
    for record in document.records() {
        let _ = record.raw_text();
        let _ = record.source_range();
        for field in record.fields() {
            if let Some(name) = field.name() {
                let _ = document.field(&name);
                let _ = document.fields(&name).count();
            }
            let _ = field.raw_text();
            let _ = field.value();
            let _ = field.source_range();
        }
    }
    let _ = document.fields_all().count();
}

fn touch_typed_accessors(document: &Description) {
    let _ = document.version_parsed();

    touch_collection(&document.depends_parsed());
    touch_collection(&document.imports_parsed());
    touch_collection(&document.suggests_parsed());
    touch_collection(&document.enhances_parsed());
    touch_collection(&document.linking_to_parsed());
    touch_collection(&document.vignette_builder_parsed());
    touch_collection(&document.urls_parsed());
    touch_collection(&document.bug_reports_parsed());
    touch_collection(&document.additional_repositories_parsed());
    touch_collection(&document.remotes_parsed());

    macro_rules! scalar {
        ($value:expr) => {
            let _ = $value;
        };
    }
    scalar!(document.needs_compilation_parsed());
    scalar!(document.biarch_parsed());
    scalar!(document.lazy_data_parsed());
    scalar!(document.lazy_load_parsed());
    scalar!(document.byte_compile_parsed());
    scalar!(document.keep_source_parsed());
    scalar!(document.use_lto_parsed());
    scalar!(document.staged_install_parsed());
    scalar!(document.zip_data_parsed());
    scalar!(document.build_vignettes_parsed());
    scalar!(document.license_is_foss_parsed());
    scalar!(document.license_restricts_use_parsed());
    scalar!(document.os_type_parsed());
    scalar!(document.priority_parsed());
}

fn touch_collection<T, E>(result: &CollectionResult<T, E>) {
    for issue in result.issues() {
        let _ = (&issue.error, issue.field_span, issue.value_span);
    }
    let _ = result.entries();
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/cran/latest")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    fixture_count: usize,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    package: String,
    version: String,
    path: String,
}

#[derive(Debug, Default)]
struct Failures {
    groups: BTreeMap<String, FailureGroup>,
}

impl Failures {
    fn record(&mut self, category: &str, identity: &str, detail: impl std::fmt::Display) {
        let group = self.groups.entry(category.to_owned()).or_default();
        group.count += 1;
        if group.examples.len() < MAX_EXAMPLES {
            group.examples.push(format!("{identity}: {detail}"));
        }
    }

    fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    fn report(&self) -> String {
        let mut output = format!("{} failure categories\n", self.groups.len());
        for (category, group) in &self.groups {
            writeln!(output, "\n{category}: {}", group.count)
                .expect("writing to a String cannot fail");
            for example in &group.examples {
                output.push_str("  ");
                output.push_str(example);
                output.push('\n');
            }
        }
        output
    }
}

#[derive(Debug, Default)]
struct FailureGroup {
    count: usize,
    examples: Vec<String>,
}
