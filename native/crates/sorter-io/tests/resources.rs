use sha2::{Digest, Sha256};
use sorter_io::resources::{AliasSource, ResourceError, load_aliases};

#[test]
fn bundled_first_lookup_keeps_duplicates_and_whitespace() {
    let bundle = load_aliases(
        "דוד,first\nדוד,second\nדוד , spaced \n".as_bytes(),
        Some("דוד,personal\n".as_bytes()),
    )
    .unwrap();
    assert_eq!(bundle.ordered_rows.len(), 4);
    assert_eq!(bundle.alias_at(2).unwrap().source, "דוד ");
    assert_eq!(bundle.alias_at(2).unwrap().target, " spaced ");
    let found = bundle.find_alias("דוד").unwrap().unwrap();
    assert_eq!(found.index, 0);
    assert_eq!(found.alias.target, "first");
    assert_eq!(bundle.alias_at(3).unwrap().target, "personal");
}

#[test]
fn python_csv_quotes_newlines_crlf_and_no_header_skip() {
    let bundle = load_aliases(
        b"source,target\r\n\"a,\"\"b\"\"\r\nc\",\"x\rY\"\r\n a, b\r\n",
        None,
    )
    .unwrap();
    assert_eq!(bundle.ordered_rows[0].fields, ["source", "target"]);
    assert_eq!(bundle.ordered_rows[1].fields, ["a,\"b\"\nc", "x\nY"]);
    assert_eq!(bundle.ordered_rows[2].fields, [" a", " b"]);
    assert_eq!(
        bundle
            .ordered_rows
            .iter()
            .map(|row| row.record)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn bom_and_empty_fields_are_retained() {
    let bundle = load_aliases("\u{feff}דוד,יעד\n,\n\"\",\"\"\n".as_bytes(), None).unwrap();
    assert_eq!(bundle.alias_at(0).unwrap().source, "\u{feff}דוד");
    let bom_only = load_aliases("\u{feff}דוד,יעד\n".as_bytes(), None).unwrap();
    assert!(bom_only.find_alias("דוד").unwrap().is_none());
    // The release matcher accepts an empty source at a word boundary.
    assert_eq!(bundle.find_alias("דוד").unwrap().unwrap().index, 1);
    assert_eq!(bundle.ordered_rows[1].fields, ["", ""]);
    assert_eq!(bundle.ordered_rows[2].fields, ["", ""]);
}

#[test]
fn malformed_width_is_evidence_not_eager_loss_of_rows() {
    let bundle = load_aliases(b"a,A\n\none\nx,y,z\n", None).unwrap();
    assert_eq!(
        bundle
            .ordered_rows
            .iter()
            .map(|r| r.fields.len())
            .collect::<Vec<_>>(),
        [2, 0, 1, 3]
    );
    assert_eq!(bundle.diagnostics.len(), 3);
    for (index, width) in [(1, 0), (2, 1), (3, 3)] {
        assert_eq!(
            bundle.alias_at(index),
            Err(ResourceError::MalformedRow {
                source: AliasSource::Bundled,
                index,
                record: index as u64 + 1,
                width,
            })
        );
        let diagnostic = &bundle.diagnostics[index - 1];
        assert_eq!((diagnostic.index, diagnostic.width), (index, width));
        assert_eq!(diagnostic.code, "ALIAS_ROW_WIDTH");
        assert!(diagnostic.blocks_apply);
    }
}

#[test]
fn valid_early_decision_survives_later_malformed_personal_row() {
    let bundle = load_aliases(b"a,A\n", Some(b"broken\n")).unwrap();
    let found = bundle.find_alias("a song").unwrap().unwrap();
    assert_eq!((found.index, found.alias.target.as_str()), (0, "A"));
    assert!(bundle.has_apply_blockers());
    assert_eq!(
        bundle.find_alias("unknown"),
        Err(ResourceError::MalformedRow {
            source: AliasSource::Personal,
            index: 1,
            record: 1,
            width: 1,
        })
    );
}

#[test]
fn invalid_utf8_is_explicit_and_never_an_empty_alias_list() {
    assert_eq!(
        load_aliases(b"a,A\n\xff", None),
        Err(ResourceError::InvalidUtf8 {
            source: AliasSource::Bundled,
            valid_up_to: 4,
            error_len: Some(1),
        })
    );
    assert_eq!(
        load_aliases(b"a,A\n", Some(b"\xc3")),
        Err(ResourceError::InvalidUtf8 {
            source: AliasSource::Personal,
            valid_up_to: 0,
            error_len: None,
        })
    );
}

#[test]
fn permissive_excel_dialect_matches_python_not_strict_csv() {
    let bundle = load_aliases(b"\"a\"x,b\na\"b,c\n\"unterminated", None).unwrap();
    assert_eq!(bundle.ordered_rows[0].fields, ["ax", "b"]);
    assert_eq!(bundle.ordered_rows[1].fields, ["a\"b", "c"]);
    assert_eq!(bundle.ordered_rows[2].fields, ["unterminated"]);
}

#[test]
fn empty_input_blank_records_trailing_delimiters_and_nul_are_distinct() {
    assert!(load_aliases(b"", None).unwrap().ordered_rows.is_empty());
    let bundle = load_aliases(b"\n\r\n,\n\"\"\na,\na\0b,c\n", None).unwrap();
    assert_eq!(
        bundle
            .ordered_rows
            .iter()
            .map(|r| r.fields.clone())
            .collect::<Vec<_>>(),
        vec![
            Vec::<String>::new(),
            vec![],
            vec!["".into(), "".into()],
            vec!["".into()],
            vec!["a".into(), "".into()],
            vec!["a\0b".into(), "c".into()]
        ]
    );
}

#[test]
fn field_limit_counts_unicode_code_points_like_python() {
    let allowed = format!("{},ok", "ד".repeat(131_072));
    assert!(load_aliases(allowed.as_bytes(), None).is_ok());
    let oversized = format!("{},ok", "ד".repeat(131_073));
    assert_eq!(
        load_aliases(oversized.as_bytes(), None),
        Err(ResourceError::FieldTooLarge {
            source: AliasSource::Bundled,
            record: 1,
            limit: 131_072,
        })
    );
}

#[test]
fn release_csv_is_exact_original_1196_rows_and_hash() {
    let csv = include_bytes!("../../../tests/fixtures/full_contract/released-singer-list.csv");
    let expected = "7d2b0c806a18043c16b7baa99ebd174955bad1f8b70f606ee8f56e9559d4d391";
    assert_eq!(format!("{:x}", Sha256::digest(csv)), expected);
    let bundle = load_aliases(csv, None).unwrap();
    assert_eq!(bundle.ordered_rows.len(), 1196);
    assert!(bundle.diagnostics.is_empty());
    assert_eq!(bundle.source_hashes[0].sha256, expected);
    assert_eq!(bundle.source_hashes[0].records, 1196);
    assert_eq!(bundle.source_hashes[0].bytes, 49_284);
    let frozen: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/ordered-aliases.json")).unwrap();
    let aliases = frozen["aliases"].as_array().unwrap();
    for (index, row) in aliases.iter().enumerate() {
        let alias = bundle.alias_at(index).unwrap();
        assert_eq!(alias.source, row[0].as_str().unwrap());
        assert_eq!(alias.target, row[1].as_str().unwrap());
    }
}

#[test]
fn index_error_is_typed_and_sources_keep_independent_record_numbers() {
    let bundle = load_aliases(b"a,A\n", Some(b"b,B\n\n")).unwrap();
    assert_eq!(bundle.ordered_rows[1].record, 1);
    assert_eq!(bundle.ordered_rows[2].record, 2);
    assert_eq!(
        bundle.alias_at(3),
        Err(ResourceError::IndexOutOfBounds { index: 3, rows: 3 })
    );
    assert_eq!(bundle.source_hashes.len(), 2);
    assert_eq!(bundle.source_hashes[1].source, AliasSource::Personal);
}

#[test]
fn exhaustive_frozen_python_dialect_corpus_is_independent_of_rust_parser() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/full_contract/csv-dialect.json"
    ))
    .unwrap();
    assert_eq!(corpus["cases"].as_array().unwrap().len(), 1561);
    for case in corpus["cases"].as_array().unwrap() {
        let bundle = load_aliases(case["text"].as_str().unwrap().as_bytes(), None).unwrap();
        let actual = serde_json::to_value(
            bundle
                .ordered_rows
                .iter()
                .map(|row| &row.fields)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(actual, case["rows"], "Python reference case {}", case["id"]);
    }
}
