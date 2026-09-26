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

## Converting individual records

Conversions with `r_description::Description` support both borrowed and owned
values:

```rust
use r_description::Description;
use r_packages::{PackageRecord, Packages};

let packages = Packages::parse("Package: alpha\nVersion: 1.0\nX-Custom: kept\n");
let record = packages.record(0).unwrap();
let description = Description::from(&record);
let round_trip = PackageRecord::try_from(&description).unwrap();
assert_eq!(round_trip.to_string(), record.to_string());
```

Add `r-description-parser` as a dependency to use `Description` directly.
Conversion preserves every field, including custom fields and duplicate
declarations, in source order. Lookups remain case-sensitive and use the last
declaration. Missing metadata is never filled in, and invalid semantic values or
malformed DCF lines are retained without panicking or requiring validation.

Record-local text is preserved exactly: spacing, continuation indentation, line
endings, and final-newline presence. Surrounding blank lines and PACKAGES record
separators belong to the document and are excluded. DESCRIPTION syntax diagnostics
are recomputed relative to the standalone record.

`PackageRecord::try_from` requires exactly one DESCRIPTION record. Empty or
blank-only documents return `RecordConversionError::NoRecords`; multiple records
return `RecordConversionError::MultipleRecords`. A single malformed record is
accepted. These conversions do not generate a filtered repository index.

## Constructing documents from existing records

`Packages::from_records` accepts an iterator of owned or borrowed package records.
Normalization remains an explicit caller choice:

```rust
use r_description::{Description, LogicalValue};
use r_packages::{PackageRecord, Packages, RecordBuilder};

let mut records = Vec::new();
for name in ["alpha", "beta"] {
    let description = Description::builder()
        .package(LogicalValue::new(name).unwrap())
        .version(LogicalValue::new("1.0").unwrap())
        .build()
        .normalize()
        .unwrap();
    records.push(PackageRecord::try_from(description).unwrap());
}
let packages = Packages::from_records(&records);
assert_eq!(packages.len(), 2);

let mixed = Packages::builder()
    .existing_record(&records[0])
    .record(RecordBuilder::new("gamma", "2.0").unwrap())
    .existing_records(&records[1..])
    .build();
assert_eq!(mixed.len(), 3);
```

Assembly preserves record order and all record-local text, including custom and
duplicate fields, multiline values, Unicode, and malformed content. It does not
normalize, sort, deduplicate, filter, or validate existing records. Builder format
settings apply to newly constructed records, not existing text.

Empty input produces empty text. A single existing record is unchanged. Between
records, assembly inserts one blank separator line: if the preceding record has a
trailing newline, that newline convention is reused for the separator; otherwise,
two configured line endings are added (LF by default). The final record's newline
state is preserved. Document-level blank lines surrounding an original record
are not part of the record and are not copied.

Construction appends text into one buffer and parses the completed document once,
with time and memory proportional to total input/output size.

## License

MIT
