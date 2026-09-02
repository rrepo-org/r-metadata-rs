# r-metadata

Typed semantic values shared by R package metadata formats.

[crates.io](https://crates.io/crates/r-metadata) |
[API documentation](https://docs.rs/r-metadata) |
[repository](https://github.com/rrepo-org/r-metadata-rs)

```sh
cargo add r-metadata
```

```rust
use r_metadata::Version;

let version: Version = "1.2-0".parse().unwrap();
assert_eq!(version.components(), &[1, 2, 0]);
assert_eq!(version.as_str(), "1.2-0");
```

The crate parses and represents R package versions, dependency relations,
version requirements, URLs, logical values, operating-system and priority
values, and CRAN, Bioconductor, Git, SVN, URL, and local remote sources.
Collection parsers retain valid entries while reporting positioned errors for
malformed entries.

This crate does not parse DCF documents. Use
[`r-description-parser`](https://crates.io/crates/r-description-parser) or
[`r-packages-parser`](https://crates.io/crates/r-packages-parser) for complete
metadata files.

## License

MIT
