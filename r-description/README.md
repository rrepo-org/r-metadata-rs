# r-description-parser

Lossless, failure-tolerant parsing, validation, building, and editing of R
package `DESCRIPTION` files.

[crates.io](https://crates.io/crates/r-description-parser) |
[API documentation](https://docs.rs/r-description-parser) |
[repository](https://github.com/rrepo-org/r-metadata-rs)

```sh
cargo add r-description-parser
```

The package name is `r-description-parser`; the Rust library name is
`r_description`.

```rust
use r_description::Description;

let description = Description::parse(
    "Package: example\nVersion: broken\nTitle: Example\nDescription: Demo\nLicense: MIT\nAuthors@R: person('A', 'B')\n",
);

assert_eq!(description.package().unwrap().as_str(), "example");
assert!(description.version_parsed().unwrap().is_err());
assert_eq!(description.to_string().lines().next(), Some("Package: example"));
```

Parsing preserves the original text and never rejects malformed DCF syntax.
Structural diagnostics, document validation, and field-local semantic parsing
are separate, so one malformed field does not prevent access to unrelated
metadata. Duplicate lookup follows R's case-sensitive, last-occurrence
behavior.

See the [workspace overview](https://github.com/rrepo-org/r-metadata-rs) for
the lower-level syntax and semantic-value crates.

## License

MIT
