use proptest::prelude::*;
use static_assertions::assert_impl_all;

use crate::{
    DiagnosticKind, FieldName, FormatStyle, LogicalValue, Parse, document, field, parse, record,
};

assert_impl_all!(Parse: Send, Sync, Clone);

#[test]
fn parses_records_fields_duplicates_and_case_sensitively() {
    let source = "Name: first\r\nName: second\r\nname: lower\r\n \t\r\nOther:\r";
    let parsed = parse(source);
    assert_eq!(parsed.to_string(), source);
    let records: Vec<_> = parsed.records().collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].fields_named("Name").count(), 2);
    assert_eq!(records[0].fields_named("name").count(), 1);
    assert_eq!(records[1].field("Other").unwrap().value().as_str(), "");
}

#[test]
fn continuation_and_dot_are_unfolded() {
    let parsed = parse("Description: one\n  two\n .\n\tthree: still value\n");
    let value = parsed
        .records()
        .next()
        .unwrap()
        .field("Description")
        .unwrap()
        .value();
    assert_eq!(value.as_str(), "one\ntwo\n\nthree: still value");
    assert_eq!(value.source_range(), crate::SourceSpan::new(12, 45));
}

#[test]
fn malformed_input_is_lossless_and_diagnosed() {
    let source = " orphan\n# not comment\nBad name: x\n: empty\nGood:a:b\n";
    let parsed = parse(source);
    assert_eq!(parsed.to_string(), source);
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .map(crate::Diagnostic::kind)
            .collect::<Vec<_>>(),
        [
            DiagnosticKind::OrphanContinuation,
            DiagnosticKind::MalformedField,
            DiagnosticKind::InvalidFieldName,
            DiagnosticKind::InvalidFieldName,
        ]
    );
    assert_eq!(
        parsed
            .records()
            .next()
            .unwrap()
            .field("Good")
            .unwrap()
            .value()
            .as_str(),
        "a:b"
    );
}

#[test]
fn builders_and_edits_work() {
    let style = FormatStyle::default();
    let name = FieldName::new("Description").unwrap();
    let value = LogicalValue::new("first\n\nthird").unwrap();
    let built = field(&name, &value, &style);
    assert_eq!(built, "Description: first\n .\n third");
    assert_eq!(
        document(&[record(&[built], &style)], &style),
        "Description: first\n .\n third"
    );

    let parsed = parse("X: 1\r\nX:\t2\r\nY: 3\r\n");
    let changed = parsed
        .replace_last(0, "X", &LogicalValue::new("a\nb").unwrap())
        .unwrap();
    assert_eq!(changed.to_string(), "X: 1\r\nX:\ta\r\n b\r\nY: 3\r\n");
    let removed = changed.remove_all(0, "X").unwrap();
    assert_eq!(removed.to_string(), "Y: 3\r\n");

    let inserted = removed
        .insert_after(
            0,
            "Y",
            &FieldName::new("Authors@R").unwrap(),
            &LogicalValue::new("person").unwrap(),
        )
        .unwrap();
    assert_eq!(inserted.to_string(), "Y: 3\r\nAuthors@R: person\r\n");
}

proptest! {
    #[test]
    fn arbitrary_utf8_roundtrips(input in any::<String>()) {
        prop_assert_eq!(parse(&input).to_string(), input);
    }

    #[test]
    fn arbitrary_physical_dcf_roundtrips(
        chunks in prop::collection::vec("[^\\r\\n]*", 0..30),
        endings in prop::collection::vec(prop_oneof![Just("\n"), Just("\r\n"), Just("\r")], 0..30),
    ) {
        let mut source = String::new();
        for (index, chunk) in chunks.iter().enumerate() {
            source.push_str(chunk);
            if let Some(ending) = endings.get(index) {
                source.push_str(ending);
            }
        }
        prop_assert_eq!(parse(&source).to_string(), source);
    }
}
