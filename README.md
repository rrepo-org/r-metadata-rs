# R metadata for Rust

This workspace provides lossless, failure-tolerant parsing and editing of R
package metadata.

- `r-description` is the public facade for one `DESCRIPTION` record.
- `r-packages` is the public facade for multi-record `PACKAGES` indexes.
- `r-metadata` contains shared versions, dependency relations, URLs, logical
  values, and remote-source specifications.
- `r-dcf-syntax` contains the Rowan 0.17 lossless DCF syntax tree.

Parsing never rejects malformed UTF-8 text. Structural diagnostics, general
metadata validation, and field-local semantic parsing are separate operations,
so malformed fields do not prevent callers from reading unrelated metadata.
Field names are case-sensitive and duplicate lookup follows R's default
last-occurrence behavior.

```rust
use r_description::Description;

let description = Description::parse(
    "Package: example\nVersion: broken\nTitle: Example\nDescription: Demo\nLicense: MIT\nAuthors@R: person('A', 'B')\n",
);

assert_eq!(description.package().unwrap().as_str(), "example");
assert!(description.version_parsed().unwrap().is_err());
assert_eq!(description.to_string().lines().next(), Some("Package: example"));
```

The lossless syntax model currently accepts valid UTF-8 input. Arbitrary
non-UTF-8 DESCRIPTION encodings require a separate byte-preserving source
layer and are outside this version's contract.

## CRAN corpus

`testdata/cran/latest` contains the latest DESCRIPTION available for every
package in rrepo's CRAN mirror. Refresh and verify the snapshot with:

```sh
cargo run -p xtask --release -- cran-snapshot
```

The corpus regression test parses every fixture, checks exact byte round-trip,
and invokes every raw and typed accessor. Semantic errors remain field-local
results because the mirror intentionally includes malformed historical
metadata.
