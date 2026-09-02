# r-dcf-syntax

Lossless syntax trees, builders, and text-preserving edits for the R dialect of
Debian Control File (DCF) metadata.

[crates.io](https://crates.io/crates/r-dcf-syntax) |
[API documentation](https://docs.rs/r-dcf-syntax) |
[repository](https://github.com/rrepo-org/r-metadata-rs)

```sh
cargo add r-dcf-syntax
```

```rust
use r_dcf_syntax::parse;

let parsed = parse("Package: demo\nVersion: 1.0.0\n");
assert!(parsed.diagnostics().is_empty());

let record = parsed.records().next().unwrap();
assert_eq!(record.last_field("Package").unwrap().value().as_str(), "demo");
assert_eq!(parsed.to_string(), "Package: demo\nVersion: 1.0.0\n");
```

R DCF is related to deb822 but has different application rules. This crate
uses exact case-sensitive field lookup, preserves duplicate fields, does not
give `#` lines Debian comment semantics, and validates the portable field-name
grammar used by R metadata. Malformed lines remain in Rowan error nodes and
produce positioned diagnostics instead of preventing access to valid fields.

Use this crate for raw DCF syntax. Use
[`r-description-parser`](https://crates.io/crates/r-description-parser) or
[`r-packages-parser`](https://crates.io/crates/r-packages-parser) for typed R
package metadata APIs.

## License

MIT
