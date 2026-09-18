use arbitrary::Arbitrary;
use diesel_fuzz::{document, mysql, pg, sqlite, sqlite_blob};
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
    let operations = [
        1, 1, 0, // Seek from start to 1
        0, 1, 0, // Read "b"
        3, 0xFB, 0xFF, // Reject a seek before start from current
        0, 1, 0, // Still read "c"
        2, 0xFF, 0xFF, // Seek one byte before end
        2, 0xFC, 0xFF, // Reject a seek before start from end
        0, 64, 0, // Still read "c"
        1, 0, 0, // Seek to start
        0, 3, 0, // Read "abc"
    ];

    for close_explicitly in [true, false, true] {
        let input = sqlite_blob::BlobInput {
            data: b"abc",
            operations: &operations,
            close_explicitly,
        };
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
