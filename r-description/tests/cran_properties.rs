//! Corpus-seeded parser properties over representative CRAN metadata.

use std::{fs, path::Path, sync::OnceLock};

use proptest::prelude::*;
use r_description::Description;
use serde::Deserialize;

const PACKAGES: [&str; 20] = [
    "bsitar",
    "ANTs",
    "SLC",
    "multilm",
    "zicounts",
    "dplyr",
    "PLSbiplot1",
    "ABACUS",
    "AnalyzeIO",
    "emu",
    "VR",
    "HTML",
    "bdc",
    "xlsxjars",
    "stringi",
    "data.table",
    "Rcpp",
    "ggplot2",
    "sf",
    "xml2",
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn representative_descriptions_remain_lossless_and_total(
        case in 0_usize..PACKAGES.len(),
        mutation in 0_u8..7,
        offset in any::<usize>(),
        extra in "[ -~]{0,64}",
    ) {
        let source = &corpus_cases()[case];
        let mutated = mutate(source, mutation, offset, &extra);
        let document = Description::parse(&mutated);

        prop_assert_eq!(document.to_string(), mutated);
        prop_assert_eq!(Description::parse(&document.to_string()), document.clone());
        touch_accessors(&document);
    }
}

fn mutate(source: &str, mutation: u8, offset: usize, extra: &str) -> String {
    match mutation {
        0 => source.to_owned(),
        1 => source
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', "\r\n"),
        2 => format!(" {extra}\n{source}"),
        3 => format!("{source}\nMalformed {extra}"),
        4 => {
            let mut end = offset % (source.len() + 1);
            while !source.is_char_boundary(end) {
                end -= 1;
            }
            source[..end].to_owned()
        }
        5 => format!("{source}\n\nX-Proptest: {extra}\n"),
        6 => format!("{source}{source}"),
        _ => unreachable!(),
    }
}

fn touch_accessors(document: &Description) {
    let _ = document.diagnostics();
    let _ = document.validate();
    let _ = document.fields_all().count();
    let _ = document.package();
    let _ = document.version();
    let _ = document.version_parsed();
    let _ = document.depends_parsed();
    let _ = document.imports_parsed();
    let _ = document.suggests_parsed();
    let _ = document.enhances_parsed();
    let _ = document.linking_to_parsed();
    let _ = document.urls_parsed();
    let _ = document.bug_reports_parsed();
    let _ = document.additional_repositories_parsed();
    let _ = document.remotes_parsed();
}

fn corpus_cases() -> &'static [String] {
    static CASES: OnceLock<Vec<String>> = OnceLock::new();
    CASES.get_or_init(load_cases)
}

fn load_cases() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testdata/cran/latest");
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(root.join("snapshot.json")).unwrap()).unwrap();
    PACKAGES
        .iter()
        .map(|package| {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.package == *package)
                .unwrap_or_else(|| panic!("fixture missing for {package}"));
            String::from_utf8(fs::read(root.join(&entry.path)).unwrap()).unwrap()
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    package: String,
    path: String,
}
