# r-packages-parser

Lossless, failure-tolerant parsing, validation, building, and editing of R
repository `PACKAGES` indexes.

[crates.io](https://crates.io/crates/r-packages-parser) |
[API documentation](https://docs.rs/r-packages-parser) |
[repository](https://github.com/rrepo-org/r-metadata-rs)

```sh
cargo add r-packages-parser
```

The package name is `r-packages-parser`; the Rust library name is `r_packages`.

```rust
use r_packages::Packages;

let packages = Packages::parse(
    "Package: alpha\nVersion: 1.0.0\n\nPackage: beta\nVersion: 2.0.0\n",
);

assert_eq!(packages.len(), 2);
assert_eq!(packages.record(1).unwrap().package().unwrap().as_str(), "beta");
assert_eq!(packages.to_string().lines().next(), Some("Package: alpha"));
```

Every record remains accessible even when its DCF structure or typed fields
are malformed. The API provides record-scoped immutable edits, canonical
builders, typed field parsing, and index validation while preserving untouched
source text exactly.

See the [workspace overview](https://github.com/rrepo-org/r-metadata-rs) for
the lower-level syntax and semantic-value crates.

## License

MIT
