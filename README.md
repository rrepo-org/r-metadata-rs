# R metadata for Rust

This workspace provides lossless, failure-tolerant parsing and editing of R
package metadata. Parsing, structural diagnostics, document validation, and
field-local semantic errors are separate, so malformed fields do not prevent
access to unrelated metadata.

## Crates

| Crate | Use it for | Documentation |
| --- | --- | --- |
| [`r-description-parser`](https://crates.io/crates/r-description-parser) | One package `DESCRIPTION` file | [docs.rs](https://docs.rs/r-description-parser) |
| [`r-packages-parser`](https://crates.io/crates/r-packages-parser) | Multi-record repository `PACKAGES` indexes | [docs.rs](https://docs.rs/r-packages-parser) |
| [`r-metadata`](https://crates.io/crates/r-metadata) | Shared versions, dependency relations, URLs, logical values, and remote sources | [docs.rs](https://docs.rs/r-metadata) |
| [`r-dcf-syntax`](https://crates.io/crates/r-dcf-syntax) | Raw lossless Rowan syntax trees and text-preserving edits | [docs.rs](https://docs.rs/r-dcf-syntax) |

Most applications should start with one of the two parser facade crates:

```sh
cargo add r-description-parser
# or
cargo add r-packages-parser
```

The package names carry a `-parser` suffix to distinguish them on crates.io;
their Rust library names remain `r_description` and `r_packages`.

## DESCRIPTION

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

## Architecture

`r-dcf-syntax` handles only the physical R DCF representation: records,
fields, continuations, diagnostics, formatting, and lossless edits.
`r-metadata` handles semantic values independently of any document. The two
facade crates combine those layers for the field sets and validation rules of
`DESCRIPTION` and `PACKAGES` files.

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

## License

MIT
