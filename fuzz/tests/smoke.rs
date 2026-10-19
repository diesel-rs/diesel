use arbitrary::Arbitrary;
use diesel_fuzz::{document, mysql, pg, pg_array, sqlite, sqlite_blob};
use std::num::NonZeroU32;

#[test]
fn every_pg_case_decodes_without_panicking() {
    let oid = NonZeroU32::MIN;
    for selector in 0..pg::CASES.len() {
        let selector = u8::try_from(selector).expect("under 256 cases");
        for bytes in [&[][..], &[0x00], &[0xFF; 4], &[0x7F; 16], &[0xAA; 64]] {
            pg::decode_case(selector, oid, bytes);
        }
    }
}

#[test]
fn arrays_match_the_binary_format_model() {
    use pg_array::{ArrayCase, ArrayInput};

    let int4 = |elements: Vec<Option<i32>>, dimensions: Vec<u8>, lower_bound| {
        ArrayInput::Int4(ArrayCase {
            elements,
            dimensions,
            lower_bound,
        })
    };
    let cases = [
        int4(vec![], vec![], 1),
        int4(vec![Some(7), None, Some(-9)], vec![3], 1),
        int4((1..=6).map(Some).collect(), vec![2, 3], -4),
        int4(vec![Some(1)], vec![2, 2], 0),
        ArrayInput::Text(ArrayCase {
            elements: vec![Some(String::new()), None, Some("ünïcode".into())],
            dimensions: vec![1, 3, 0],
            lower_bound: i32::MAX,
        }),
    ];
    for input in &cases {
        pg_array::run_case(input).expect("arrays match the model");
    }
}

#[test]
fn every_mysql_case_decodes_without_panicking() {
    for selector in 0..mysql::CASES.len() {
        let selector = u8::try_from(selector).expect("under 256 cases");
        for tpe in 0..mysql::TYPES.len() {
            let tpe = u8::try_from(tpe).expect("under 256 types");
            for bytes in [&[][..], &[0x00], &[0xFF; 4], &[0x30; 12]] {
                mysql::decode_case(selector, tpe, bytes);
            }
        }
    }
}

#[test]
fn every_sqlite_case_decodes_without_panicking() {
    for selector in 0..sqlite::CASES.len() {
        let selector = u8::try_from(selector).expect("under 256 cases");
        for kind in 0..4 {
            for bytes in [
                &[][..],
                &[0x00],
                &[0xFF; 8],
                b"2024-06-15 10:30:45",
                b"1.5e3",
            ] {
                sqlite::decode_case(selector, kind, bytes);
            }
        }
    }
}

#[test]
fn blob_operations_match_byte_slice_model() {
    use sqlite_blob::SqliteBlobOp::{Read, SeekCurrent, SeekEnd, SeekStart};

    let mut input = sqlite_blob::BlobInput {
        data: b"abc",
        operations: vec![
            SeekStart(1),
            Read(1),         // "b"
            SeekCurrent(-5), // rejected before the start
            Read(1),         // "c"
            SeekEnd(-1),
            SeekEnd(-4), // rejected before the start
            Read(64),    // "c"
            SeekStart(0),
            Read(3), // "abc"
        ],
        close_explicitly: true,
    };

    for close_explicitly in [true, false, true] {
        input.close_explicitly = close_explicitly;
        sqlite_blob::run_case(&input).expect("blob operations match the model");
    }
}

#[test]
fn documents_survive_the_round_trip() {
    sqlite::with_conn(|conn| {
        for document in [
            "null",
            "true",
            "false",
            "0",
            "-1",
            "1.5",
            "1e-7",
            "\"\"",
            "\"a\"",
            "[]",
            "{}",
            "[null,true,1,\"a\",[],{}]",
            "{\"a\":{\"b\":[1,2,3]}}",
        ] {
            let value: serde_json::Value = serde_json::from_str(document).expect("valid json");
            sqlite::roundtrip_jsonb(conn, &value).expect(document);
            sqlite::roundtrip_json(conn, &value).expect(document);
        }
    });
}

#[test]
fn generated_documents_stay_within_the_read_limit() {
    for bytes in [
        vec![0xC5; 976],
        vec![0x06; 976],
        vec![0xFF; 976],
        vec![0x00; 976],
        (0u8..=255).cycle().take(4096).collect(),
    ] {
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let document =
            document::Document::arbitrary(&mut unstructured).expect("documents from bytes");
        assert!(
            document.nesting() <= document::MAX_NESTING,
            "the generator exceeded the depth serde_json reads"
        );
    }
}

#[test]
fn a_document_at_the_read_limit_survives_the_round_trip() {
    let mut value = serde_json::Value::Null;
    for _ in 0..document::MAX_NESTING {
        value = serde_json::Value::Array(vec![value]);
    }
    sqlite::with_conn(|conn| {
        sqlite::roundtrip_jsonb(conn, &value).expect("jsonb at the nesting cap");
        sqlite::roundtrip_json(conn, &value).expect("json text at the nesting cap");
    });
}

#[test]
fn a_decoded_blob_is_the_one_sqlite_calls_valid() {
    sqlite::with_conn(|conn| {
        assert_eq!(sqlite::jsonb_valid(conn, &[0x00]), Ok(true));
        assert_eq!(sqlite::jsonb_valid(conn, &[0xFF]), Ok(false));
        assert!(sqlite::decode_jsonb(conn, &[0xFF]).is_err());
    });
}
